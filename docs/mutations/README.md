# Mutations

One file per change: the defects that change is supposed to make impossible, written so they can
be **put back** and the suite asked whether it notices.

```bash
python3 scripts/mutate.py docs/mutations/several-companies.json
python3 scripts/mutate.py docs/mutations/example-ideas.json
python3 scripts/mutate.py docs/mutations/read-order.json
python3 scripts/mutate.py docs/mutations/anonymous-cap.json
python3 scripts/mutate.py docs/mutations/trust-posture.json
python3 scripts/mutate.py docs/mutations/where-they-invest.json
python3 scripts/mutate.py docs/mutations/the-search-channel.json
python3 scripts/mutate.py docs/mutations/the-second-reader.json
python3 scripts/mutate.py docs/mutations/asking-whether-it-changed.json
python3 scripts/mutate.py docs/mutations/where-the-roles-actually-live.json
python3 scripts/mutate.py docs/mutations/the-set-is-editable.json
python3 scripts/mutate.py docs/mutations/refused-is-not-slow.json
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

`the-search-channel.json` is the first set written *before* the code rather than after a review,
and one of its ten never got to run as written. The guard it aimed at — an explicit refusal of an
empty subject host — turned out to change nothing when removed: `host == ""` is false for every
host `Target::parse` admits, and no host survives `strip_www` ending in a dot. So the guard was
deleted and **the mutation inverted**, putting the wrong rule *in* (`subject.is_empty() || …`,
which makes every page on the web the subject's own domain) where it now fails a test. That is
the same disposition as entry 32 and the paragraph above it: reaching for a new assertion by
reflex is how a rule nothing needs acquires a test that keeps it for ever.

`the-search-channel.json` grew from ten to sixteen after review found seven defects the first
ten could not see, and losing one of them is the more interesting half.

**A mutation was dropped because the property is structural.** Review found that
`Searx::new` ended `.build().unwrap_or_default()`, so the one failure path silently replaced a
client carrying an eight-second timeout with one carrying none. The fix made the constructor
fallible — and the mutation that puts `unwrap_or_default()` back reports `MISSED`, because
`reqwest`'s builder only fails on a TLS backend that will not initialise and no test can make
that happen. The guarantee now is the **signature**: there is no path that produces a `Searx`
without a timeout, which is a property of the type rather than a branch. Same disposition as
*"a prompt that was refused costs nothing"* above — the fix stays, the mutation goes, and this
paragraph is why it is absent.

**And one anchor was refused for being non-unique, which found a test restating a rule.** The
grammar that decides what a subject name may contain appeared twice: once in `quote`, once in a
test that recomputed it to decide what should survive. A test that restates the production rule
passes whatever the rule becomes. It was rewritten as exact expected output — `AT&T` in,
`"AT&T" pricing plans` out — and the anchor became unique as a side effect.

**Not every property can be mutated.** *"A prompt that was refused costs nothing"* holds because
the prompt is parsed before anything is counted — a structural fact, not a branch. Every
single-line edit that removes it stops compiling, which `mutate.py` reports as `BROKEN` rather
than pretending. The test stays; the mutation was dropped rather than left in the file proving
nothing, and this paragraph is the reason it is absent.

Every entry must be a defect a person can picture. `"labels are not reassigned when reports
merge"` is one; `"change line 214"` is not, and will mean nothing to whoever reads it next.

**And sometimes the answer is a dependency.** `an-idea-becomes-companies.json` had a mutation for
a hand-written list of public suffixes, and the mutation passed while the list was wrong: it
tested that the *listed* suffixes worked. What no mutation could test was the entries nobody had
thought of, which is the whole failure mode of a curated subset. Review found `github.io`, the
list became the `psl` crate, and the mutation now asks whether the boundary is computed at all.

**A surviving mutation has two answers, and the second one is easy to miss.**
`where-they-invest.json` produced three, and only two of them wanted a test. The third put back
a plural-matching rule in the careers scanner and nothing failed — because nothing needed the
rule: no title on any of the three frozen pages is plural, and every line it reached was a
navigation label. **The rule was deleted and the mutation inverted**, so it now puts the rule
*back* and a test fails. Entry 32 of the register is the account. Reaching for a new assertion
by reflex is how a rule nothing needs acquires a test that keeps it for ever.

The other two are the reason the harness is worth running on code that already passes: both
tests were green, both assertions were right, and **each named a guard while exercising a
different one**. A sentence was being rejected by the word list rather than by the full stop, and
a navigation label by the plural rule rather than by the length floor. Either guard could have
been removed in silence.

**And a third answer: change the code so the guard can be asserted at all.**
`search-fills-the-gaps.json` labelled every page a search engine returned as the company's own
and the whole suite passed — a stranger's page would have been rendered as the company speaking,
unmarked. Nothing was testing it because the URL and its standing were two arguments to one
function, and a test can only assert a pair that exists. They are one value now, and the mutation
that survived fails. That is entry 7 of the register — a value parted from its evidence — found
by tooling rather than by review.

`the-set-is-editable.json` did the third thing again, and this time the answer was to **delete
code rather than test it.** Two of its eleven entries survived a first run, and one of the two
never stopped surviving: the mutation emptied a `useEffect` that copied the report's set into the
component's state, and the page carried on behaving. Both the effect and the `key` that replaced
it were dead, because a corrected set starts a new run — and the set is only offered once a run is
over, so the component is unmounted between the two reports and reads the second one fresh. The
entry was **removed as a duplicate** of the guard that does the work. Three more were added over two rounds of review, for the ways
into that same cost the guard could not see: adding a company already in the set, adding it in the
spelling the interface itself asks for, and two edits that cancel out. All thirteen are caught. A `MISSED` on a rule that is genuinely enforced somewhere else is not a missing test;
it is a second answer to a question already answered, and a second answer can disagree.

`refused-is-not-slow.json` is the clearest case yet of the harness finding what review would
not. One of its eleven put a constant where a failed search's reason is read off the error, and
**nothing failed** — because every test of the resulting sentence built its input by hand, so the
single line that connects an engine's answer to what a reader is told had no test at all. The
sentences were pinned; the wire between them was not. A second entry came back `BROKEN` rather
than `MISSED`, which is the harness saying *this mutation proves nothing* — it had made the match
non-exhaustive — and that distinction is why the two words are separate.

It kept earning it after review. Two more entries were added for what review found — a `408`
filed as a refusal, and one remedy printed for every refusal — and a fourteenth caught the same
collapse one level further down: the per-query line said *"no answer"* beside a query the engine
had answered `408` to. Three of the fourteen exist because the first fix was too coarse in
exactly the way the change was about.

**These files are also read backwards.** `scripts/no_live_mutations.py` takes every `new`
payload as the shape of a defect this repository can recognise, and refuses a working tree that
contains one — because an interrupted run leaves the deliberate defect in place, and a
`git add -A` will commit it. That has happened; entry 28 of the register is the account. It is
the first gate `verify.py` runs, and the only one that looks at the working tree rather than at
a clean checkout of `HEAD`.

A file here is only worth keeping while its anchors still match the code. `scripts/mutate.py`
refuses an anchor it cannot find, and an anchor that has silently become non-unique — so a stale
file says so rather than quietly measuring the wrong thing.
