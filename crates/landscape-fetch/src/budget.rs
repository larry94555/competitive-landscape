//! How many requests one analysis may send to strangers.
//!
//! `ROADMAP.md` has had *"a cap on total fetches per analysis — that belongs with the
//! orchestrator"* open since the fetch primitive was written. Everything else in this crate
//! bounds a **single** request: the size cap bounds one body, the pacer bounds how often one
//! host is asked, `robots.txt` bounds which paths. Nothing bounded the **run**.
//!
//! The implicit bound was an accident of arithmetic — eight pages a company, three companies —
//! and an accident is not a bound. A site whose `sitemap.xml` names ten thousand URLs, a
//! redirect chain per page, a `robots.txt` per host reached through search: each is a way for
//! one reader's question to become an unbounded number of requests to somebody else's server.
//! **That is a politeness failure before it is a cost failure**, which is why this lives here
//! beside the cache and the pacer rather than in a configuration file.
//!
//! # What counts
//!
//! **A request that leaves the process.** A page served from the cache costs nothing, because
//! nothing was sent — the same reason a hit skips the address guard. A `robots.txt` fetch counts,
//! because it is a request to a stranger like any other. Every redirect hop counts, because each
//! one is its own request; a chain that never ends is exactly the case this exists for.

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many requests one analysis may send.
///
/// The arithmetic it has to cover: three companies, each with a `robots.txt`, up to eight pages
/// from discovery and up to three more from search, plus a redirect or two along the way — call
/// it fifteen a company. **Sixty-four leaves room for the ordinary run and stops the pathological
/// one**, which is what a bound is for. It is a starting value on the same footing as
/// [`crate::cache::FRESH_FOR`], and it should move when there is evidence rather than an opinion.
pub const MAX_FETCHES_PER_ANALYSIS: usize = 64;

/// What is left of one analysis's allowance.
///
/// **Created per analysis, never per process.** The `Fetcher` outlives an analysis and its caches
/// are meant to; this is the opposite — it exists to bound *one reader's question*, and sharing
/// it across readers would let a quiet afternoon pay for a busy one.
#[derive(Debug)]
pub struct Budget {
    limit: usize,
    spent: AtomicUsize,
}

impl Budget {
    /// An allowance of `limit` requests.
    #[must_use]
    pub fn of(limit: usize) -> Self {
        Self {
            limit,
            spent: AtomicUsize::new(0),
        }
    }

    /// The usual allowance. See [`MAX_FETCHES_PER_ANALYSIS`].
    #[must_use]
    pub fn for_one_analysis() -> Self {
        Self::of(MAX_FETCHES_PER_ANALYSIS)
    }

    /// Take one request from the allowance, if there is one.
    ///
    /// Returns whether the caller may send. **Counted before the request rather than after**, so
    /// a request in flight has already been paid for and two callers cannot both spend the last
    /// one — `fetch_add` on the way in makes that true without a lock.
    pub fn spend(&self) -> bool {
        let taken = self.spent.fetch_add(1, Ordering::Relaxed);
        if taken < self.limit {
            return true;
        }
        // Give it back, so a run that is refused a hundred times still reports what it spent
        // rather than a number that grows for ever.
        self.spent.fetch_sub(1, Ordering::Relaxed);
        false
    }

    /// How many requests have been sent, for the diagnostic and for the tests.
    #[must_use]
    pub fn spent(&self) -> usize {
        self.spent.load(Ordering::Relaxed)
    }

    /// How many are left.
    #[must_use]
    pub fn left(&self) -> usize {
        self.limit.saturating_sub(self.spent())
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::for_one_analysis()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn an_allowance_runs_out_and_stays_out() {
        let budget = Budget::of(3);
        assert_eq!(budget.left(), 3);
        for i in 1..=3 {
            assert!(budget.spend(), "request {i} should have been allowed");
            assert_eq!(budget.spent(), i);
        }
        assert!(!budget.spend(), "a fourth request was allowed");
        assert!(!budget.spend(), "and a fifth");
        assert_eq!(
            budget.spent(),
            3,
            "refusals were counted as requests, so the diagnostic lies about what was sent"
        );
        assert_eq!(budget.left(), 0);
    }

    #[test]
    fn the_usual_allowance_covers_an_ordinary_run() {
        // Three companies, `robots.txt` each, eight discovered pages, three searched, and a
        // redirect or two. A bound that the ordinary case trips is a bug wearing a limit's hat.
        let ordinary = 3 * (1 + 8 + 3 + 2);
        assert!(
            MAX_FETCHES_PER_ANALYSIS >= ordinary,
            "the usual allowance ({MAX_FETCHES_PER_ANALYSIS}) is under an ordinary run ({ordinary})"
        );
    }

    #[test]
    fn two_callers_cannot_both_spend_the_last_request() {
        // The counter is taken on the way in rather than checked and then incremented, so there
        // is no window between the two in which both callers see one left.
        let budget = Budget::of(1);
        assert!(budget.spend());
        assert!(!budget.spend());
        assert_eq!(budget.spent(), 1);
    }
}
