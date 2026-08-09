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
/// **A ceiling, not a policy.** It applies when the origin says nothing, the origin's own number
/// wins whenever it is shorter, and time the response already spent in caches between the origin
/// and us comes off it — see [`storable`].
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
    /// For this long — the shorter of what the origin allows and [`FRESH_FOR`], less whatever
    /// of it the response had already spent in caches before reaching us.
    For(Duration),
}

/// The response headers that decide whether, and for how long, a page may be kept.
///
/// A struct rather than four positional `Option<&str>`, because the call site is where a
/// `date` and an `age` get swapped and nothing complains.
#[derive(Debug, Default, Clone, Copy)]
pub struct Freshness<'a> {
    pub cache_control: Option<&'a str>,
    pub expires: Option<&'a str>,
    /// When the origin says it generated the response.
    pub date: Option<&'a str>,
    /// How long the response has already spent in caches between the origin and us.
    pub age: Option<&'a str>,
}

/// How stale the response already was when it arrived.
///
/// **Review's finding, and it is the difference between a lifetime and a deadline.** A CDN
/// answering `Cache-Control: max-age=3600` with `Age: 3590` is telling us the response has ten
/// seconds of freshness left, not an hour: the hour is measured from when the *origin* generated
/// it, not from when it reached us. Restarting the clock on arrival lets a chain of caches keep
/// a response fresh for ever, one hop at a time.
///
/// RFC 9111 §4.2.3 in its two useful terms — the apparent age from `Date`, and the stated `Age`,
/// whichever is larger. The round-trip correction is left out: it can only make the age larger,
/// so omitting it errs toward serving, and the two terms below are what a real chain reports.
///
/// A clock skewed so the origin's `Date` is in the future gives no apparent age rather than a
/// negative one.
fn already_aged(headers: Freshness<'_>, now: DateTime<Utc>) -> Option<Duration> {
    let mut age = Duration::ZERO;

    if let Some(raw) = headers.date {
        let at = DateTime::parse_from_rfc2822(raw.trim()).ok()?;
        if let Ok(apparent) = (now - at.with_timezone(&Utc)).to_std() {
            age = apparent;
        }
    }
    if let Some(raw) = headers.age {
        age = age.max(Duration::from_secs(raw.trim().parse::<u64>().ok()?));
    }
    Some(age)
}

