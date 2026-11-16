/-!
# Reading read-as clauses out of a paragraph of Japanese law

A read-as clause has the form `第七条第一項中「A」とあるのは「B」と`: in the provision named before
`中`, the words `A` are to be read as `B`. This module splits the text of a paragraph into such
clauses and resolves the provisions each clause names, relative to the paragraph it stands in.

- `parseKansuji` reads a number written in kanji numerals (`百五十八` is 158).
- `rawClauses` splits a paragraph into clauses: the words before `中`, the quoted words and the
  replacement.
- `analyze` resolves the words before `中` of every clause to provisions of the same law, or
  marks the clause as naming another law (`external`) or as written in a form this module does
  not resolve (`unsupported`).
- `stripParens` removes parenthesized remarks, for comparing a quotation with a provision that
  defines an abbreviation in the middle of the quoted words.
-/

namespace Ritsulint

def kansujiDigit? : Char → Option Nat
  | '一' => some 1
  | '二' => some 2
  | '三' => some 3
  | '四' => some 4
  | '五' => some 5
  | '六' => some 6
  | '七' => some 7
  | '八' => some 8
  | '九' => some 9
  | _ => none

def kansujiUnit? : Char → Option Nat
  | '十' => some 10
  | '百' => some 100
  | '千' => some 1000
  | _ => none

def isKansuji (c : Char) : Bool :=
  (kansujiDigit? c).isSome || (kansujiUnit? c).isSome

/-- One step of reading a kanji numeral. The state is the total so far and a digit that still
waits for its unit. Two digits in a row are not a numeral. -/
def kansujiStep (st : Nat × Nat) (c : Char) : Option (Nat × Nat) :=
  match kansujiDigit? c with
  | some d => if st.2 = 0 then some (st.1, d) else none
  | none =>
    match kansujiUnit? c with
    | some u => some (st.1 + (if st.2 = 0 then 1 else st.2) * u, 0)
    | none => none

/-- The number a kanji numeral below 10,000 writes, or `none` for anything else. -/
def parseKansuji (cs : List Char) : Option Nat :=
  match cs with
  | [] => none
  | _ => (cs.foldlM kansujiStep (0, 0)).map fun st => st.1 + st.2

/-- `rest` when `cs` starts with `p`. -/
def stripPrefix? : List Char → List Char → Option (List Char)
  | [], cs => some cs
  | _ :: _, [] => none
  | p :: ps, c :: cs => if p = c then stripPrefix? ps cs else none

/-- A provision of the law a paragraph belongs to. `article` is the `Num` attribute of the
`Article` element (`14_2` for 第十四条の二). `paragraph` is 0 for the whole article. -/
structure Target where
  article : String
  paragraph : Nat
deriving Repr, BEq, DecidableEq

inductive Tok where
  | art (key : String)
  | para (n : Nat)
  | item
  | sameArt
  | samePara
  | prevArt
  | nextArt
  | prevPara
  | nextPara
  | conn
  | filler
  | range
  | all
  | other (c : Char)
deriving Repr, BEq

def keywords : List (List Char × Tok) :=
  [ ("各号列記以外の部分".toList, .filler),
    ("この場合において".toList, .filler),
    ("これらの規定".toList, .all),
    ("ただし書".toList, .filler),
    ("の見出し".toList, .filler),
    ("の規定".toList, .filler),
    ("それぞれ".toList, .filler),
    ("並びに".toList, .conn),
    ("及び".toList, .conn),
    ("又は".toList, .conn),
    ("本文".toList, .filler),
    ("前段".toList, .filler),
    ("後段".toList, .filler),
    ("同条".toList, .sameArt),
    ("同項".toList, .samePara),
    ("前条".toList, .prevArt),
    ("次条".toList, .nextArt),
    ("前項".toList, .prevPara),
    ("次項".toList, .nextPara),
    ("同号".toList, .item),
    ("前号".toList, .item),
    ("から".toList, .range),
    ("まで".toList, .range),
    ("、".toList, .conn) ]

def matchKeyword (cs : List Char) : List (List Char × Tok) → Option (Tok × List Char)
  | [] => none
  | (p, t) :: rest =>
    match stripPrefix? p cs with
    | some r => some (t, r)
    | none => matchKeyword cs rest

/-- The branch numbers after an article or item number: `の二の三` gives `[2, 3]`. -/
def branches : Nat → List Char → List Nat → List Nat × List Char
  | 0, cs, acc => (acc.reverse, cs)
  | fuel + 1, 'の' :: cs, acc =>
    let (num, rest) := cs.span isKansuji
    match parseKansuji num with
    | some n => branches fuel rest (n :: acc)
    | none => (acc.reverse, 'の' :: cs)
  | _, cs, acc => (acc.reverse, cs)

def joinKey (n : Nat) (bs : List Nat) : String :=
  bs.foldl (fun s b => s ++ "_" ++ toString b) (toString n)

