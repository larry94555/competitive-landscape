//! The Postgres [`Store`](crate::Store).
//!
//! The job queue lives in the same database as the data it refers to, claimed with
//! `FOR UPDATE SKIP LOCKED`. That is one fewer service to run, back up and pay for, and
//! it makes "queued a job" and "wrote the row" a single transaction — a separate broker
//! would put those two facts in different systems and eventually disagree.
//!
//! Queries use the runtime API rather than `sqlx::query!`. The compile-time macros need a
//! live database or a checked-in offline cache to *compile*, which would mean `cargo
//! build` failing on a machine with nothing installed. That trade is revisited once the
//! schema settles; until then a build that always works is worth more.

use async_trait::async_trait;
use landscape_core::{Analysis, AnalysisId, AnalysisStatus, Failure, NewAnalysis, Report};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::{Result, Store, StoreError};

#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Connect, with a pool sized for a 4-core shared machine rather than a big server.
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    /// Wrap a pool the caller built.
    ///
    /// Exists so a caller can control connection setup — tests use it to give each test
    /// its own Postgres schema, which is what lets them run in parallel against one
    /// database without colliding.
    #[must_use]
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Apply migrations. Safe to call on every boot; applied migrations are skipped.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Corrupt(format!("migration failed: {e}")))
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Build an [`Analysis`] from a row, failing loudly rather than defaulting.
///
/// A status we cannot parse means the database holds something this binary does not
/// understand — usually a rollback to an older version. Guessing would hide that.
fn analysis_from_row(row: &sqlx::postgres::PgRow) -> Result<Analysis> {
    let status_text: String = row.try_get("status")?;
    let status = AnalysisStatus::from_db_str(&status_text)
        .ok_or_else(|| StoreError::Corrupt(format!("unknown status {status_text:?}")))?;

    let report_json: Option<serde_json::Value> = row.try_get("report")?;
    let report = match report_json {
        None => None,
        Some(v) => Some(
            serde_json::from_value::<Report>(v)
                .map_err(|e| StoreError::Corrupt(format!("report does not match schema: {e}")))?,
        ),
    };

    let failure: Option<String> = row.try_get("failure_kind")?;
    Ok(Analysis {
        id: AnalysisId(row.try_get("id")?),
        prompt: row.try_get("prompt")?,
        status,
        created_at: row.try_get("created_at")?,
        report,
        // Only meaningful on a failed row. A kind left behind by a retry that later
        // succeeded would tell a reader an analysis they can see failed.
        failure: (status == AnalysisStatus::Failed)
            .then(|| failure.as_deref().map(Failure::from_db_str))
            .flatten(),
    })
}

const COLUMNS: &str = "id, prompt, status, created_at, report, failure_kind";

#[async_trait]
impl Store for PgStore {
    async fn enqueue(&self, new: &NewAnalysis) -> Result<Analysis> {
        let row = sqlx::query(&format!(
            "INSERT INTO analyses (id, prompt, status) VALUES ($1, $2, $3) RETURNING {COLUMNS}"
        ))
        .bind(uuid::Uuid::new_v4())
        .bind(new.prompt())
        .bind(AnalysisStatus::Queued.as_db_str())
        .fetch_one(&self.pool)
        .await?;
        analysis_from_row(&row)
    }

    async fn get(&self, id: AnalysisId) -> Result<Analysis> {
        let row = sqlx::query(&format!("SELECT {COLUMNS} FROM analyses WHERE id = $1"))
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::NotFound(id))?;
        analysis_from_row(&row)
    }

    async fn claim_next(&self) -> Result<Option<Analysis>> {
        // SKIP LOCKED is what makes this safe to run from several workers at once: a row
        // another transaction has locked is passed over rather than waited on, so workers
        // never queue behind each other for the same job.
        let row = sqlx::query(&format!(
            "UPDATE analyses SET status = $1, started_at = now()
             WHERE id = (
                 SELECT id FROM analyses
                 WHERE status = $2
                 ORDER BY created_at
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1
             )
             RETURNING {COLUMNS}"
        ))
        .bind(AnalysisStatus::Running.as_db_str())
        .bind(AnalysisStatus::Queued.as_db_str())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(analysis_from_row).transpose()
    }

    async fn save_progress(&self, id: AnalysisId, report: &Report) -> Result<()> {
        let json = serde_json::to_value(report)
            .map_err(|e| StoreError::Corrupt(format!("report will not serialise: {e}")))?;
        // `WHERE status = 'running'` rather than by id alone: a worker that lost a race and
        // writes late must not overwrite a finished report, and must not resurrect a failed
        // row. A no-op is the correct outcome there, so no rows affected is not an error.
        sqlx::query("UPDATE analyses SET report = $1 WHERE id = $2 AND status = $3")
            .bind(json)
            .bind(id.0)
            .bind(AnalysisStatus::Running.as_db_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn complete(&self, id: AnalysisId, report: &Report) -> Result<()> {
        let json = serde_json::to_value(report)
            .map_err(|e| StoreError::Corrupt(format!("report will not serialise: {e}")))?;
        let done = sqlx::query(
            "UPDATE analyses SET status = $1, report = $2, finished_at = now() WHERE id = $3",
        )
        .bind(AnalysisStatus::Complete.as_db_str())
        .bind(json)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        if done.rows_affected() == 0 {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    async fn fail(&self, id: AnalysisId, kind: Failure, reason: &str) -> Result<()> {
        let done = sqlx::query(
            "UPDATE analyses SET status = $1, failure_reason = $2, failure_kind = $3,
                    finished_at = now()
             WHERE id = $4",
        )
        .bind(AnalysisStatus::Failed.as_db_str())
        .bind(reason)
        .bind(kind.as_db_str())
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        if done.rows_affected() == 0 {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    async fn reclaim_stale(&self, max_age: chrono::Duration) -> Result<u64> {
        // `started_at` rather than `created_at`: a job that waited an hour in the queue
        // before a worker picked it up has not been running for an hour.
        // `report = NULL` with the requeue: the partial report belongs to the worker that
        // died, and a queued row carrying sections nobody stands behind is worse than an
        // empty one. The replacement run rebuilds from scratch, so keeping it would show a
        // reader answers blanking themselves out mid-run.
        let done = sqlx::query(
            "UPDATE analyses SET status = $1, started_at = NULL, report = NULL
             WHERE status = $2 AND started_at < now() - $3::interval",
        )
        .bind(AnalysisStatus::Queued.as_db_str())
        .bind(AnalysisStatus::Running.as_db_str())
        .bind(format!("{} seconds", max_age.num_seconds()))
        .execute(&self.pool)
        .await?;

        let n = done.rows_affected();
        if n > 0 {
            tracing::warn!(count = n, "returned stale running analyses to the queue");
        }
        Ok(n)
    }

    async fn count_with_status(&self, status: AnalysisStatus) -> Result<i64> {
        let row = sqlx::query("SELECT count(*) AS n FROM analyses WHERE status = $1")
            .bind(status.as_db_str())
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("n")?)
    }
}
