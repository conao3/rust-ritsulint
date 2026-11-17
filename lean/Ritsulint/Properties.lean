import Ritsulint.Core

/-!
# Properties of the functions the Rust crate runs

The theorems here are about definitions of `Ritsulint.Core` that leanfran compiles into the
crate, for every input.

- `parseKansuji_toKansuji`: `parseKansuji` reads every number from 1 to 9,999 back from the way
  `toKansuji` writes it. `toKansuji` is a writer defined here from the digits of the number.
- `unquoteAux_plain`, `unquoteAux_quotation`: `unquote` keeps a text without quotations as it is
  and puts one `〓` where a quotation stands, whatever the quoted words are.
- `stripParensAux_plain`, `stripParensAux_remark`: `stripParens` keeps a text without parentheses
  as it is and drops a parenthesized remark with its parentheses.
-/

namespace Ritsulint

/-- The kanji digit of a number from 1 to 9. -/
def kansujiDigitChar : Nat → Char
  | 1 => '一'
  | 2 => '二'
  | 3 => '三'
  | 4 => '四'
  | 5 => '五'
  | 6 => '六'
  | 7 => '七'
  | 8 => '八'
  | _ => '九'

/-- A digit at a unit: nothing for 0, the unit alone for 1, the digit and the unit otherwise. -/
def writeUnit (digit : Nat) (unit : Char) : List Char :=
  if digit = 0 then [] else if digit = 1 then [unit] else [kansujiDigitChar digit, unit]

/-- The digit of the ones. -/
def writeOnes (digit : Nat) : List Char :=
  if digit = 0 then [] else [kansujiDigitChar digit]

/-- A number below 10,000 in kanji numerals, as statutes write it: 2,358 is `二千三百五十八` and
1,110 is `千百十`. -/
def toKansuji (n : Nat) : List Char :=
  writeUnit (n / 1000 % 10) '千' ++ writeUnit (n / 100 % 10) '百' ++ writeUnit (n / 10 % 10) '十' ++
    writeOnes (n % 10)

theorem kansujiStep_digit (st : Nat × Nat) (c : Char) (digit : Nat)
    (h : kansujiDigit? c = some digit) :
    kansujiStep st c = if st.2 = 0 then some (st.1, digit) else none := by
  simp [kansujiStep, h]

theorem kansujiStep_unit (st : Nat × Nat) (c : Char) (value : Nat)
    (h : kansujiDigit? c = none ∧ kansujiUnit? c = some value) :
    kansujiStep st c = some (st.1 + (if st.2 = 0 then 1 else st.2) * value, 0) := by
  simp [kansujiStep, h.1, h.2]

theorem kansujiDigit?_kansujiDigitChar (digit : Nat) (h : 0 < digit ∧ digit < 10) :
    kansujiDigit? (kansujiDigitChar digit) = some digit := by
  have hcases : digit = 1 ∨ digit = 2 ∨ digit = 3 ∨ digit = 4 ∨ digit = 5 ∨ digit = 6 ∨
      digit = 7 ∨ digit = 8 ∨ digit = 9 := by omega
  rcases hcases with h | h | h | h | h | h | h | h | h <;> subst h <;> rfl

theorem foldlM_writeUnit (total digit value : Nat) (unit : Char) (hdigit : digit < 10)
    (hunit : kansujiDigit? unit = none ∧ kansujiUnit? unit = some value) :
    (writeUnit digit unit).foldlM kansujiStep (total, 0) = some (total + digit * value, 0) := by
  unfold writeUnit
  by_cases h0 : digit = 0
  · simp [h0]
  · by_cases h1 : digit = 1
    · simp [h1, kansujiStep_unit _ _ _ hunit]
    · have hchar := kansujiDigit?_kansujiDigitChar digit (by omega)
      simp [h0, h1, kansujiStep_digit _ _ _ hchar, kansujiStep_unit _ _ _ hunit]

