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

use chrono::{DateTime, Utc};

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
/// does not bound the thing it claims to bound.
///
/// Review pointed the same argument back the other way, and it landed: the first version counted
/// only [`Page::body`], so a hundred thousand **empty** responses sat in the map reporting zero
/// bytes held and nothing was ever evicted. A byte budget that ignores keys and headers does not
/// bound entries any more than an entry budget bounds bytes. [`cost`] counts the whole of what
/// is retained, and [`MAX_ENTRIES`] bounds the count as well — **both**, because each alone has
/// a hole shaped exactly like the other.
pub const MAX_CACHED_BYTES: usize = 32 * 1024 * 1024;

/// How many pages the cache may hold, whatever they weigh.
///
/// The second half of the bound. With [`OVERHEAD`] counted this is very unlikely to be the limit
/// that binds — which is the point of having it: a number that only matters when the other one
/// is wrong.
pub const MAX_ENTRIES: usize = 4_096;

/// What one entry costs beyond the bytes of the page.
///
/// The key, a duplicated [`Page::url`], two optional headers, the entry itself and the map's
/// slot. **Deliberately approximate** — an exact figure would need the allocator's cooperation
/// and does not need to be exact to do its job, which is to make a page with no body still cost
/// something. That is the difference between a bound and a number.
const OVERHEAD: usize = 256;

/// What keeping this page actually costs.
fn cost(url: &str, page: &Page) -> usize {
    OVERHEAD
        + url.len()
        + page.url.len()
        + page.body.len()
        + page.etag.as_ref().map_or(0, String::len)
        + page.last_modified.as_ref().map_or(0, String::len)
}

/// What the origin said we may do with a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storable {
    /// Not at all. `no-store`, `no-cache` or `private`.
    No,
    /// For this long — the shorter of what the origin allows and [`FRESH_FOR`].
    For(Duration),
}