/// The statuses HTTP defines as cacheable without the origin saying so.
///
/// RFC 9110 §15.1, minus the redirects: a 3xx never reaches the store here, because
/// [`crate::Fetcher::get`] follows it and only the response at the end of the chain is kept.
///
/// **Everything else needs the origin's permission in writing.** Review found the hole this
/// closes, and it is one this crate had already argued against itself: `robots::Rules::from_status`
/// treats a 429 or a 5xx as *"the site is unwell, leave it alone"* — and the cache then wrote that
/// bad afternoon down and replayed it to every later reader for an hour, without ever asking the
/// origin whether it had recovered. A cached failure is worse than a slow one: it is a gap in a
/// report that no longer has a live cause.
pub const CACHEABLE_BY_DEFAULT: [u16; 9] = [200, 203, 204, 206, 404, 405, 410, 414, 501];

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
/// | `s-maxage=N` | kept for `N` **less the age it arrived with**, capped at [`FRESH_FOR`]. Shared caches are told this one first. |
/// | `max-age=N` | the same, one directive down. |
/// | `Expires: <past>` | not kept. |
/// | `Expires: <future>` | kept until then, capped. Already relative to now, so the age is not subtracted twice. |
/// | nothing | kept for [`FRESH_FOR`] less the age it arrived with — **but only if the status is one HTTP says may be kept on our own initiative**. See [`CACHEABLE_BY_DEFAULT`]. |
///
/// Directives are matched on whole tokens, so `no-store` is found in `public, no-store` and not
/// inside a hypothetical `x-no-store`. Anything unparseable is treated as **not cacheable**: a
/// header we cannot read is not permission, and the cost of being wrong that way is one extra
/// request rather than one ignored instruction.
///
/// **[`FRESH_FOR`] is reduced by the age too**, not only the origin's number. It is a ceiling on
/// how stale served bytes may be, and time a response spent in somebody else's cache is
/// staleness that has already happened.
///
/// # The status is part of the policy
///
/// A `500` or a `429` with no headers used to be kept for the hour like anything else. Storing
/// without the origin's say-so is only permitted for the statuses in [`CACHEABLE_BY_DEFAULT`];
/// for anything else the origin has to state a freshness explicitly, and then its number is
/// obeyed like any other.
///
/// **A bare `public` is not taken as that permission**, though RFC 9111 §3 would allow it. This
/// is stricter on purpose: `public` says *may be stored*, not *is fresh for*, and inventing an
/// hour of freshness for a `503` on the strength of it is the reading that costs an origin its
/// recovery. The price of the stricter reading is one extra request.
#[must_use]
pub fn storable(status: u16, headers: Freshness<'_>, now: DateTime<Utc>) -> Storable {
    let Some(aged) = already_aged(headers, now) else {
        return Storable::No;
    };
    let mut allowed = FRESH_FOR.saturating_sub(aged);
    // Whether the origin stated a freshness of its own, which is what a status HTTP does not
    // consider cacheable needs before anything is kept.
    let mut stated = false;

    if let Some(raw) = headers.cache_control {
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
                // **Less the age, not from now.** `max-age` is measured from the origin's
                // `Date`, so a response that spent most of it in a CDN has most of it spent.
                allowed = allowed.min(Duration::from_secs(seconds).saturating_sub(aged));
                stated = true;
                break;
            }
        }
    }

    if let Some(raw) = headers.expires {
        let Ok(at) = DateTime::parse_from_rfc2822(raw.trim()) else {
            // RFC 9111 says an unparseable `Expires` means already expired. It is also the
            // conservative reading, which is the tie-breaker whenever those two agree.
            return Storable::No;
        };
        let Ok(remaining) = (at.with_timezone(&Utc) - now).to_std() else {
            return Storable::No;
        };
        allowed = allowed.min(remaining);
        stated = true;
    }

    // **Heuristic freshness is for the statuses HTTP says it is for.** A transient failure kept
    // on our own initiative is a gap in every later report until the hour is up, and the origin
    // is never asked whether it recovered.
    if !stated && !CACHEABLE_BY_DEFAULT.contains(&status) {
        return Storable::No;
    }

    if allowed.is_zero() {
        Storable::No
    } else {
        Storable::For(allowed)
    }
}

/// A page we still hold and the proof we can ask whether it has changed.
///
/// **The body travels with the validators on purpose.** Revalidation needs the old bytes to hand
/// back on a `304`, and looking them up again after the round trip would open a window in which
/// another thread's insert had evicted them — a branch that could then only be tested by
/// arranging a race. Carrying the page costs one clone on the revalidation path and closes the
/// case rather than handling it.
#[derive(Debug, Clone)]
pub struct Stale {
    pub page: Page,
    /// `ETag`, the strong one, first.
    pub etag: Option<String>,
    /// `Last-Modified`, for origins that do not send an `ETag`.
    pub last_modified: Option<String>,
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