theorem foldlM_writeOnes (total digit : Nat) (hdigit : digit < 10) :
    (writeOnes digit).foldlM kansujiStep (total, 0) = some (total, digit) := by
  unfold writeOnes
  by_cases h0 : digit = 0
  · simp [h0]
  · have hchar := kansujiDigit?_kansujiDigitChar digit (by omega)
    simp [h0, kansujiStep_digit _ _ _ hchar]

theorem foldlM_toKansuji (n : Nat) :
    (toKansuji n).foldlM kansujiStep (0, 0) =
      some (n / 1000 % 10 * 1000 + n / 100 % 10 * 100 + n / 10 % 10 * 10, n % 10) := by
  unfold toKansuji
  simp only [List.foldlM_append, Option.bind_eq_bind]
  rw [foldlM_writeUnit 0 _ 1000 '千' (by omega) (by decide)]
  simp only [Option.bind_some]
  rw [foldlM_writeUnit _ _ 100 '百' (by omega) (by decide)]
  simp only [Option.bind_some]
  rw [foldlM_writeUnit _ _ 10 '十' (by omega) (by decide)]
  simp only [Option.bind_some]
  rw [foldlM_writeOnes _ _ (by omega)]
  simp

/-- `parseKansuji` reads every number from 1 to 9,999 back from its kanji numeral. -/
theorem parseKansuji_toKansuji (n : Nat) (h : 0 < n ∧ n < 10000) :
    parseKansuji (toKansuji n) = some n := by
  have hfold := foldlM_toKansuji n
  cases hwritten : toKansuji n with
  | nil =>
    rw [hwritten] at hfold
    simp [List.foldlM] at hfold
    omega
  | cons c cs =>
    rw [hwritten] at hfold
    simp only [parseKansuji, hfold, Option.map_some]
    congr 1
    omega

/-- `kansujiOr0`, which the crate calls, reads every number from 1 to 9,999 back. -/
theorem kansujiOr0_toKansuji (n : Nat) (h : 0 < n ∧ n < 10000) :
    kansujiOr0 (String.ofList (toKansuji n)) = n := by
  simp [kansujiOr0, parseKansuji_toKansuji n h]

theorem unquoteAux_plain (cs : List Char) (h : ∀ c ∈ cs, c ≠ '「') :
    ∀ acc, unquoteAux cs 0 acc = acc.reverse ++ cs := by
  induction cs with
  | nil => intro acc; simp [unquoteAux]
  | cons c rest ih =>
    intro acc
    have hc : c ≠ '「' := h c (by simp)
    have hrest : ∀ c ∈ rest, c ≠ '「' := fun c hmem => h c (by simp [hmem])
    rw [unquoteAux.eq_5 _ _ _ hc, ih hrest]
    simp

theorem unquoteAux_prefix (a rest : List Char) (h : ∀ c ∈ a, c ≠ '「') :
    ∀ acc, unquoteAux (a ++ rest) 0 acc = unquoteAux rest 0 (a.reverse ++ acc) := by
  induction a with
  | nil => intro acc; rfl
  | cons c a ih =>
    intro acc
    have hc : c ≠ '「' := h c (by simp)
    have ha : ∀ c ∈ a, c ≠ '「' := fun c hmem => h c (by simp [hmem])
    rw [List.cons_append, unquoteAux.eq_5 _ _ _ hc, ih ha]
    simp

theorem unquoteAux_quoted (q rest : List Char) (h : ∀ c ∈ q, c ≠ '「' ∧ c ≠ '」') :
    ∀ depth acc, unquoteAux (q ++ rest) (depth + 1) acc = unquoteAux rest (depth + 1) acc := by
  induction q with
  | nil => intro depth acc; rfl
  | cons c q ih =>
    intro depth acc
    have hc := h c (by simp)
    have hq : ∀ c ∈ q, c ≠ '「' ∧ c ≠ '」' := fun c hmem => h c (by simp [hmem])
    rw [List.cons_append, unquoteAux.eq_6 _ _ _ _ hc.1 hc.2, ih hq]

