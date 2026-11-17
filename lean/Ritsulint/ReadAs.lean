/-!
# What read-as clauses do to a provision

A read-as clause `第七条第一項中「A」とあるのは「B」と読み替える` says: where the provision named has
the words `A`, read `B`. A paragraph usually carries several clauses for one provision. This
module states the reading as a function on the text of the provision and proves when the plain
reading, one clause after another, gives the same text.

A text is a list of characters of any type with decidable equality; `Ritsulint.Api` uses `Char`
and `Ritsulint.Findings` the code points.

The reading chosen here (`applyAll`) goes through the provision once from the left. At each place
it takes the clause whose quoted words stand there, the longest when several do, writes its
replacement and goes on after the quoted words. What a clause wrote is not read again. The
statutes need this: 公職選挙法第三十四条の二第四項 reads `長の任期満了による選挙` as
`議会の議員の任期満了による一般選挙` and the latter as the former in one sentence, which exchanges the
two only when both clauses are read at once. No official source for "from the left" and "the
longest" was found, so provisions where the choice matters are reported (`rightToLeft`).

- `applyAll cs t`: the text `t` read under all the clauses `cs` at once.
- `applySeq cs t`: the text after replacing clause by clause, in the order written.
- `independent cs t`: no clause meets the places of the earlier clauses or finds its quoted words
  in what they wrote. Decided on the reading at once only.
- `applySeq_eq_applyAll`: for independent clauses the two readings agree.
- `swap_differs`: two clauses that exchange two words read differently one after another.
- `vacuous_of_absent`: a clause whose quoted words do not occur in the text changes nothing.
-/

namespace Ritsulint.ReadAs

variable {α : Type} [DecidableEq α]

/-- A read-as clause: read `quoted` as `replacement`. -/
structure Clause (α : Type) where
  quoted : List α
  replacement : List α
deriving DecidableEq, Repr

/-- The words `q` are not empty and open the text. -/
def starts (q u : List α) : Bool :=
  !q.isEmpty && q.isPrefixOf u

/-- The words `q` stand somewhere in the text. -/
def occurs (q : List α) : List α → Bool
  | [] => q.isPrefixOf []
  | c :: cs => q.isPrefixOf (c :: cs) || occurs q cs

/-- The clause whose quoted words open the text: the longest, and of equally long ones the one
written first. -/
def pick : List (Clause α) → List α → Option (Clause α)
  | [], _ => none
  | c :: cs, u =>
    match pick cs u with
    | some d => if starts c.quoted u && d.quoted.length ≤ c.quoted.length then some c else some d
    | none => if starts c.quoted u then some c else none

/-- A piece of a provision as the clauses meet it: a character no clause takes, or the place of a
clause. -/
inductive Seg (α : Type) where
  | lit (a : α)
  | hit (c : Clause α)
deriving DecidableEq, Repr

/-- The provision cut into pieces from the left. The number counts the characters still covered
by the place found last. -/
def segs (cs : List (Clause α)) : Nat → List α → List (Seg α)
  | _, [] => []
  | n + 1, _ :: t => segs cs n t
  | 0, a :: t =>
    match pick cs (a :: t) with
    | some c => .hit c :: segs cs (c.quoted.length - 1) t
    | none => .lit a :: segs cs 0 t

/-- A piece as read: the replacement at the place of a clause. -/
def Seg.read : Seg α → List α
  | .lit a => [a]
  | .hit c => c.replacement

/-- The text of the provision read under all the clauses at once. -/
def applyAll (cs : List (Clause α)) (t : List α) : List α :=
  (segs cs 0 t).flatMap Seg.read

/-- The text of the provision read under one clause. -/
def Clause.apply (c : Clause α) (t : List α) : List α :=
  applyAll [c] t

/-- The text after replacing clause by clause, in the order written. -/
def applySeq (cs : List (Clause α)) (t : List α) : List α :=
  cs.foldl (fun u c => c.apply u) t

/-- The provision read from the right: the same reading on the mirrored text. -/
def rightToLeft (cs : List (Clause α)) (t : List α) : List α :=
  (applyAll (cs.map fun c => ⟨c.quoted.reverse, c.replacement.reverse⟩) t.reverse).reverse

