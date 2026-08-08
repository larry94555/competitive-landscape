-- The companies a reader picks between when a name matched several.
--
-- `failure_kind` says a name matched more than one company. It cannot say *which*, so the
-- interface could name the situation and not the question — and a reader was left to guess at
-- exactly what `landscape_analyze::subject::decide` refused to guess at. The gate had already
-- resolved them: a name read off each company's own front page, a canonical domain, and the one
-- line that tells it apart from the others.
--
-- **This is not `failure_reason`, and it deliberately does not go in it.** `0001_init.sql` is
-- explicit that `failure_reason` is written for operators and never shown verbatim; every entry
-- here is shown verbatim, to a reader, as the label on a button. Two audiences, two columns. A
-- human-readable field is an output, not a source.
--
-- **jsonb rather than a table.** These are read as a whole or not at all, never queried across
-- analyses, and never joined. A `clarification_choices` table would buy indexing nobody needs
-- and cost a second write on the failure path — and this list is rewritten on every refusal,
-- empty included, so that a later attempt cannot inherit a question it never asked.
--
-- Nullable, because every analysis that has never failed has no question attached, and rows
-- written before this column existed have none either. `analysis_from_row` reads NULL as an
-- empty list and zeroes it unless the status is `failed`.
ALTER TABLE analyses ADD COLUMN IF NOT EXISTS clarification jsonb;

COMMENT ON COLUMN analyses.clarification IS 'The companies offered when a name matched several. Shown to a reader verbatim, unlike failure_reason. Empty or NULL unless failure_kind = ''ambiguous''.';
