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

/// A fresh, isolated schema per test run.
///
/// Two runs against the same database would see each other's rows, and the conformance
/// body asserts on exact counts. A per-run schema is cheaper and more honest than trying
/// to clean up afterwards.
async fn fresh_store() -> Option<PgStore> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let store = PgStore::connect(&url)
        .await
        .expect("DATABASE_URL is set but unreachable - is `docker compose up -d db` running?");

    sqlx::query("DROP TABLE IF EXISTS analyses")
        .execute(store.pool())
        .await
        .expect("drop previous table");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
        .execute(store.pool())
        .await
        .expect("drop migration record");

    store.migrate().await.expect("migrate");
    Some(store)
}

#[tokio::test]
#[ignore = "needs DATABASE_URL and a running Postgres"]
async fn postgres_store_satisfies_the_same_contract() {
    let Some(store) = fresh_store().await else {
        panic!("DATABASE_URL must be set for this test - it is #[ignore]d by default");
    };
    conformance::run(&store).await;
}

#[tokio::test]
#[ignore = "needs DATABASE_URL and a running Postgres"]
async fn two_workers_never_claim_the_same_analysis() {
    // The reason the queue lives in Postgres at all is FOR UPDATE SKIP LOCKED. This is the
    // test that would catch losing it — the in-memory store cannot prove anything about
    // concurrent transactions.
    let Some(store) = fresh_store().await else {
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
}
