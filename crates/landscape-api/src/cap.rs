//! A daily cap on anonymous runs.
//!
//! `ROADMAP.md` §2·D item D6, and the last thing on the deployment track.
//!
//! **One request starts minutes of work on the only scarce resource this machine has.** Four
//! ARM cores, one model, no accounts and no quota: a public URL is an invitation to spend all
//! of it, and [`DEPLOY.md`] has been opening with that warning rather than a solution.
//! `PRODUCT_SPEC.md` §2.1 fixes the number — **two analyses a day, on a hashed address**.
//!
//! # Which address, and why the rightmost one
//!
//! Behind a reverse proxy the peer is the proxy, so the client is in `X-Forwarded-For`. That
//! header is **appended to**, not replaced: a client that sends `X-Forwarded-For: 1.2.3.4` gets
//! `1.2.3.4, <what the proxy saw>`. So the leftmost entry is whatever the client felt like
//! claiming and the **rightmost is what our own proxy observed** — taking the first is the
//! classic way a cap like this is bypassed on the first afternoon it is public.
//!
//! This assumes the application is only reachable *through* the proxy, which is what
//! [`DEPLOY.md`] arranges: `BIND_ADDR` is loopback and nothing opens 8787. Anyone who can reach
//! the port directly can forge any header they like, and that is true of every scheme of this
//! kind rather than of this one.
//!
//! # No header means no proxy, and no cap
//!
//! A request with no `X-Forwarded-For` did not come through the proxy, which on this design
//! means it came from the machine itself — a developer running `landscape dev`, or somebody on
//! the box through a tunnel. **Those are not capped**, because a two-a-day limit on a laptop
//! would make the application unusable to the person building it, and the threat this exists
//! for arrives over the internet.
//!
//! # What is stored
//!
//! A keyed hash of the address and a count, in memory, for one day.
//!
//! The key comes from [`RandomState`] — the operating system's randomness, fresh per process —
//! so the table cannot be read back to an address, and a restart makes yesterday's hashes
//! meaningless. It is a keyed hash rather than a cryptographic commitment, which is the right
//! strength for a number that never leaves memory and is never written down.
//!
//! **A restart resets the counts.** That is the honest cost of not putting this in the
//! database, and it is proportionate: the cap exists to stop a URL being drained by strangers,
//! not to be an accounting record. If it ever needs to survive a restart, it needs a table.
//!
//! [`DEPLOY.md`]: https://github.com/larry94555/competitive-landscape/blob/main/docs/DEPLOY.md
//! [`RandomState`]: std::collections::hash_map::RandomState

use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Mutex;

use chrono::NaiveDate;

/// Analyses one anonymous address may start in a day, unless configured otherwise.
///
/// Two, from `PRODUCT_SPEC.md` §2.1: *"generous enough to prove value; tight enough to
/// survive"*. A demo is one analysis; a second is somebody trying their own idea.
pub const DEFAULT_DAILY_LIMIT: usize = 2;

/// The environment variable that changes it.
pub const LIMIT_VAR: &str = "ANONYMOUS_DAILY_LIMIT";

/// Whether this request may start a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Allowed {
    /// Start it.
    Yes,
    /// Refuse, and say how many they have had.
    No { used: usize, limit: usize },
}

/// The counts, for today.
#[derive(Debug)]
struct Today {
    on: NaiveDate,
    counts: HashMap<u64, usize>,
}

/// How many runs each anonymous address has started today.
#[derive(Debug)]
pub struct Cap {
    limit: usize,
    /// Per process, from the operating system. Never logged, never stored.
    keys: RandomState,
    today: Mutex<Today>,
}

impl Cap {
    /// A cap of `limit` runs a day.
    #[must_use]
    pub fn of(limit: usize) -> Self {
        Self {
            limit,
            keys: RandomState::new(),
            today: Mutex::new(Today {
                // Any date: the first request replaces it, because the day is compared rather
                // than assumed. Starting from a real clock read here would be one more thing
                // that cannot be tested without waiting.
                on: NaiveDate::default(),
                counts: HashMap::new(),
            }),
        }
    }

    /// From the environment, or [`DEFAULT_DAILY_LIMIT`].
    ///
    /// An unreadable value is the default rather than a failure to boot: a typo in an
    /// environment variable should cost the operator a stricter limit than they meant, not a
    /// service that will not start.
    #[must_use]
    pub fn from_env() -> Self {
        let limit = std::env::var(LIMIT_VAR)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(DEFAULT_DAILY_LIMIT);
        Self::of(limit)
    }

    /// How many a day this allows. For the message a refusal carries.
    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Count one attempt, and say whether it may proceed.
    ///
    /// `client` is `None` when the request did not arrive through a proxy — see the module
    /// documentation. `today` is an argument rather than a clock read, so "it resets tomorrow"
    /// is a test rather than a promise.
    ///
    /// **Counted before the run, not after.** A run that fails still spent the machine's time,
    /// and a cap that only counted successes would be defeated by prompts that fail.
    pub fn allow(&self, client: Option<&str>, today: NaiveDate) -> Allowed {
        let Some(client) = client else {
            return Allowed::Yes;
        };
        // A limit of zero would refuse everybody, which is a plausible thing to configure by
        // accident and never a thing to configure on purpose.
        if self.limit == 0 {
            return Allowed::Yes;
        }

        let key = self.keys.hash_one(client);

        let Ok(mut state) = self.today.lock() else {
            // A poisoned lock means another thread panicked holding it. Refusing every request
            // afterwards would turn one panic into an outage; the cap is a guard rail, and a
            // guard rail that fails closed on the whole site is worse than one that fails open.
            return Allowed::Yes;
        };
        if state.on != today {
            state.on = today;
            state.counts.clear();
        }
        let used = state.counts.entry(key).or_insert(0);
        if *used >= self.limit {
            return Allowed::No {
                used: *used,
                limit: self.limit,
            };
        }
        *used += 1;
        Allowed::Yes
    }
}