/// Read the origin's caching policy out of its own headers.
///
/// **Review found this missing, and it is the most pointed thing in the change.** The module
/// argues that the cache belongs beside `robots.txt` as part of one commitment about how a
/// stranger's server is treated — and then ignored the single header by which that stranger
/// states the commitment. A `Cache-Control: no-store` was retained anyway and handed to a
/// second reader; a `max-age=30` was served for an hour.
///
/// | The origin says | What happens |
/// |---|---|
/// | `no-store` | not kept. It said not to. |
/// | `no-cache` | not kept. Reuse requires revalidating, and conditional GET is a later row. |
/// | `private` | not kept. **This is a shared cache** — one process serving every reader — and that is exactly what `private` forbids. |
/// | `s-maxage=N` | kept for `N`, capped at [`FRESH_FOR`]. Shared caches are told this one first. |
/// | `max-age=N` | kept for `N`, capped. |
/// | `Expires: <past>` | not kept. |
/// | `Expires: <future>` | kept until then, capped. |
/// | nothing | kept for [`FRESH_FOR`], which is our own restraint rather than a claim about theirs. |
///
/// Directives are matched on whole tokens, so `no-store` is found in `public, no-store` and not
/// inside a hypothetical `x-no-store`. Anything unparseable is treated as **not cacheable**: a
/// header we cannot read is not permission, and the cost of being wrong that way is one extra
/// request rather than one ignored instruction.
#[must_use]
pub fn storable(
    cache_control: Option<&str>,
    expires: Option<&str>,
    now: DateTime<Utc>,
) -> Storable {
    let mut allowed = FRESH_FOR;

    if let Some(raw) = cache_control {
        let lowered = raw.to_lowercase();
        let directives: Vec<&str> = lowered.split(',').map(str::trim).collect();
        if directives
            .iter()
            .any(|d| matches!(*d, "no-store" | "no-cache" | "private"))
        {
            return Storable::No;
        }
        // `s-maxage` first: it is the one addressed to shared caches, and this is one.
        for name in ["s-maxage", "max-age"] {
            if let Some(value) = directives
                .iter()
                .find_map(|d| d.strip_prefix(name)?.strip_prefix('='))
            {
                let Ok(seconds) = value.trim().parse::<u64>() else {
                    // A `max-age` we cannot read is not a licence to keep it for an hour.
                    return Storable::No;
                };
                allowed = allowed.min(Duration::from_secs(seconds));
                break;
            }
        }
    }

    if let Some(raw) = expires {
        let Ok(at) = DateTime::parse_from_rfc2822(raw.trim()) else {
            // RFC 9111 says an unparseable `Expires` means already expired. It is also the
            // conservative reading, which is the tie-breaker whenever those two agree.
            return Storable::No;
        };
        let Ok(remaining) = (at.with_timezone(&Utc) - now).to_std() else {
            return Storable::No;
        };
        allowed = allowed.min(remaining);
    }

    if allowed.is_zero() {
        Storable::No
    } else {
        Storable::For(allowed)
    }
}

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
    /// What this entry costs to keep, remembered so eviction subtracts what insertion added.
    /// Recomputing it would go wrong the moment [`cost`] changed and an old entry was removed.
    cost: usize,
    /// How long this one may be served — the origin's number, not ours, when it gave one.
    fresh_for: Duration,
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
            .filter(|e| e.stored.elapsed() < e.fresh_for)
            .map(|e| e.page.clone())
    }

    /// Keep a page, evicting the oldest until it fits.
    pub fn insert(&mut self, url: String, page: Page) {
        self.insert_for(url, page, FRESH_FOR);
    }

    /// Keep a page for as long as the origin's own headers allow, or not at all.
    ///
    /// Returns whether it was kept.
    ///
    /// **The decision lives here rather than in [`crate::Fetcher::get`], and that placement is
    /// the point.** The obvious shape is for the fetcher to read [`storable`] and branch on it,
    /// and the first version did — but no test in this repository can reach that branch, because
    /// a test server binds loopback and the address guard refuses loopback absolutely. A rule
    /// written where nothing can exercise it is a rule a mutation deleting it survives, and the
    /// harness said exactly that. Here it is one function call away from an assertion.
    ///
    /// The fetcher's remaining share is reading two header strings off the response — the same
    /// untestable line as `etag` and `last-modified`, with no branch in it.
    pub fn insert_allowed(
        &mut self,
        url: String,
        page: Page,
        cache_control: Option<&str>,
        expires: Option<&str>,
        now: DateTime<Utc>,
    ) -> bool {
        match storable(cache_control, expires, now) {
            Storable::No => false,
            Storable::For(fresh_for) => {
                self.insert_for(url, page, fresh_for);
                true
            }
        }
    }

    /// Keep a page for a stated time, evicting the oldest until it fits.
    ///
    /// The duration comes from the origin — see [`storable`] — capped at [`FRESH_FOR`]. A
    /// publisher saying *thirty seconds* is not overruled by our hour.
    pub fn insert_for(&mut self, url: String, page: Page, fresh_for: Duration) {
        let size = cost(&url, &page);
        // A page too large to keep alongside anything else is not kept at all. Without this the
        // loop below could evict the entire cache to make room for one document and then still
        // not fit it — a cache that empties itself is worse than one that declines.
        if size > MAX_CACHED_BYTES {
            return;
        }
        if let Some(old) = self.by_url.remove(&url) {
            self.bytes -= old.cost;
        }
        while self.bytes + size > MAX_CACHED_BYTES || self.by_url.len() >= MAX_ENTRIES {
            let Some(oldest) = self
                .by_url
                .iter()
                .min_by_key(|(_, e)| e.order)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(gone) = self.by_url.remove(&oldest) {
                self.bytes -= gone.cost;
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
                cost: size,
                fresh_for,
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

    /// How many bytes are held, counting keys, headers and per-entry overhead as well as
    /// bodies. See [`cost`].
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
        let each = MAX_CACHED_BYTES / 4 - OVERHEAD - 64;
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

    fn at(raw: &str) -> DateTime<Utc> {
        raw.parse().expect("a fixed instant")
    }

    #[test]
    fn a_publisher_saying_do_not_store_this_is_obeyed() {
        // The module argues the cache belongs beside `robots.txt` as one commitment about how a
        // stranger's server is treated. Ignoring the header they state it in would make that
        // sentence decorative.
        let now = at("2026-08-09T00:00:00Z");
        for said in [
            "no-store",
            "public, no-store",
            "No-Store",
            "private",
            "no-cache",
        ] {
            assert_eq!(
                storable(Some(said), None, now),
                Storable::No,
                "{said:?} was kept anyway"
            );
        }
    }

    #[test]
    fn a_shorter_freshness_than_ours_wins_and_a_longer_one_does_not() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(
            storable(Some("max-age=30"), None, now),
            Storable::For(Duration::from_secs(30)),
            "half a minute was stretched to an hour"
        );
        assert_eq!(
            storable(Some("max-age=86400"), None, now),
            Storable::For(FRESH_FOR),
            "a day was taken as permission to hold it for a day"
        );
        // `s-maxage` is addressed to shared caches, and this is one, so it is read first.
        assert_eq!(
            storable(Some("max-age=600, s-maxage=60"), None, now),
            Storable::For(Duration::from_secs(60))
        );
    }

    #[test]
    fn an_expiry_already_past_is_not_kept() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(
            storable(None, Some("Fri, 08 Aug 2026 00:00:00 GMT"), now),
            Storable::No
        );
        assert_eq!(
            storable(None, Some("Sun, 09 Aug 2026 00:00:30 GMT"), now),
            Storable::For(Duration::from_secs(30))
        );
    }

    #[test]
    fn a_header_we_cannot_read_is_not_permission() {
        // The cost of refusing to cache something we misread is one extra request. The cost of
        // keeping something we misread is ignoring an instruction somebody wrote down.
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(storable(Some("max-age=soon"), None, now), Storable::No);
        assert_eq!(storable(None, Some("whenever"), now), Storable::No);
    }

    #[test]
    fn silence_is_our_own_restraint_rather_than_their_permission() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(storable(None, None, now), Storable::For(FRESH_FOR));
    }

    #[test]
    fn a_page_kept_for_the_origins_time_stops_being_served_at_that_time() {
        // The whole point of reading the header: our hour must not outlive their thirty seconds.
        let mut cache = Cache::new();
        cache.insert_for(
            "https://a.example/".to_owned(),
            page("https://a.example/", 4),
            Duration::from_secs(30),
        );
        cache.age(Duration::from_secs(31));
        assert!(
            cache.get("https://a.example/").is_none(),
            "a page the origin allowed for thirty seconds was served after thirty-one"
        );
    }

    #[test]
    fn a_page_the_origin_said_not_to_store_is_not_stored() {
        // **Reading the header and acting on it are one call apart on purpose.** The first shape
        // of this put the branch in `Fetcher::get`, where the address guard means no test can
        // reach it — and the mutation that read the headers and then ignored them survived.
        let now = at("2026-08-09T00:00:00Z");
        let mut cache = Cache::new();

        let held = cache.insert_allowed(
            "https://a.example/".to_owned(),
            page("https://a.example/", 4),
            Some("no-store"),
            None,
            now,
        );
        assert!(!held, "the origin said not to store it, and it was stored");
        assert!(
            cache.get("https://a.example/").is_none(),
            "a page the origin refused was handed to the next reader"
        );

        // And when they do allow it, it is their number that is kept, not ours.
        let held = cache.insert_allowed(
            "https://b.example/".to_owned(),
            page("https://b.example/", 4),
            Some("max-age=30"),
            None,
            now,
        );
        assert!(held, "a page we were allowed to keep was dropped");
        cache.age(Duration::from_secs(31));
        assert!(
            cache.get("https://b.example/").is_none(),
            "held for our hour rather than for their thirty seconds"
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
    fn an_empty_body_still_costs_something_to_keep() {
        // **Review found this, and it is the argument in this file turned back on itself.** The
        // budget counted only the body, so a hundred thousand empty responses reported zero
        // bytes held and nothing was ever evicted. A byte budget that ignores keys does not
        // bound entries any more than an entry budget bounds bytes.
        //
        // Asserted against `OVERHEAD` rather than against zero: a bodyless entry still holds its
        // key, so `bytes() > 0` passes while counting nothing but strings — which is the hole
        // itself, spelt slightly differently.
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 0),
        );
        assert!(
            cache.bytes() >= OVERHEAD,
            "an entry with no body was costed at {} bytes, less than the entry itself",
            cache.bytes()
        );

        let mut cache = Cache::new();
        for i in 0..(MAX_ENTRIES * 2) {
            cache.insert(
                format!("https://a.example/{i}"),
                page("https://a.example/x", 0),
            );
        }
        assert!(
            cache.len() <= MAX_ENTRIES,
            "held {} entries with no body between them",
            cache.len()
        );
        assert!(
            cache.bytes() > 0,
            "entries were held and reported as weighing nothing"
        );
        assert!(cache.bytes() <= MAX_CACHED_BYTES);
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

        // **The property, not a figure.** What must hold is that replacing an entry costs what
        // the replacement costs — the same as having inserted it alone. A literal byte count
        // here would pin `OVERHEAD` into a test that is about accounting rather than about it.
        let mut once = Cache::new();
        once.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 30),
        );

        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.bytes(),
            once.bytes(),
            "the replaced page's bytes were counted for ever"
        );
    }
}