/-- The clause leaves the text of the provision as it is. -/
def Clause.Vacuous (c : Clause α) (text : List α) : Prop :=
  c.apply text = text

instance (c : Clause α) (text : List α) : Decidable (c.Vacuous text) :=
  inferInstanceAs (Decidable (c.apply text = text))

/-- A piece with the places of `c` written out as plain characters again. -/
def Seg.without (c : Clause α) : Seg α → List (Seg α)
  | .lit a => [.lit a]
  | .hit d => if d = c then d.quoted.map .lit else [.hit d]

/-- A piece as it stands when the clauses before `c` have been read and `c` has not. -/
def Seg.before (c : Clause α) : Seg α → List α
  | .lit a => [a]
  | .hit d => if d = c then d.quoted else d.replacement

/-- The words `q` open at no character of `w` in the text `w ++ u`. -/
def noStart (q : List α) : List α → List α → Bool
  | [], _ => true
  | a :: w, u => !starts q (a :: w ++ u) && noStart q w u

/-- In the text where the clauses before `c` have been read, the quoted words of `c` open only at
the places of `c`: not at a character no clause took, and not inside what another clause wrote. -/
def clean (c : Clause α) : List (Seg α) → Bool
  | [] => true
  | .lit a :: s => !starts c.quoted (a :: s.flatMap (Seg.before c)) && clean c s
  | .hit d :: s =>
    (if d = c then !c.quoted.isEmpty else noStart c.quoted d.replacement (s.flatMap (Seg.before c)))
      && clean c s

/-- `clean` in one pass from the right: the text as it stands before `c` is read, and whether the
pieces are clean. -/
def cleanScan (c : Clause α) : List (Seg α) → List α × Bool
  | [] => ([], true)
  | x :: s =>
    let (u, ok) := cleanScan c s
    match x with
    | .lit a => (a :: u, !starts c.quoted (a :: u) && ok)
    | .hit d =>
      if d = c then (d.quoted ++ u, !c.quoted.isEmpty && ok)
      else (d.replacement ++ u, noStart c.quoted d.replacement u && ok)

theorem cleanScan_eq (c : Clause α) (s : List (Seg α)) :
    cleanScan c s = (s.flatMap (Seg.before c), clean c s) := by
  induction s with
  | nil => rfl
  | cons x s ih =>
    cases x with
    | lit a => simp [cleanScan, ih, clean, Seg.before]
    | hit d => by_cases hd : d = c <;> simp [cleanScan, ih, clean, Seg.before, hd]

/-- `clean` as it runs. -/
def cleanFast (c : Clause α) (s : List (Seg α)) : Bool :=
  (cleanScan c s).2

@[csimp] theorem clean_eq_cleanFast : @clean = @cleanFast := by
  funext α _ c s
  simp [cleanFast, cleanScan_eq]

/-- The clause `c`, written after the clauses `cs`, is independent of them on the text: the
places of `cs` are the same with and without `c`, and the quoted words of `c` do not appear in
what `cs` wrote. -/
def step (cs : List (Clause α)) (c : Clause α) (t : List α) : Bool :=
  let s := segs (cs ++ [c]) 0 t
  decide (segs cs 0 t = s.flatMap (Seg.without c)) && clean c s

/-- `independent` on the clauses in reverse order. -/
def independentRev : List (Clause α) → List α → Bool
  | [], _ => true
  | c :: rs, t => step rs.reverse c t && independentRev rs t

/-- Every clause is independent of the clauses written before it. -/
def independent (cs : List (Clause α)) (t : List α) : Bool :=
  independentRev cs.reverse t

theorem isPrefixOf_append_self (q u : List α) : q.isPrefixOf (q ++ u) = true := by
  induction q with
  | nil => simp
  | cons a q ih => simp [ih]

theorem segs_append (cs : List (Clause α)) (w u : List α) :
    segs cs w.length (w ++ u) = segs cs 0 u := by
  induction w with
  | nil => rfl
  | cons a w ih => simpa [segs] using ih

