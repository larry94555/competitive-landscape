//! The same contract [`MemoryStore`] passes, run against a real database.
//!
//! Marked `#[ignore]` so `cargo test` stays green on a machine with nothing installed.
//! CI runs `cargo test -- --ignored` with a Postgres service, so the two implementations
//! cannot drift apart without something going red.
//!
//! Run it locally with:
//!
//! ```text
//! docker compose up -d db
//! DATABASE_URL=postgres://landscape:landscape@127.0.0.1:5432/landscape \
//!   cargo test -p landscape-db -- --ignored
//! ```

#![allow(clippy::expect_used, clippy::panic)]

use landscape_db::{conformance, PgStore, Store};
use sqlx::postgres::PgPoolOptions;

/// A store with a Postgres schema all to itself.
///
/// `cargo test` runs test functions on separate threads, so these two tests execute at the
/// same time against one database. An earlier version dropped and recreated a shared
/// `analyses` table in each, which meant whichever test got there second wiped the other's
/// rows — and the conformance body, which asserts on exact counts and claim order, failed
/// with an id it had never enqueued.
///
/// Isolating by schema rather than serializing keeps the tests parallel and, more
/// importantly, keeps them independent: a test that only passes when it runs alone is a
/// test that will fail again the moment a third one is added.
async fn fresh_store() -> Option<(PgStore, String)> {
    let url = std::env::var("DATABASE_URL").ok()?;

    // Unique per run, and a valid unquoted identifier.
    let schema = format!("test_{}", uuid::Uuid::new_v4().simple());

    // Every pooled connection needs the search_path, not just the first — a later
    // connection would otherwise look in `public` and find nothing. Setting it to a schema
    // that does not exist yet is allowed; Postgres resolves the path lazily.
    let for_connect = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .after_connect(move |conn, _meta| {
            let schema = for_connect.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO {schema}"))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("DATABASE_URL is set but unreachable - is `docker compose up -d db` running?");

    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}"))
        .execute(&pool)
        .await
        .expect("create an isolated schema");

    let store = PgStore::from_pool(pool);
    store.migrate().await.expect("migrate");
    Some((store, schema))
}

/// Remove the schema once a test has passed.
///
/// Deliberately only on the success path: a schema left behind by a failing test is the
/// state that failure happened in, which is worth more than a tidy database.
async fn drop_schema(store: &PgStore, schema: &str) {
    let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(store.pool())
        .await;
}

#[tokio::test]
#[ignore = "needs DATABASE_URL and a running Postgres"]
async fn postgres_store_satisfies_the_same_contract() {
    let Some((store, schema)) = fresh_store().await else {
        panic!("DATABASE_URL must be set for this test - it is #[ignore]d by default");
    };
    conformance::run(&store).await;
    drop_schema(&store, &schema).await;
}

#[tokio::test]
#[ignore = "needs DATABASE_URL and a running Postgres"]
async fn two_workers_never_claim_the_same_analysis() {
    // The reason the queue lives in Postgres at all is FOR UPDATE SKIP LOCKED. This is the
    // test that would catch losing it — the in-memory store cannot prove anything about
    // concurrent transactions.
    let Some((store, schema)) = fresh_store().await else {
        panic!("DATABASE_URL must be set for this test");
    };

    const JOBS: usize = 20;
    for i in 0..JOBS {
        let prompt = landscape_core::NewAnalysis::parse(&format!(
            "an application for testing concurrent claims number {i}"
        ))
        .expect("valid prompt");
        store.enqueue(&prompt).await.expect("enqueue");
    }

    // Eight workers racing for twenty jobs.
    let mut handles = Vec::new();
    for _ in 0..8 {
        let s = store.clone();
        handles.push(tokio::spawn(async move {
            let mut mine = Vec::new();
            while let Ok(Some(a)) = s.claim_next().await {
                mine.push(a.id);
            }
            mine
        }));
    }
    let mut claims = Vec::new();
    for h in handles {
        claims.extend(h.await.expect("worker task"));
    }

    assert_eq!(
        claims.len(),
        JOBS,
        "every job should be claimed exactly once"
    );

    let mut unique = claims.clone();
    unique.sort_by_key(|id| id.0);
    unique.dedup();
    assert_eq!(
        unique.len(),
        claims.len(),
        "an analysis was handed to more than one worker"
    );

    drop_schema(&store, &schema).await;
}