impl Default for Cap {
    fn default() -> Self {
        Self::of(DEFAULT_DAILY_LIMIT)
    }
}

/// The client an `X-Forwarded-For` header names, if it names one.
///
/// **The rightmost entry**, which is the one our own proxy appended. See the module
/// documentation: taking the leftmost is how this kind of cap is bypassed.
#[must_use]
pub fn client_in(forwarded_for: Option<&str>) -> Option<String> {
    let header = forwarded_for?;
    let last = header
        .rsplit(',')
        .map(str::trim)
        .find(|entry| !entry.is_empty())?;
    Some(last.to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, n).expect("a real date")
    }

    #[test]
    fn two_analyses_are_allowed_and_the_third_is_not() {
        // PRODUCT_SPEC §2.1's number, as behaviour.
        let cap = Cap::of(2);
        assert_eq!(cap.allow(Some("198.51.100.7"), day(6)), Allowed::Yes);
        assert_eq!(cap.allow(Some("198.51.100.7"), day(6)), Allowed::Yes);
        assert_eq!(
            cap.allow(Some("198.51.100.7"), day(6)),
            Allowed::No { used: 2, limit: 2 }
        );
    }

    #[test]
    fn one_address_using_it_all_does_not_refuse_anybody_else() {
        // The failure that would make this worse than having no cap: one visitor exhausting
        // the day for everybody who arrives after them.
        let cap = Cap::of(2);
        for _ in 0..5 {
            cap.allow(Some("198.51.100.7"), day(6));
        }
        assert_eq!(cap.allow(Some("203.0.113.9"), day(6)), Allowed::Yes);
    }

    #[test]
    fn tomorrow_starts_again() {
        let cap = Cap::of(1);
        assert_eq!(cap.allow(Some("198.51.100.7"), day(6)), Allowed::Yes);
        assert!(matches!(
            cap.allow(Some("198.51.100.7"), day(6)),
            Allowed::No { .. }
        ));
        assert_eq!(cap.allow(Some("198.51.100.7"), day(7)), Allowed::Yes);
    }

    #[test]
    fn a_new_day_forgets_yesterday_rather_than_accumulating() {
        // The memory half of the same rule. A map that only ever grew would be a slow leak on
        // a box with 24GB and no restarts.
        let cap = Cap::of(2);
        cap.allow(Some("198.51.100.7"), day(6));
        cap.allow(Some("203.0.113.9"), day(6));
        cap.allow(Some("192.0.2.1"), day(7));
        let held = cap.today.lock().expect("not poisoned").counts.len();
        assert_eq!(held, 1, "yesterday's addresses are still being counted");
    }

    #[test]
    fn a_request_that_did_not_come_through_a_proxy_is_not_capped() {
        // A laptop. Two a day would make the application unusable to whoever is building it,
        // and the abuse this exists for arrives over the internet.
        let cap = Cap::of(1);
        assert_eq!(cap.allow(None, day(6)), Allowed::Yes);
        assert_eq!(cap.allow(None, day(6)), Allowed::Yes);
    }

    #[test]
    fn a_limit_of_zero_is_read_as_no_cap_rather_than_no_service() {
        let cap = Cap::of(0);
        assert_eq!(cap.allow(Some("198.51.100.7"), day(6)), Allowed::Yes);
    }

    #[test]
    fn the_address_counted_is_the_one_our_proxy_saw() {
        // **The bypass this is written against.** A client sending its own `X-Forwarded-For`
        // does not replace the header, it prepends to it: the proxy appends what it actually
        // saw. So the leftmost entry is a claim and the rightmost is an observation, and
        // reading the first would let one machine mint a fresh quota per request.
        assert_eq!(
            client_in(Some("1.2.3.4, 198.51.100.7")).as_deref(),
            Some("198.51.100.7")
        );
    }

    #[test]
    fn a_forged_header_cannot_buy_a_second_allowance() {
        // The same point, as the behaviour a person would see rather than a parse.
        let cap = Cap::of(1);
        let first = client_in(Some("198.51.100.7"));
        let forged = client_in(Some("10.0.0.99, 198.51.100.7"));
        assert_eq!(cap.allow(first.as_deref(), day(6)), Allowed::Yes);
        assert!(matches!(
            cap.allow(forged.as_deref(), day(6)),
            Allowed::No { .. }
        ));
    }

    #[test]
    fn a_header_of_whitespace_names_nobody() {
        assert_eq!(client_in(Some("  ,  ")), None);
        assert_eq!(client_in(Some("")), None);
        assert_eq!(client_in(None), None);
    }

    #[test]
    fn one_entry_is_read_as_itself() {
        assert_eq!(
            client_in(Some("198.51.100.7")).as_deref(),
            Some("198.51.100.7")
        );
    }
}