theorem applyAll_nil_aux (n : Nat) (t : List α) :
    (segs ([] : List (Clause α)) n t).flatMap Seg.read = t.drop n := by
  induction t generalizing n with
  | nil => cases n <;> simp [segs]
  | cons a t ih =>
    cases n with
    | zero => simp [segs, pick, Seg.read, ih 0]
    | succ n => simp [segs, ih n]

theorem applyAll_nil (t : List α) : applyAll ([] : List (Clause α)) t = t := by
  simpa [applyAll] using applyAll_nil_aux 0 t

/-- Where the quoted words do not open, the clause passes the character. -/
theorem apply_cons (c : Clause α) (a : α) (u : List α) (h : starts c.quoted (a :: u) = false) :
    c.apply (a :: u) = a :: c.apply u := by
  simp [Clause.apply, applyAll, segs, pick, h, Seg.read]

/-- Where the quoted words open, the clause writes its replacement and goes on after them. -/
theorem apply_quoted (c : Clause α) (u : List α) (h : c.quoted ≠ []) :
    c.apply (c.quoted ++ u) = c.replacement ++ c.apply u := by
  cases hq : c.quoted with
  | nil => exact absurd hq h
  | cons a q =>
    have hs : starts c.quoted (a :: (q ++ u)) = true := by
      have := isPrefixOf_append_self (a :: q) u
      simp [starts, hq] at this ⊢
    have hl : c.quoted.length - 1 = q.length := by simp [hq]
    simp [Clause.apply, applyAll, segs, pick, hs, Seg.read, hl, segs_append]

theorem apply_noStart (c : Clause α) (w u : List α) (h : noStart c.quoted w u = true) :
    c.apply (w ++ u) = w ++ c.apply u := by
  induction w with
  | nil => rfl
  | cons a w ih =>
    simp [noStart] at h
    have := apply_cons c a (w ++ u) (by simpa using h.1)
    simp [this, ih h.2]

omit [DecidableEq α] in
theorem read_lits (w : List α) : (w.map Seg.lit).flatMap Seg.read = w := by
  induction w with
  | nil => rfl
  | cons a w ih => simp [Seg.read, ih]

theorem read_without (c : Clause α) (s : List (Seg α)) :
    (s.flatMap (Seg.without c)).flatMap Seg.read = s.flatMap (Seg.before c) := by
  induction s with
  | nil => rfl
  | cons x s ih =>
    cases x with
    | lit a => simp [Seg.without, Seg.before, Seg.read, ih]
    | hit d =>
      by_cases hd : d = c
      · simp [Seg.without, Seg.before, hd, read_lits, ih]
      · simp [Seg.without, Seg.before, hd, Seg.read, ih]

theorem apply_clean (c : Clause α) (s : List (Seg α)) (h : clean c s = true) :
    c.apply (s.flatMap (Seg.before c)) = s.flatMap Seg.read := by
  induction s with
  | nil => simp [Clause.apply, applyAll, segs]
  | cons x s ih =>
    cases x with
    | lit a =>
      simp [clean] at h
      have := apply_cons c a (s.flatMap (Seg.before c)) (by simpa using h.1)
      simp [Seg.before, Seg.read, this, ih h.2]
    | hit d =>
      by_cases hd : d = c
      · simp [clean, hd] at h
        have := apply_quoted c (s.flatMap (Seg.before c)) (by simpa using h.1)
        simp [Seg.before, Seg.read, hd, this, ih h.2]
      · simp [clean, hd] at h
        have := apply_noStart c d.replacement (s.flatMap (Seg.before c)) h.1
        simp [Seg.before, Seg.read, hd, this, ih h.2]

/-- Reading an independent clause after the others gives the reading of all at once. -/
theorem apply_applyAll (cs : List (Clause α)) (c : Clause α) (t : List α)
    (h : step cs c t = true) : c.apply (applyAll cs t) = applyAll (cs ++ [c]) t := by
  simp [step] at h
  rw [applyAll, h.1, read_without, apply_clean c _ h.2, applyAll]

