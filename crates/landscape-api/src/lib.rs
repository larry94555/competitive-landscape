//! The HTTP surface.
//!
//! Handlers take a `&dyn Store`, so every test in this crate runs against
//! [`MemoryStore`](landscape_db::MemoryStore) with no database, no Docker and no network.
//! That is the whole reason the trait exists: a request path nobody can exercise on a
//! laptop is a request path nobody exercises.

pub mod cap;
mod error;
mod events;
mod extract;
pub mod request_id;
mod routes;

pub use cap::{Allowed, Cap};
pub use error::ApiError;
pub use request_id::RequestId;
pub use routes::{router, web_dir, with_ui, AppState};