/-- `unquote` keeps a text without quotations as it is. -/
theorem unquote_plain (cs : List Char) (h : ∀ c ∈ cs, c ≠ '「') :
    unquote (String.ofList cs) = String.ofList cs := by
  simp [unquote, unquoteAux_plain cs h]

/-- `unquote` reads a quotation as one `〓`, whatever the quoted words are. -/
theorem unquote_quotation (a q b : List Char) (ha : ∀ c ∈ a, c ≠ '「')
    (hq : ∀ c ∈ q, c ≠ '「' ∧ c ≠ '」') :
    unquote (String.ofList (a ++ '「' :: q ++ '」' :: b)) =
      unquote (String.ofList (a ++ '〓' :: b)) := by
  simp only [unquote, String.toList_ofList]
  rw [List.append_assoc, unquoteAux_prefix a _ ha, unquoteAux_prefix a _ ha]
  rw [List.cons_append, unquoteAux.eq_2, unquoteAux_quoted q _ hq, unquoteAux.eq_4,
    unquoteAux.eq_5 _ _ _ (by decide)]

theorem stripParensAux_plain (cs : List Char) (h : ∀ c ∈ cs, c ≠ '（') :
    ∀ acc, stripParensAux cs 0 acc = acc.reverse ++ cs := by
  induction cs with
  | nil => intro acc; simp [stripParensAux]
  | cons c rest ih =>
    intro acc
    have hc : c ≠ '（' := h c (by simp)
    have hrest : ∀ c ∈ rest, c ≠ '（' := fun c hmem => h c (by simp [hmem])
    rw [stripParensAux.eq_4 _ _ _ hc, ih hrest]
    simp

theorem stripParensAux_prefix (a rest : List Char) (h : ∀ c ∈ a, c ≠ '（') :
    ∀ acc, stripParensAux (a ++ rest) 0 acc = stripParensAux rest 0 (a.reverse ++ acc) := by
  induction a with
  | nil => intro acc; rfl
  | cons c a ih =>
    intro acc
    have hc : c ≠ '（' := h c (by simp)
    have ha : ∀ c ∈ a, c ≠ '（' := fun c hmem => h c (by simp [hmem])
    rw [List.cons_append, stripParensAux.eq_4 _ _ _ hc, ih ha]
    simp

theorem stripParensAux_remark (q rest : List Char) (h : ∀ c ∈ q, c ≠ '（' ∧ c ≠ '）') :
    ∀ depth acc,
      stripParensAux (q ++ rest) (depth + 1) acc = stripParensAux rest (depth + 1) acc := by
  induction q with
  | nil => intro depth acc; rfl
  | cons c q ih =>
    intro depth acc
    have hc := h c (by simp)
    have hq : ∀ c ∈ q, c ≠ '（' ∧ c ≠ '）' := fun c hmem => h c (by simp [hmem])
    rw [List.cons_append, stripParensAux.eq_5 _ _ _ _ hc.1 hc.2, ih hq]

/-- `stripParens` keeps a text without parentheses as it is. -/
theorem stripParens_plain (cs : List Char) (h : ∀ c ∈ cs, c ≠ '（') :
    stripParens (String.ofList cs) = String.ofList cs := by
  simp [stripParens, stripParensAux_plain cs h]

/-- `stripParens` drops a parenthesized remark with its parentheses, whatever the remark is. -/
theorem stripParens_remark (a q b : List Char) (ha : ∀ c ∈ a, c ≠ '（')
    (hq : ∀ c ∈ q, c ≠ '（' ∧ c ≠ '）') :
    stripParens (String.ofList (a ++ '（' :: q ++ '）' :: b)) =
      stripParens (String.ofList (a ++ b)) := by
  simp only [stripParens, String.toList_ofList]
  rw [List.append_assoc, stripParensAux_prefix a _ ha, stripParensAux_prefix a _ ha]
  rw [List.cons_append, stripParensAux.eq_2, stripParensAux_remark q _ hq, stripParensAux.eq_3]

end Ritsulint
