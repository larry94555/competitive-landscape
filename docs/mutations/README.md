# Mutations

One file per change: the defects that change is supposed to make impossible, written so they can
be **put back** and the suite asked whether it notices.

```bash
python3 scripts/mutate.py docs/mutations/several-companies.json
python3 scripts/mutate.py docs/mutations/example-ideas.json
python3 scripts/mutate.py docs/mutations/read-order.json
python3 scripts/mutate.py docs/mutations/anonymous-cap.json
python3 scripts/mutate.py docs/mutations/trust-posture.json
```

**Why these are committed when the register says to keep them in a scratchpad.** Most are
throwaway — they belong to one pull request and describe code that has since moved. These are
kept because they are the worked examples: a reader who has never written one can see the shape,
and the set for a change that review found eight defects in is a useful thing to compare
against.

Nine of the fourteen entries in `several-companies.json` were written **after** review found the
defect rather than before, which is what they are for: a finding that arrives from a person is a
finding the suite could not produce, so the first thing the fix owes is a mutation proving it now
can.

Two of them went stale in a later round — the code they aimed at was the code being fixed — and
`mutate.py` said `NOT APPLIED` rather than `MISSED`. That distinction is the file earning its
keep: a mutation that no longer applies is a maintenance job, and one that applies and is missed
is a defect.

`example-ideas.json` earned its keep differently: **three of its twelve reported `MISSED` about
code that was fine.** One appended text to a prompt that the parser steps over as its own word,
so it broke nothing; another removed one of two guards while the second still carried the case.
The docstring's rule — *check the mutation before believing the test is missing* — is there
because this is the usual way a `MISSED` line is wrong, and both were corrected rather than
written up as gaps.

The third is the more interesting failure: a review fix made a mutation's defect **unreachable**.
Removing the `Array.isArray` guard stopped changing anything once every entry was filtered, so
the mutation could no longer fail — which is the thing this harness exists to find, pointed at
itself. It was re-aimed at a rule that was real and unenforced. **Retiring a mutation is a normal
outcome; keeping one to preserve a green line is not.**

**Not every property can be mutated.** *"A prompt that was refused costs nothing"* holds because
the prompt is parsed before anything is counted — a structural fact, not a branch. Every
single-line edit that removes it stops compiling, which `mutate.py` reports as `BROKEN` rather
than pretending. The test stays; the mutation was dropped rather than left in the file proving
nothing, and this paragraph is the reason it is absent.

Every entry must be a defect a person can picture. `"labels are not reassigned when reports
merge"` is one; `"change line 214"` is not, and will mean nothing to whoever reads it next.

**These files are also read backwards.** `scripts/no_live_mutations.py` takes every `new`
payload as the shape of a defect this repository can recognise, and refuses a working tree that
contains one — because an interrupted run leaves the deliberate defect in place, and a
`git add -A` will commit it. That has happened; entry 28 of the register is the account. It is
the first gate `verify.py` runs, and the only one that looks at the working tree rather than at
a clean checkout of `HEAD`.

A file here is only worth keeping while its anchors still match the code. `scripts/mutate.py`
refuses an anchor it cannot find, and an anchor that has silently become non-unique — so a stale
file says so rather than quietly measuring the wrong thing.
