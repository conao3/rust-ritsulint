import Assay
import Ritsulint.Api
import Audit.Core

/-!
Generators of clauses over three characters, so that quoted words meet a random text often
enough to tell a broken function from the original. The import of `Audit.Core` makes Lake run the
two audits one after the other: run together they slow each other into timeouts.
-/

open Ritsulint.ReadAs Plausible

def smallWords : Gen (List Int) := do
  let n ← Gen.choose Nat 0 2 (by omega)
  let mut out := []
  for _ in [0:n.val + 1] do
    let x ← Gen.choose Nat 0 2 (by omega)
    out := (Int.ofNat x.val - 1) :: out
  pure out

instance : Shrinkable (Clause Int) := ⟨fun _ => []⟩

instance : Arbitrary (Clause Int) :=
  ⟨do
    let q ← smallWords
    let r ← smallWords
    pure ⟨q, r⟩⟩

#assay Ritsulint.ReadAs.starts
#assay Ritsulint.ReadAs.pick
#assay Ritsulint.ReadAs.segs
#assay Ritsulint.ReadAs.applyAll
#assay Ritsulint.ReadAs.applySeq
#assay Ritsulint.ReadAs.noStart
#assay Ritsulint.ReadAs.clean
#assay Ritsulint.ReadAs.step
#assay Ritsulint.ReadAs.independentRev
