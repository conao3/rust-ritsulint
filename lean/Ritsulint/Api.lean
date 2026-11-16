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

end Ritsulint.Api