    /// A page we hold that is past its freshness, with something to revalidate it by.
    ///
    /// **This is what makes an expired entry worth more than an empty one.** Without it a page
    /// whose hour is up costs a full download to learn that nothing about it changed; with it,
    /// the origin is asked in a request that carries no body in either direction when the answer
    /// is *no*. `Page` has carried `etag` and `last_modified` since the first fetch in this
    /// crate and nothing has ever sent them back — see `ROADMAP.md`.
    ///
    /// `None` when the entry is missing, still fresh (in which case [`Self::get`] already served
    /// it), or carries no validator — an origin that offered neither has given us no way to ask
    /// a cheap question, and asking an expensive one is just a fetch.
    #[must_use]
    pub fn stale(&self, url: &str) -> Option<Stale> {
        let entry = self.by_url.get(url)?;
        if entry.stored.elapsed() < entry.fresh_for {
            return None;
        }
        if entry.page.etag.is_none() && entry.page.last_modified.is_none() {
            return None;
        }
        Some(Stale {
            page: entry.page.clone(),
            etag: entry.page.etag.clone(),
            last_modified: entry.page.last_modified.clone(),
        })
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
        headers: Freshness<'_>,
        now: DateTime<Utc>,
    ) -> bool {
        // **The status comes from the page rather than from a parameter.** They cannot disagree
        // that way, and a caller cannot pass the status of a different response.
        match storable(page.status, headers, now) {
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

    /// What an origin said, when all it said was one `Cache-Control`.
    fn said(cache_control: &str) -> Freshness<'_> {
        Freshness {
            cache_control: Some(cache_control),
            ..Freshness::default()
        }
    }

    /// What an origin said, when all it said was one `Expires`.
    fn expiring(expires: &str) -> Freshness<'_> {
        Freshness {
            expires: Some(expires),
            ..Freshness::default()
        }
    }

    #[test]
    fn a_publisher_saying_do_not_store_this_is_obeyed() {
        // The module argues the cache belongs beside `robots.txt` as one commitment about how a
        // stranger's server is treated. Ignoring the header they state it in would make that
        // sentence decorative.
        let now = at("2026-08-09T00:00:00Z");
        for directive in [
            "no-store",
            "public, no-store",
            "No-Store",
            "private",
            "no-cache",
        ] {
            assert_eq!(
                storable(200, said(directive), now),
                Storable::No,
                "{directive:?} was kept anyway"
            );
        }
    }

