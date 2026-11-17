import Ritsulint.Core

/-!
# The functions the Rust crate calls

leanfran compiles this module, with everything it calls in `Ritsulint.Core`, to one Rust module in
`src/generated/`.
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

end Ritsulint.Api
