import Ritsulint.Core
import Ritsulint.ReadAs

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

def refsOf (text : String) : List (String × Nat × Nat) :=
  (relativeRefs (unquote text)).map fun r => (r.kind, r.count, r.offset)

/-- 鉄道事業法施行規則 第46条第2項: `前条第二項第一号` names paragraph 2 of the article before. -/
example :
    refsOf "前条第二項第一号の索道施設の設置の場所を示す図面には、次に掲げる事項を記載しなければならない。"
      = [("prev-article-paragraph", 2, 0)] := by
  decide +kernel

/-- 漁業災害補償法施行規則 第31条第2項: `前項第五号` is also a reference to the paragraph before. -/
example :
    refsOf "前項の払戻しを請求する場合は、前項第五号の当該時期による。"
      = [("prev-paragraphs", 1, 0), ("prev-paragraph-item", 5, 15), ("prev-paragraphs", 1, 15)] := by
  decide +kernel

/-- A reference inside a quotation is not a reference of the paragraph. -/
example :
    refsOf "この場合において、同条中「前項」とあるのは「次項」と読み替えるものとする。" = [] := by
  decide +kernel

/-- 経済産業省組織規則 第314条第5号 and 行政改革…特例措置に関する法律 第14条第2項. -/
example : refsOf "前五号に掲げる事務のほか" = [("prev-items", 5, 0)] := by decide +kernel
example : refsOf "前二項の規定の適用があるものとして" = [("prev-paragraphs", 2, 0)] := by
  decide +kernel
example : refsOf "次項において同じ。" = [("next-paragraph", 1, 0)] := by decide +kernel

/-- 経済産業省関係産業競争力強化法施行規則 第60条: `法第九十八第二項` lacks 条. -/
example :
    refsOf "法第九十八第二項第二号の経済産業省令で定める方法" = [("missing-unit", 98, 1)] := by
  decide +kernel
example : refsOf "法第二十七の二十第二項の規定" = [("missing-unit", 27, 1)] := by decide +kernel
example : refsOf "法第九十八条第二項第二号" = [] := by decide +kernel

example : kansujiOr0 "四十四" = 44 := by decide
example : kansujiOr0 "第四" = 0 := by decide

/-- A read-as clause replaces every occurrence of its quoted words, and leaves a text without
them as it is. -/
example : ReadAs.occurs [1, 2] [0, 1, 2, 3] = true := by decide
example : ReadAs.occurs [1, 2] [0, 1, 3, 2] = false := by decide
example : (ReadAs.Clause.apply ⟨[1, 2], [9]⟩ [0, 1, 2, 3, 1, 2]) = [0, 9, 3, 9] := by decide
example : (ReadAs.Clause.apply ⟨[1, 2], [9]⟩ [0, 1, 3, 2]) = [0, 1, 3, 2] := by decide
example : ¬ ReadAs.Clause.Vacuous ⟨[1, 2], [9]⟩ [0, 1, 2, 3] := by decide

end Ritsulint.Test