theorem applySeq_eq_applyAll_rev (rs : List (Clause α)) (t : List α)
    (h : independentRev rs t = true) : applySeq rs.reverse t = applyAll rs.reverse t := by
  induction rs with
  | nil => simp [applySeq, applyAll_nil]
  | cons c rs ih =>
    simp [independentRev] at h
    have hs := ih h.2
    simp only [applySeq] at hs
    simp only [applySeq, List.reverse_cons, List.foldl_append, List.foldl_cons, List.foldl_nil, hs]
    exact apply_applyAll rs.reverse c t h.1

/-- For independent clauses, replacing clause by clause in the order written gives the text read
under all the clauses at once. -/
theorem applySeq_eq_applyAll (cs : List (Clause α)) (t : List α)
    (h : independent cs t = true) : applySeq cs t = applyAll cs t := by
  simpa using applySeq_eq_applyAll_rev cs.reverse t h

/-- Two clauses that exchange two words: read at once the words change places, replaced one
after the other both become the first word. The clauses are not independent. -/
theorem swap_differs (x y : α) (h : x ≠ y) :
    applyAll [⟨[x], [y]⟩, ⟨[y], [x]⟩] [x, y] = [y, x]
      ∧ applySeq [⟨[x], [y]⟩, ⟨[y], [x]⟩] [x, y] = [x, x]
      ∧ independent [⟨[x], [y]⟩, ⟨[y], [x]⟩] [x, y] = false := by
  have h' : y ≠ x := fun e => h e.symm
  refine ⟨?_, ?_, ?_⟩ <;>
    simp [applyAll, applySeq, Clause.apply, independent, independentRev, step, clean, noStart,
      segs, pick, starts, Seg.read, Seg.without, Seg.before, h, h']

/-- Where the quoted words of two clauses open, the longer are read, in whichever order the
clauses are written. -/
theorem pick_longer (x y : α) (r r' : List α) :
    pick [⟨[x], r⟩, ⟨[x, y], r'⟩] [x, y] = some ⟨[x, y], r'⟩
      ∧ pick [⟨[x, y], r'⟩, ⟨[x], r⟩] [x, y] = some ⟨[x, y], r'⟩ := by
  simp [pick, starts]

/-- Of two clauses with the same quoted words the one written first is read. -/
theorem pick_first (x : α) (r r' : List α) :
    pick [⟨[x], r⟩, ⟨[x], r'⟩] [x] = some ⟨[x], r⟩ := by
  simp [pick, starts]

/-- Two clauses for two different words, neither writing the word of the other, are
independent. -/
theorem independent_apart (x y a b : α) (hxy : x ≠ y) (hay : a ≠ y) :
    independent [⟨[x], [a]⟩, ⟨[y], [b]⟩] [x, y] = true := by
  have hyx : y ≠ x := fun e => hxy e.symm
  have hya : y ≠ a := fun e => hay e.symm
  simp [independent, independentRev, step, clean, noStart, segs, pick, starts, Seg.without,
    Seg.before, List.isPrefixOf, hxy, hyx, hya]

/-- A clause written after another with the same quoted words is never read, and the two are
independent when the first does not write those words again. -/
theorem independent_same_words (x a b : α) (h : a ≠ x) (hab : a ≠ b) :
    independent [⟨[x], [a]⟩, ⟨[x], [b]⟩] [x] = true := by
  have h' : x ≠ a := fun e => h e.symm
  simp [independent, independentRev, step, clean, noStart, segs, pick, starts, Seg.without,
    List.isPrefixOf, h', hab]

theorem apply_of_not_occurs (c : Clause α) :
    ∀ t : List α, occurs c.quoted t = false → c.apply t = t := by
  intro t
  induction t with
  | nil => intro _; simp [Clause.apply, applyAll, segs]
  | cons a t ih =>
    intro h
    simp only [occurs, Bool.or_eq_false_iff] at h
    rw [apply_cons c a t (by simp [starts, h.1]), ih h.2]

/-- A read-as clause whose quoted words do not occur in the provision changes nothing. -/
theorem vacuous_of_absent (c : Clause α) (text : List α)
    (h : occurs c.quoted text = false) : c.Vacuous text :=
  apply_of_not_occurs c text h

end Ritsulint.ReadAs