    #[test]
    fn a_shorter_freshness_than_ours_wins_and_a_longer_one_does_not() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(
            storable(200, said("max-age=30"), now),
            Storable::For(Duration::from_secs(30)),
            "half a minute was stretched to an hour"
        );
        assert_eq!(
            storable(200, said("max-age=86400"), now),
            Storable::For(FRESH_FOR),
            "a day was taken as permission to hold it for a day"
        );
        // `s-maxage` is addressed to shared caches, and this is one, so it is read first.
        assert_eq!(
            storable(200, said("max-age=600, s-maxage=60"), now),
            Storable::For(Duration::from_secs(60))
        );
    }

    #[test]
    fn an_expiry_already_past_is_not_kept() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(
            storable(200, expiring("Fri, 08 Aug 2026 00:00:00 GMT"), now),
            Storable::No
        );
        assert_eq!(
            storable(200, expiring("Sun, 09 Aug 2026 00:00:30 GMT"), now),
            Storable::For(Duration::from_secs(30))
        );
    }

    #[test]
    fn a_header_we_cannot_read_is_not_permission() {
        // The cost of refusing to cache something we misread is one extra request. The cost of
        // keeping something we misread is ignoring an instruction somebody wrote down.
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(storable(200, said("max-age=soon"), now), Storable::No);
        assert_eq!(storable(200, expiring("whenever"), now), Storable::No);
        // And the two that say how much of the freshness is already gone.
        let unreadable_age = Freshness {
            cache_control: Some("max-age=600"),
            age: Some("ages"),
            ..Freshness::default()
        };
        assert_eq!(storable(200, unreadable_age, now), Storable::No);
        let unreadable_date = Freshness {
            cache_control: Some("max-age=600"),
            date: Some("last tuesday"),
            ..Freshness::default()
        };
        assert_eq!(storable(200, unreadable_date, now), Storable::No);
    }

    #[test]
    fn silence_is_our_own_restraint_rather_than_their_permission() {
        let now = at("2026-08-09T00:00:00Z");
        assert_eq!(
            storable(200, Freshness::default(), now),
            Storable::For(FRESH_FOR)
        );
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
    fn freshness_already_spent_in_somebody_elses_cache_is_not_handed_back() {
        // **Review's finding, and it is the difference between a lifetime and a deadline.** A
        // CDN answering `max-age=3600` with `Age: 3590` has ten seconds of freshness left, not
        // an hour: the hour runs from the origin's `Date`, not from our doorstep. Restarting it
        // on arrival lets a chain of caches keep one response fresh for ever, a hop at a time.
        let now = at("2026-08-09T00:00:00Z");

        let nearly_stale = Freshness {
            cache_control: Some("max-age=3600"),
            age: Some("3590"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, nearly_stale, now),
            Storable::For(Duration::from_secs(10)),
            "an hour was restarted on arrival"
        );

        // The same thing said with `Date` instead, which is the header every response carries.
        let old_date = Freshness {
            cache_control: Some("max-age=3600"),
            date: Some("Sat, 08 Aug 2026 23:00:10 GMT"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, old_date, now),
            Storable::For(Duration::from_secs(10)),
            "the apparent age from `Date` was ignored"
        );

        // Already past its deadline: not reusable at all, rather than reusable for a moment.
        let stale = Freshness {
            cache_control: Some("max-age=60"),
            age: Some("60"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, stale, now),
            Storable::No,
            "a stale response was kept"
        );

        // The larger of the two wins — a proxy that states `Age` is not overruled by a `Date`
        // that makes the response look younger.
        let both = Freshness {
            cache_control: Some("max-age=3600"),
            date: Some("Sat, 08 Aug 2026 23:59:59 GMT"),
            age: Some("3000"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, both, now),
            Storable::For(Duration::from_secs(600))
        );

        // And the other way round, which is the half that pins "whichever is larger": a `Date`
        // an hour old is not overruled by a proxy reporting ten seconds.
        let honest_date = Freshness {
            cache_control: Some("max-age=3600"),
            date: Some("Sat, 08 Aug 2026 23:00:00 GMT"),
            age: Some("10"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, honest_date, now),
            Storable::No,
            "an hour-old response was kept because a proxy under-reported its age"
        );

        // Our own hour is a ceiling on staleness too, not only theirs.
        let old_and_silent = Freshness {
            age: Some("3599"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, old_and_silent, now),
            Storable::For(Duration::from_secs(1))
        );

        // A clock skewed the other way gives no age rather than a negative one.
        let from_the_future = Freshness {
            cache_control: Some("max-age=60"),
            date: Some("Sun, 09 Aug 2026 01:00:00 GMT"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, from_the_future, now),
            Storable::For(Duration::from_secs(60))
        );

        // `Expires` is already measured against now, so the age must not come off it twice.
        let expiring_soon = Freshness {
            expires: Some("Sun, 09 Aug 2026 00:00:30 GMT"),
            age: Some("20"),
            ..Freshness::default()
        };
        assert_eq!(
            storable(200, expiring_soon, now),
            Storable::For(Duration::from_secs(30)),
            "the age was subtracted from a deadline that already accounts for it"
        );
    }

    #[test]
    fn a_bad_afternoon_is_not_written_down_and_replayed() {
        // **Review found this, and this crate had already argued against it.**
        // `robots::Rules::from_status` treats a 429 or a 5xx as *"the site is unwell, leave it
        // alone"* — and the cache then kept that failure for an hour and handed it to every later
        // reader without ever asking whether the origin had recovered. A cached failure is worse
        // than a slow one: it is a gap in a report that no longer has a live cause.
        let now = at("2026-08-09T00:00:00Z");
        for status in [429, 500, 502, 503, 504, 400, 403] {
            assert_eq!(
                storable(status, Freshness::default(), now),
                Storable::No,
                "a headerless {status} was kept on our own initiative"
            );
        }

        // The statuses HTTP does say may be kept without being asked still are.
        for status in CACHEABLE_BY_DEFAULT {
            assert_eq!(
                storable(status, Freshness::default(), now),
                Storable::For(FRESH_FOR),
                "a {status} that HTTP says is cacheable was refused"
            );
        }

        // And an origin that states a freshness is obeyed whatever the status — its number, not
        // ours, and not an hour invented on its behalf.
        assert_eq!(
            storable(503, said("max-age=30"), now),
            Storable::For(Duration::from_secs(30)),
            "an origin asking us to hold a 503 for thirty seconds was ignored"
        );
        assert_eq!(
            storable(503, expiring("Sun, 09 Aug 2026 00:00:30 GMT"), now),
            Storable::For(Duration::from_secs(30))
        );

        // A bare `public` is *not* that permission. RFC 9111 §3 would allow storing on it; this
        // is stricter on purpose, because `public` says "may be stored", not "is fresh for", and
        // inventing an hour of freshness for a 503 on that basis costs an origin its recovery.
        assert_eq!(storable(503, said("public"), now), Storable::No);
    }

    #[test]
    fn a_failure_that_reached_the_page_is_still_not_held() {
        // The whole way through, with the status coming off the `Page` rather than a parameter —
        // they cannot disagree that way.
        let now = at("2026-08-09T00:00:00Z");
        let mut cache = Cache::new();
        let mut down = page("https://a.example/", 4);
        down.status = 503;

        let held = cache.insert_allowed(
            "https://a.example/".to_owned(),
            down,
            Freshness::default(),
            now,
        );
        assert!(!held, "a 503 was written down");
        assert!(cache.is_empty());
    }

    #[test]
    fn a_page_past_its_hour_is_kept_to_ask_about_rather_than_thrown_away() {
        // **What makes an expired entry worth more than an empty one.** Without this, a page
        // whose hour is up costs a full download to learn that nothing about it changed. With
        // it, the question is asked in a request whose answer carries no body when the answer
        // is *no* — which is bandwidth on somebody else's server, not ours.
        let mut cache = Cache::new();
        let mut page = page("https://a.example/pricing", 400);
        page.etag = Some("\"v1\"".to_owned());
        cache.insert("https://a.example/pricing".to_owned(), page);

        assert!(
            cache.stale("https://a.example/pricing").is_none(),
            "a page still inside its hour is served, not revalidated"
        );

        cache.age(FRESH_FOR + Duration::from_secs(1));
        let asking = cache
            .stale("https://a.example/pricing")
            .expect("an expired page with an ETag is worth asking about");
        assert_eq!(asking.etag.as_deref(), Some("\"v1\""));
        assert_eq!(
            asking.page.body.len(),
            400,
            "the body travels with the validators, so a 304 needs nothing from the cache"
        );
    }

    #[test]
    fn a_page_the_origin_gave_us_no_way_to_ask_about_is_not_asked_about() {
        // An origin that sent neither `ETag` nor `Last-Modified` has given us no cheap question
        // to ask. Asking an expensive one is just a fetch, and pretending otherwise would put a
        // conditional header on a request that cannot be answered conditionally.
        let mut cache = Cache::new();
        cache.insert(
            "https://a.example/".to_owned(),
            page("https://a.example/", 4),
        );
        cache.age(FRESH_FOR + Duration::from_secs(1));
        assert!(cache.stale("https://a.example/").is_none());

        // And `Last-Modified` alone is enough, for the origins that offer only that.
        let mut only_dated = page("https://b.example/", 4);
        only_dated.last_modified = Some("Sat, 08 Aug 2026 23:00:00 GMT".to_owned());
        cache.insert("https://b.example/".to_owned(), only_dated);
        cache.age(FRESH_FOR + Duration::from_secs(1));
        let asking = cache.stale("https://b.example/").expect("dated is askable");
        assert!(asking.etag.is_none());
        assert!(asking.last_modified.is_some());
    }

    #[test]
    fn nothing_we_never_held_is_worth_asking_about() {
        let cache = Cache::new();
        assert!(cache.stale("https://a.example/never-seen").is_none());
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
            said("no-store"),
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
            said("max-age=30"),
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
