//! The HTTP surface.
//!
//! Handlers take a `&dyn Store`, so every test in this crate runs against
//! [`MemoryStore`](landscape_db::MemoryStore) with no database, no Docker and no network.
//! That is the whole reason the trait exists: a request path nobody can exercise on a
//! laptop is a request path nobody exercises.

mod error;
mod extract;
mod routes;

pub use error::ApiError;
pub use routes::{router, AppState};
