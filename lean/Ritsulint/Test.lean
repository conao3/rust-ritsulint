import Ritsulint.Core

/-!
# Checks of `Ritsulint.Core` on provisions as they are written

Each `example` is checked by the kernel when the package is built.
-/

namespace Ritsulint.Test

open Ritsulint

example : parseKansuji "百五十八".toList = some 158 := by decide
example : parseKansuji "千九百四十七".toList = some 1947 := by decide
example : parseKansuji "十".toList = some 10 := by decide
example : parseKansuji "二三".toList = none := by decide
example : parseKansuji [] = none := by decide

def targetsOf (text article prev next : String) (paragraph : Nat) : List (String × List Target) :=
  (analyze text article prev next paragraph).map fun f => (f.kind, f.targets)

/-- 出入国管理及び難民認定法 第13条第7項: a reference by article and paragraph, a clause that
inherits the reference before it, and `同条` after a list. -/
example :
    targetsOf
      "第四十条から第四十二条第一項までの規定は、前項の規定による収容に準用する。この場合において、第四十条中「前条第一項の収容令書」とあるのは「第十三条第六項の収容令書」と、「容疑者」とあるのは「仮上陸の許可を受けた外国人」と、第四十一条第一項中「三十日以内とする。但し、主任審査官は」とあるのは「第三章に規定する」と、同条第三項及び第四十二条第一項中「容疑者」とあるのは「仮上陸」と読み替えるものとする。"
      "13" "12" "13_2" 7
      = [("refs", [⟨"40", 0⟩]), ("refs", [⟨"40", 0⟩]), ("refs", [⟨"41", 1⟩]),
         ("refs", [⟨"41", 3⟩, ⟨"42", 1⟩])] := by
  decide +kernel

/-- A clause that names another rule makes the clauses after it external as well. -/
example :
    targetsOf
      "この場合において、拳銃規範第十条第一項中「警察官は、」とあるのは「現場指揮官は」と、同条第三項中「所属長」とあるのは「指定所属長」とする。"
      "14" "13" "15" 2
      = [("external", []), ("external", [])] := by
  decide +kernel

/-- 電波法施行規則 第50条の7第3項: words before the reference, separated by a comma. -/
example :
    targetsOf
      "この場合において、届出については、第五十条の四第五項中「二通」とあるのは、「一通」と読み替えるものとする。"
      "50_7" "50_6" "50_8" 3
      = [("refs", [⟨"50_4", 5⟩])] := by
  decide +kernel

/-- A read-as of a provision that another read-as has already changed is left unresolved. -/
example :
    targetsOf
      "第七十条第三項において読み替えて準用する第三十四条の二第三項中「前条第十項本文及び第十二項」とあるのは「前条第十項本文」と読み替える。"
      "92" "91" "93" 5
      = [("unsupported", [])] := by
  decide +kernel

example : stripParens "社会福祉士法（昭和六十二年法律第三十号。以下「法」という。）第十条第二項"
    = "社会福祉士法第十条第二項" := by
  decide +kernel

/-- 保険業法 第199条: a reference before `各号並びに` is part of the words before `中`. -/
example :
    targetsOf "この場合において、第百五条の二第一項各号並びに同条第二項及び第三項第二号中「A機関」とあるのは「B機関」とする。"
      "199" "198" "200" 1
      = [("refs", [⟨"105_2", 1⟩, ⟨"105_2", 2⟩, ⟨"105_2", 3⟩])] := by
  decide +kernel

end Ritsulint.Test
