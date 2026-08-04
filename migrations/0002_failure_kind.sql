-- Why an analysis failed, in terms an interface can turn into a sentence.
--
-- `failure_reason` above it is for operators and is never shown verbatim. This is the other
-- half of that rule: a closed set of situations, so a reader can be told something they can
-- act on without anything internal leaking into it.
--
-- The distinction it exists for: "we could not work out which company you meant" is the
-- reader's to fix, and "nothing you did caused it" — which is what every failure used to
-- say — sends them away with no way forward.
ALTER TABLE analyses ADD COLUMN IF NOT EXISTS failure_kind text;

ALTER TABLE analyses DROP CONSTRAINT IF EXISTS analyses_failure_kind_known;
ALTER TABLE analyses ADD CONSTRAINT analyses_failure_kind_known
    CHECK (failure_kind IS NULL OR failure_kind IN ('no_subject', 'internal'));