/-- Reads `第…条`, `第…項` or `第…号` after the `第`. -/
def numbered (cs : List Char) : Option (Tok × List Char) :=
  let (num, rest) := cs.span isKansuji
  match parseKansuji num, rest with
  | some n, '条' :: rest =>
    let (bs, rest) := branches rest.length rest []
    some (.art (joinKey n bs), rest)
  | some n, '項' :: rest => some (.para n, rest)
  | some _, '号' :: rest =>
    let (_, rest) := branches rest.length rest []
    some (.item, rest)
  | _, _ => none

def tokenize : Nat → List Char → List Tok
  | 0, _ => []
  | _, [] => []
  | fuel + 1, '第' :: cs =>
    match numbered cs with
    | some (t, rest) => t :: tokenize fuel rest
    | none => .other '第' :: tokenize fuel cs
  | fuel + 1, c :: cs =>
    match matchKeyword (c :: cs) keywords with
    | some (t, rest) => t :: tokenize fuel rest
    | none => .other c :: tokenize fuel cs

/-- Characters that end the name of a law or of another kind of rule (法, 令, 規則, 規程, 規範,
条約, 定款, 附則). A clause with one of them before `中` names a provision outside the main
provisions of the law it stands in. -/
def lawish (c : Char) : Bool :=
  c = '法' || c = '令' || c = '則' || c = '程' || c = '範' || c = '約' || c = '款' || c = '旧'

/-- Where a paragraph stands: its article, the articles before and after, its number. -/
structure Context where
  article : String
  prevArticle : String
  nextArticle : String
  paragraph : Nat

structure RefState where
  openRef : Option Target
  done : List Target
  last : Option Target

def RefState.flush (s : RefState) : RefState :=
  match s.openRef with
  | some t => { s with openRef := none, done := t :: s.done, last := some t }
  | none => s

def RefState.start (s : RefState) (t : Target) : RefState :=
  { s.flush with openRef := some t }

def RefState.current (s : RefState) : Option Target :=
  match s.openRef with
  | some t => some t
  | none => s.last

def refStep (ctx : Context) (s : RefState) : Tok → Option RefState
  | .art key => some (s.start ⟨key, 0⟩)
  | .sameArt => s.current.map fun t => s.start ⟨t.article, 0⟩
  | .prevArt => if ctx.prevArticle = "" then none else some (s.start ⟨ctx.prevArticle, 0⟩)
  | .nextArt => if ctx.nextArticle = "" then none else some (s.start ⟨ctx.nextArticle, 0⟩)
  | .samePara => s.current.map fun t => s.start t
  | .prevPara =>
    if ctx.paragraph ≤ 1 then none else some (s.start ⟨ctx.article, ctx.paragraph - 1⟩)
  | .nextPara => some (s.start ⟨ctx.article, ctx.paragraph + 1⟩)
  | .para n =>
    match s.openRef with
    | some t =>
      if t.paragraph = 0 then some { s with openRef := some ⟨t.article, n⟩ }
      else some (s.start ⟨t.article, n⟩)
    | none => none
  | _ => some s

/-- What the words before `中` of a clause name. -/
inductive Scope where
  | refs (targets : List Target) (afterRemark : Bool)
  | inherit
  | external
  | unsupported
deriving Repr, BEq

def isOther : Tok → Bool
  | .other _ => true
  | _ => false

def isLawishOther : Tok → Bool
  | .other c => lawish c
  | _ => false

def isRefTok : Tok → Bool
  | .art _ | .para _ | .sameArt | .samePara | .prevArt | .nextArt | .prevPara | .nextPara => true
  | _ => false

def resolveRefs (ctx : Context) (last : Option Target) (toks : List Tok) (afterRemark : Bool) :
    Scope :=
  if toks.any (fun t => t == .range || t == .all) then .unsupported
  else if !toks.any isRefTok then (if afterRemark then .unsupported else .inherit)
  else
    match toks.foldlM (refStep ctx) ⟨none, [], last⟩ with
    | some s => .refs s.flush.done.reverse afterRemark
    | none => .unsupported

/-- Resolves the words before `中`. Words that are not part of a reference (`届出については、`)
may come first when a comma separates them from the reference; `afterRemark` then tells whether
those words contain the name of a law or rule. A reference that follows such words without a
comma (`拳銃規範第十条第一項`) belongs to that law or rule. -/
def resolveScope (ctx : Context) (last : Option Target) (scope : List Char) : Scope :=
  let toks := tokenize scope.length scope
  if !toks.any isOther then resolveRefs ctx last toks false
  else
    let tail := (toks.reverse.takeWhile (fun t => !isOther t)).reverse
    match tail with
    | .conn :: rest => resolveRefs ctx last rest (toks.any isLawishOther)
    | _ => if toks.any isLawishOther then .external else .unsupported

/-- A clause as written: the words before `中` (without the `中`), the quoted words, the
replacement. `hasNaka` is false when the words before the quotation do not end in `中`. -/
structure RawClause where
  scope : List Char
  quoted : List Char
  replacement : List Char
  hasNaka : Bool

