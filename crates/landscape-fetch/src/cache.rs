//! Pages already fetched, so two readers of one company do not both pay for them.
//!
//! `ROADMAP.md` calls this *"the highest-leverage cache in the system"*, and the arithmetic is
//! why: an analysis reads up to eight pages per company and up to three companies, and the
//! companies are not evenly distributed. Two people asking about website analytics get the same
//! three vendors, and before this every one of those pages was fetched twice — once for each of
//! them, from somebody else's server.
//!
//! # Politeness is the point, not a side effect
//!
//! The bandwidth we save is ours; the requests we do not send are **theirs**. A cache is the
//! only thing in this crate that reduces the number of times a stranger's server is asked for
//! the same bytes, and that sits beside `robots.txt` and the per-host delay as part of the same
//! commitment rather than as an optimisation next to them.
//!
//! # A hit performs no network I/O, which is what makes it safe to serve
//!
//! [`crate::Fetcher::get`] consults this **before** the address guard, `robots.txt` and the
//! pacer, and each of those is fine to skip for exactly one reason: *nothing is sent*. The
//! guard exists to stop us reaching an address; robots exists to stop us requesting a path; the
//! pacer exists to stop us asking too often. None of them protects anything when no request
//! leaves.
//!
//! What makes that sound rather than convenient is the invariant on the way **in**: only a page
//! we were allowed to fetch is ever stored, because [`crate::Fetcher::get`] inserts after the
//! guard and after robots have passed. A path a site disallowed produces an error and is
//! therefore never in here to be served. A test asserts that a second attempt at a disallowed
//! path is refused again rather than answered from memory.
//!
//! The one thing a hit can be wrong about is a `robots.txt` that changed since the fetch, and
//! that window is bounded by [`FRESH_FOR`] rather than open.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::Page;

/// How long a fetched page may be served again.
///
/// **A starting value, not a measured one** — the same footing as
/// `landscape_core::subject::AMBIGUITY_MARGIN`, and it should move when there is evidence
/// rather than an opinion.
///
/// An hour is chosen from the two cases that matter. Two readers asking about the same market
/// within one sitting share everything, which is the demonstrable win. A pricing page edited
/// this morning is picked up this afternoon, which is the staleness nobody would forgive.
///
/// **A cached page is never presented as fresher than it is.** [`Page::fetched_at`] is stored
/// with the body and comes back unchanged, so every claim drawn from it carries the moment the
/// bytes were actually read — the report's `as_of` is the fetch, not the serve.
pub const FRESH_FOR: Duration = Duration::from_secs(60 * 60);

/// How much of a process's memory the cache may hold.
///
/// **Bytes rather than a count of pages.** A page may be up to [`crate::MAX_BYTES`], so a cap of
/// "256 pages" is a cap of anywhere between a few kilobytes and half a gigabyte — a number that
/// does not bound the thing it claims to bound. This one does.
pub const MAX_CACHED_BYTES: usize = 32 * 1024 * 1024;

/// Pages kept in memory, oldest evicted first.
#[derive(Debug, Default)]
pub struct Cache {
    by_url: HashMap<String, Entry>,
    /// Total body bytes held, kept alongside rather than recomputed: eviction happens on the
    /// insert path, and walking every entry to add up its length would make the common case pay
    /// for the rare one.
    bytes: usize,
    /// Increments on every insert. Cheaper than a clock and monotonic by construction, which is
    /// what eviction order needs — a wall clock can go backwards.
    next: u64,
}

#[derive(Debug)]
struct Entry {
    page: Page,
    stored: Instant,
    order: u64,
}

