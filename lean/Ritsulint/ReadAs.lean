/-!
# What a read-as clause does to a provision

A read-as clause `第七条第一項中「A」とあるのは「B」と読み替える` says: where the provision named has
the words `A`, read `B`. This module states that as a function on the text of the provision and
proves when the function changes nothing. A text is a list of characters of any type with
decidable equality; `Ritsulint.Findings` uses the code points of the characters.

- `occurs q t`: the words `q` stand somewhere in the text `t`.
- `Clause.apply c t`: the text `t` read under the clause `c`, with every `c.quoted` from the left
  replaced by `c.replacement`.
- `Clause.Vacuous c t`: the clause leaves the text as it is.
- `vacuous_of_absent`: a clause whose quoted words do not occur in the text is vacuous.
- `apply_of_prefix`: a clause whose quoted words open the text puts the replacement there.
-/

namespace Ritsulint.ReadAs

variable {α : Type} [DecidableEq α]

/-- The rest of the text when it starts with the words. -/
def dropWords : List α → List α → Option (List α)
  | [], t => some t
  | _ :: _, [] => none
  | p :: ps, c :: cs => if p = c then dropWords ps cs else none

/-- The words `q` stand somewhere in the text. -/
def occurs (q : List α) : List α → Bool
  | [] => (dropWords q []).isSome
  | c :: cs => (dropWords q (c :: cs)).isSome || occurs q cs

/-- Replaces `q` by `r` from the left. `fuel` bounds the steps; the length of the text is
enough, since every step consumes a character of it or stops. -/
def applyAux (q r : List α) : Nat → List α → List α
  | 0, t => t
  | _ + 1, [] => []
  | fuel + 1, c :: cs =>
    match dropWords q (c :: cs) with
    | some rest => r ++ applyAux q r fuel rest
    | none => c :: applyAux q r fuel cs

/-- A read-as clause: read `quoted` as `replacement`. -/
structure Clause (α : Type) where
  quoted : List α
  replacement : List α

/-- The text of a provision read under the clause. -/
def Clause.apply (c : Clause α) (text : List α) : List α :=
  applyAux c.quoted c.replacement text.length text

/-- The clause leaves the text of the provision as it is. -/
def Clause.Vacuous (c : Clause α) (text : List α) : Prop :=
  c.apply text = text

instance (c : Clause α) (text : List α) : Decidable (c.Vacuous text) :=
  inferInstanceAs (Decidable (c.apply text = text))

theorem applyAux_of_not_occurs (q r : List α) :
    ∀ (fuel : Nat) (t : List α), occurs q t = false → applyAux q r fuel t = t := by
  intro fuel
  induction fuel with
  | zero => intro t _; rfl
  | succ fuel ih =>
    intro t h
    cases t with
    | nil => rfl
    | cons c cs =>
      simp only [occurs, Bool.or_eq_false_iff] at h
      have hnone : dropWords q (c :: cs) = none := by
        cases hp : dropWords q (c :: cs) with
        | none => rfl
        | some rest => simp [hp] at h
      simp only [applyAux, hnone]
      rw [ih cs h.2]

/-- A read-as clause whose quoted words do not occur in the provision changes nothing. -/
theorem vacuous_of_absent (c : Clause α) (text : List α)
    (h : occurs c.quoted text = false) : c.Vacuous text :=
  applyAux_of_not_occurs c.quoted c.replacement text.length text h

/-- A read-as clause whose quoted words open the provision puts the replacement there. -/
theorem apply_of_prefix (c : Clause α) (ch : α) (cs rest : List α)
    (h : dropWords c.quoted (ch :: cs) = some rest) :
    c.apply (ch :: cs) = c.replacement ++ applyAux c.quoted c.replacement cs.length rest := by
  simp [Clause.apply, applyAux, h]

end Ritsulint.ReadAs