/-- The body of a quotation and the text after its closing bracket. The input starts after the
opening bracket. Brackets inside the quotation nest. -/
def takeQuote : List Char → Nat → List Char → Option (List Char × List Char)
  | [], _, _ => none
  | '」' :: rest, 0, acc => some (acc.reverse, rest)
  | '」' :: rest, d + 1, acc => takeQuote rest d ('」' :: acc)
  | '「' :: rest, d, acc => takeQuote rest (d + 1) ('「' :: acc)
  | c :: rest, d, acc => takeQuote rest d (c :: acc)

def dropOne (c : Char) : List Char → List Char
  | x :: rest => if x = c then rest else x :: rest
  | [] => []

def mkClause (segRev quoted replacement : List Char) : RawClause :=
  match segRev with
  | '中' :: rest => ⟨rest.reverse, quoted, replacement, true⟩
  | _ => ⟨segRev.reverse, quoted, replacement, segRev.isEmpty⟩

/-- Splits the text of a paragraph into read-as clauses. `seg` holds, reversed, the text since
the last clause or sentence ended. `pend` holds the clauses of the form `「A」とあり、` that still
wait for the replacement they share with the next `とあるのは`. -/
def scan : Nat → List Char → List Char → List RawClause → List RawClause → List RawClause
  | 0, _, _, _, out => out.reverse
  | _, [], _, _, out => out.reverse
  | fuel + 1, '。' :: rest, _, _, out => scan fuel rest [] [] out
  | fuel + 1, '「' :: rest, seg, pend, out =>
    match takeQuote rest 0 [] with
    | none => out.reverse
    | some (q, rest1) =>
      match stripPrefix? "とあるのは".toList rest1 with
      | some rest2 =>
        match dropOne '、' rest2 with
        | '「' :: r =>
          match takeQuote r 0 [] with
          | none => out.reverse
          | some (b, rest3) =>
            let pend := pend.map fun p => { p with replacement := b }
            scan fuel (dropOne '、' (dropOne 'と' rest3)) [] [] (mkClause seg q b :: pend ++ out)
        | rest3 => scan fuel rest3 [] [] (mkClause seg q [] :: pend ++ out)
      | none =>
        match stripPrefix? "とあり".toList rest1 with
        | some rest2 => scan fuel (dropOne '、' rest2) [] (mkClause seg q [] :: pend) out
        | none => scan fuel rest1 ('＃' :: seg) pend out
  | fuel + 1, c :: rest, seg, pend, out => scan fuel rest (c :: seg) pend out

def rawClauses (text : List Char) : List RawClause :=
  scan text.length text [] [] []

/-- A read-as clause with the provisions it names. `kind` is `refs` (the provisions are in
`targets`), `external` (the clause names another law, or follows one that does) or `unsupported`.
`afterLawName` tells that words naming a law or rule stand before the reference, separated from
it by a comma, so the reference may be to that law. -/
structure Finding where
  scope : String
  quoted : String
  replacement : String
  kind : String
  targets : List Target
  afterLawName : Bool

structure AnalyzeState where
  last : List Target
  lastAfterLawName : Bool
  external : Bool
  out : List Finding

def analyzeStep (ctx : Context) (s : AnalyzeState) (c : RawClause) : AnalyzeState :=
  let mk (kind : String) (targets : List Target) (afterLawName : Bool) : Finding :=
    ⟨String.ofList c.scope, String.ofList c.quoted, String.ofList c.replacement, kind, targets,
      afterLawName⟩
  if s.external then { s with out := mk "external" [] false :: s.out }
  else if !c.hasNaka then { s with last := [], out := mk "unsupported" [] false :: s.out }
  else
    match resolveScope ctx s.last.getLast? c.scope with
    | .refs ts after =>
      { s with last := ts, lastAfterLawName := after, out := mk "refs" ts after :: s.out }
    | .inherit =>
      if s.last.isEmpty then { s with out := mk "unsupported" [] false :: s.out }
      else { s with out := mk "refs" s.last s.lastAfterLawName :: s.out }
    | .external => { s with external := true, last := [], out := mk "external" [] false :: s.out }
    | .unsupported => { s with last := [], out := mk "unsupported" [] false :: s.out }

/-- The read-as clauses of a paragraph, in the order they are written, with the provisions each
names. `article`, `prevArticle` and `nextArticle` are `Num` attributes (`""` when there is no
such article) and `paragraph` is the number of the paragraph the text is taken from. -/
def analyze (text article prevArticle nextArticle : String) (paragraph : Nat) : List Finding :=
  let ctx : Context := ⟨article, prevArticle, nextArticle, paragraph⟩
  ((rawClauses text.toList).foldl (analyzeStep ctx) ⟨[], false, false, []⟩).out.reverse

def stripParensAux : List Char → Nat → List Char → List Char
  | [], _, acc => acc.reverse
  | '（' :: rest, d, acc => stripParensAux rest (d + 1) acc
  | '）' :: rest, d + 1, acc => stripParensAux rest d acc
  | c :: rest, 0, acc => stripParensAux rest 0 (c :: acc)
  | _ :: rest, d + 1, acc => stripParensAux rest (d + 1) acc

/-- The text without its parenthesized remarks. -/
def stripParens (text : String) : String :=
  String.ofList (stripParensAux text.toList 0 [])

end Ritsulint
