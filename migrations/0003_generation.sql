-- Which generation of this analysis is the current one.
--
-- A worker claims a row and then spends ninety seconds on it. If it takes longer than the
-- staleness threshold, the sweep hands the row to a second worker while the first is still
-- alive — and until now the first could finish, write its report over the second's, and
-- nothing anywhere could tell the two apart. `status` cannot: both workers see `running`.
--
-- So a claim is a number, not a state. `claim_next` and `reclaim_stale` both raise it, which
-- makes it mean "how many times has this run been started". A worker carries the number it
-- claimed, and every write it makes says so; a write quoting an old number is a worker whose
-- claim was revoked, and is refused rather than applied.
--
-- It is also what lets a reader be told their report is starting over. The number goes out on
-- the stream, so a client that reconnects into a different attempt knows the sections it is
-- holding belong to a run that no longer exists — which the connection itself cannot know,
-- because a new connection has no memory of the old one.
ALTER TABLE analyses ADD COLUMN IF NOT EXISTS generation integer NOT NULL DEFAULT 0;

ALTER TABLE analyses DROP CONSTRAINT IF EXISTS analyses_generation_not_negative;
ALTER TABLE analyses ADD CONSTRAINT analyses_generation_not_negative CHECK (generation >= 0);
