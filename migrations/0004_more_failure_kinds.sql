-- The situations a reader is told apart, widened from two to five.
--
-- `failure_kind` was `no_subject` or `internal`, and the analysis had five answers: no engine
-- configured, several companies matching a name, a search that did not finish, a market we
-- looked at and found empty, and companies we found and rejected. All five arrived as
-- `no_subject` and rendered as one sentence — "we could not work out which company you meant;
-- try naming its website".
--
-- That instruction is wrong for a search that timed out, and it throws away the question a
-- reader could have answered in one word. `landscape_analyze::subject::decide` had spent four
-- changes keeping those silences apart; the boundary collapsed them again.
--
-- The set stays closed and stays small: a situation earns a value when a reader would *do
-- something different*. `failure_reason` beside it is still for operators and still never shown
-- verbatim.
ALTER TABLE analyses DROP CONSTRAINT IF EXISTS analyses_failure_kind_known;
ALTER TABLE analyses ADD CONSTRAINT analyses_failure_kind_known
    CHECK (failure_kind IS NULL OR failure_kind IN (
        'no_subject',
        'ambiguous',
        'nothing_found',
        'search_incomplete',
        'internal'
    ));
