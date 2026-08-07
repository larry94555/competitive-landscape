//! Asking a search engine for the pages probes could not reach.
//!
//! [`landscape_discover`] exhausts the subject's own domain: guessed paths, `sitemap.xml`,
//! `llms.txt`. That is deterministic, free, and hits primary sources — and it stops at the
//! edge of one company's website. Everything a competitive report needs that is *not* on
//! that website has to be asked for, and this crate is where it is asked.
//!
//! ```text
//! discovery leaves questions unanswered
//!    └─> queries::for_questions   templated, versioned, one small set per question
//!         └─> SourceProvider      SearXNG today; the trait is the seam
//!              └─> admit          scheme-checked, deduplicated, dispositioned, capped
//! ```
//!
//! # Search fills gaps; it does not lead
//!
//! `FACT_CHECKING.md` §3.3 is explicit about the order, and this crate is built so the
//! order cannot be got wrong by accident: [`queries::for_questions`] takes the questions
//! discovery **failed** to answer and emits nothing for the ones it answered. A company
//! whose probes found everything costs zero searches, and that is the common case rather
//! than an optimisation — probes reached 200 on six of six demo companies' pricing pages.
//!
//! # Nothing here is trusted, and the type system is where that is said
//!
//! Three separate refusals, because they fail for three different reasons:
//!
//! - **A search engine's rank is not evidence about a page.** Whatever order results
//!   arrive in, a page's [`Disposition`] is decided by [`admit::disposition_for`] from the
//!   host it sits on and nothing else. Being first is not being right.
//! - **A snippet is text the engine wrote, not text the page contains.** [`Hit`] carries
//!   one so a person running `landscape search` can see what came back; [`admit::Found`]
//!   has no field for it, so no snippet can reach a report without somebody adding a field
//!   on purpose. This is Run 8's finding with a shorter supply chain — *a worked example
//!   in a prompt is a source of facts* — and the same answer: cut the path, do not
//!   remember to avoid it.
//! - **A URL from a third party is a URL from a stranger.** Every hit is put through
//!   [`landscape_fetch::Target::parse`] at admission, so a `file://` or `javascript:`
//!   result is refused before anything holds it, let alone fetches it. The SSRF guard in
//!   [`landscape_fetch`] is still the thing that stops the fetch; this stops the walk to
//!   it.
//!
//! # What a hit is worth
//!
//! At most [`Disposition::Unverified`] — *"a page we could read that shows nothing
//! troubling, but which we could not fully attribute"* — unless it happens to sit on the
//! subject's own domain, in which case it is [`Disposition::Primary`] for the same reason
//! a probe is: the company said it. That asymmetry is the whole point of searching. Only a
//! primary source may set a value in a comparison table
//! ([`Disposition::may_set_a_table_value`]), so what search buys is *coverage*, not
//! *numbers* — pages to report beside the table, never cells inside it.
//!
//! # What this crate does not do yet
//!
//! It does not fetch, and it is not wired into the orchestrator. `landscape search <name>`
//! runs it by hand, which is what makes this slice demonstrable; turning a set of admitted
//! URLs into candidates the analyser reads is the next piece of work, and joining it to
//! entity resolution the one after that.
//!
//! [`Disposition`]: landscape_core::Disposition
//! [`Disposition::Unverified`]: landscape_core::Disposition::Unverified
//! [`Disposition::Primary`]: landscape_core::Disposition::Primary
//! [`Disposition::may_set_a_table_value`]: landscape_core::Disposition::may_set_a_table_value

pub mod admit;
pub mod candidates;
pub mod provider;
pub mod queries;
pub mod searx;

pub use admit::Found;
pub use provider::{Hit, SearchError, SourceProvider};
pub use queries::{Query, QUERY_SET};
pub use searx::Searx;
