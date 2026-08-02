-- Analyses, and the queue they are claimed from.
--
-- The queue is not a separate table: an analysis in state 'queued' IS the queue entry.
-- Splitting them would make "the job exists" and "the row exists" two facts in two places,
-- which is one more thing that can disagree.

CREATE TABLE IF NOT EXISTS analyses (
    id              uuid        PRIMARY KEY,
    prompt          text        NOT NULL,
    status          text        NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    started_at      timestamptz,
    finished_at     timestamptz,
    -- The whole report, as generated. Stored as jsonb so it can be queried without a
    -- migration every time the schema gains a section.
    report          jsonb,
    -- Recorded for operators. Never shown to a reader verbatim: what a user is told about
    -- a failure is a presentation decision, not a database field.
    failure_reason  text,

    CONSTRAINT analyses_status_known
        CHECK (status IN ('queued', 'running', 'complete', 'failed')),

    -- A completed analysis without a report would render as a blank page. Enforce the
    -- pairing here so no code path can produce one.
    CONSTRAINT analyses_complete_has_report
        CHECK (status <> 'complete' OR report IS NOT NULL)
);

-- The claim query orders queued rows by age. Restricting the index to queued rows keeps
-- it small no matter how many finished analyses accumulate.
CREATE INDEX IF NOT EXISTS analyses_queue_idx
    ON analyses (created_at)
    WHERE status = 'queued';

CREATE INDEX IF NOT EXISTS analyses_created_at_idx ON analyses (created_at DESC);