impl Cache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The page for this URL, if it was fetched recently enough to still be served.
    ///
    /// **Keyed on the URL the caller asked for**, not on the one a redirect landed on: two
    /// callers asking the same thing is the case this exists for, and they ask with the URL
    /// they have. [`Page::url`] still reports where the bytes came from.
    #[must_use]
    pub fn get(&self, url: &str) -> Option<Page> {
        self.by_url
            .get(url)
            .filter(|e| e.stored.elapsed() < FRESH_FOR)
            .map(|e| e.page.clone())
    }

    /// Keep a page, evicting the oldest until it fits.
    pub fn insert(&mut self, url: String, page: Page) {
        let size = page.body.len();
        // A page too large to keep alongside anything else is not kept at all. Without this the
        // loop below could evict the entire cache to make room for one document and then still
        // not fit it — a cache that empties itself is worse than one that declines.
        if size > MAX_CACHED_BYTES {
            return;
        }
        if let Some(old) = self.by_url.remove(&url) {
            self.bytes -= old.page.body.len();
        }
        while self.bytes + size > MAX_CACHED_BYTES {
            let Some(oldest) = self
                .by_url
                .iter()
                .min_by_key(|(_, e)| e.order)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(gone) = self.by_url.remove(&oldest) {
                self.bytes -= gone.page.body.len();
            }
        }
        self.bytes += size;
        self.next += 1;
        self.by_url.insert(
            url,
            Entry {
                page,
                stored: Instant::now(),
                order: self.next,
            },
        );
    }

    /// How many pages are held. For the diagnostic, and for tests about eviction.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_url.len()
    }

    /// Whether anything is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_url.is_empty()
    }

    /// How many body bytes are held.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Pretend every entry was stored `by` earlier than it was.
    ///
    /// **A test-only clock, and the smallest one that works.** [`FRESH_FOR`] is an hour, so the
    /// alternative to this is a test that takes an hour or a clock injected through every
    /// caller of [`crate::Fetcher::get`] for one assertion. Without it the staleness rule is
    /// unexercised, and an unexercised rule is one a mutation deleting it survives — which is
    /// exactly what the harness reported before this existed.
    #[cfg(test)]
    fn age(&mut self, by: Duration) {
        for entry in self.by_url.values_mut() {
            // `checked_sub` returns `None` only within `by` of the monotonic clock's origin,
            // which a test process is never at. Leaving the entry alone in that case would
            // make the assertion fail rather than the test panic, which is the right way round.
            if let Some(older) = entry.stored.checked_sub(by) {
                entry.stored = older;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn page(url: &str, bytes: usize) -> Page {
        Page {
            url: url.to_owned(),
            status: 200,
            body: "x".repeat(bytes),
            etag: None,
            last_modified: None,
            fetched_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn a_page_asked_for_twice_is_fetched_once() {
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/pricing".to_owned(),
            page("https://a.example/pricing", 10),
        );
        assert_eq!(
            cache.get("https://a.example/pricing").map(|p| p.body.len()),
            Some(10)
        );
        assert!(cache.get("https://a.example/other").is_none());
    }

    #[test]
    fn the_moment_the_bytes_were_read_survives_being_served_again() {
        // **The whole honesty of a cache is here.** A claim's `as_of` comes from this, so a page
        // served from memory must report when it was fetched rather than when it was handed
        // over — otherwise a report dated today is built on a page read an hour ago and says so
        // nowhere.
        let mut original = page("https://a.example/", 4);
        original.fetched_at = "2026-08-01T09:00:00Z".parse().unwrap();
        cached_is_identical(original);
    }

    fn cached_is_identical(original: Page) {
        let mut cache = Cache::new();
        cache.insert(original.url.clone(), original.clone());
        let served = cache.get(&original.url).expect("a hit");
        assert_eq!(served.fetched_at, original.fetched_at);
        assert_eq!(served.url, original.url);
        assert_eq!(served.status, original.status);
        assert_eq!(served.etag, original.etag);
    }

    #[test]
    fn the_cache_is_bounded_in_the_unit_it_claims_to_be_bounded_in() {
        // A cap of "n pages" bounds nothing when a page may be two megabytes. Filling past the
        // byte budget has to evict, and what is left has to be under it.
        let mut cache = Cache::new();
        let each = MAX_CACHED_BYTES / 4;
        for i in 0..6 {
            cache.insert(
                format!("https://a.example/{i}"),
                page("https://a.example/x", each),
            );
        }
        assert!(
            cache.bytes() <= MAX_CACHED_BYTES,
            "held {} bytes, budget is {MAX_CACHED_BYTES}",
            cache.bytes()
        );
        assert_eq!(cache.len(), 4, "the budget divides into four of these");
        assert!(
            cache.get("https://a.example/0").is_none(),
            "the oldest survived an eviction it should not have"
        );
        assert!(
            cache.get("https://a.example/5").is_some(),
            "the newest was evicted instead of the oldest"
        );
    }

    #[test]
    fn a_page_stops_being_served_once_it_is_no_longer_current() {
        // **The whole reason there is a bound at all.** Without it a price read this morning is
        // served this time next year, and the only thing saying so is an `as_of` date somebody
        // has to notice. `FRESH_FOR` is the promise; this is the test that it is kept.
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 4),
        );
        assert!(
            cache.get("https://a.example/").is_some(),
            "fresh, and missing"
        );

        cache.age(FRESH_FOR - Duration::from_secs(1));
        assert!(
            cache.get("https://a.example/").is_some(),
            "a page one second inside the window was thrown away"
        );

        cache.age(Duration::from_secs(2));
        assert!(
            cache.get("https://a.example/").is_none(),
            "a page past the window was served anyway"
        );
    }

    #[test]
    fn a_page_too_large_to_keep_does_not_empty_the_cache_trying() {
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/small".to_owned(),
            page("https://a.example/small", 10),
        );
        cache.insert(
            "https://a.example/huge".to_owned(),
            page("https://a.example/huge", MAX_CACHED_BYTES + 1),
        );
        assert!(
            cache.get("https://a.example/small").is_some(),
            "one oversized page cleared out everything else and then did not fit either"
        );
        assert!(cache.get("https://a.example/huge").is_none());
    }

    #[test]
    fn re_fetching_the_same_url_replaces_rather_than_doubles() {
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 100),
        );
        cache.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 30),
        );
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.bytes(),
            30,
            "the replaced page's bytes were counted for ever"
        );
    }
}
