import Ritsulint.Core
import Ritsulint.ReadAs

/-!
# The functions the Rust crate calls

leanfran compiles this module, with everything it calls in `Ritsulint.Core` and
`Ritsulint.ReadAs`, to one Rust module in `src/generated/`.
-/

namespace Ritsulint.Api

/-- The read-as clauses of a paragraph with the provisions each names. See `Ritsulint.analyze`. -/
def analyze (text article prevArticle nextArticle : String) (paragraph : Nat) :
    List Ritsulint.Finding :=
  Ritsulint.analyze text article prevArticle nextArticle paragraph

/-- The text without its parenthesized remarks. -/
def stripParens (text : String) : String :=
  Ritsulint.stripParens text

/-- The text with each quotation replaced by one `〓`. -/
def unquote (text : String) : String :=
  Ritsulint.unquote text

/-- The references by position and the references without a unit in an unquoted text. -/
def relativeRefs (text : String) : List Ritsulint.RelRef :=
  Ritsulint.relativeRefs text

/-- The number a kanji numeral writes, or 0 when the text is not one. -/
def kansuji (text : String) : Nat :=
  Ritsulint.kansujiOr0 text

/-- A read-as clause as the Rust crate passes it. -/
structure ReadClause where
  quoted : String
  replacement : String

/-- A provision read under its clauses. See `Ritsulint.ReadAs`. `atOnce` is the reading the linter
takes, `inOrder` the text after replacing clause by clause, `fromRight` the reading from the
right. `independent` holds when `Ritsulint.ReadAs.applySeq_eq_applyAll` applies. -/
structure Reading where
  atOnce : String
  inOrder : String
  fromRight : String
  independent : Bool

/-- The text of a provision read under the clauses written for it. -/
def readAs (clauses : List ReadClause) (text : String) : Reading :=
  let cs : List (Ritsulint.ReadAs.Clause Char) :=
    clauses.map fun c => ⟨c.quoted.toList, c.replacement.toList⟩
  let t := text.toList
  { atOnce := String.ofList (Ritsulint.ReadAs.applyAll cs t)
    inOrder := String.ofList (Ritsulint.ReadAs.applySeq cs t)
    fromRight := String.ofList (Ritsulint.ReadAs.rightToLeft cs t)
    independent := Ritsulint.ReadAs.independent cs t }

/-- For clauses the linter reports as independent, replacing clause by clause gives the text read
at once. This is `Ritsulint.ReadAs.applySeq_eq_applyAll` for the function the Rust crate calls. -/
theorem readAs_inOrder (clauses : List ReadClause) (text : String)
    (h : (readAs clauses text).independent = true) :
    (readAs clauses text).inOrder = (readAs clauses text).atOnce := by
  simp only [readAs] at h ⊢
  rw [Ritsulint.ReadAs.applySeq_eq_applyAll _ _ h]

end Ritsulint.Api
