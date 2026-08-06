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
//! # A failed analysis costs nothing
//!
//! `PRODUCT_SPEC.md` §2.1 and `UI_FLOWS.md` §2.2 both say so, and the first version of this
//! did the opposite: it reserved an allowance before the prompt was even parsed, so a typo
//! spent half of somebody's day. Review found it.
//!
//! **Nothing is reserved now.** What is kept is the *list of analyses this address started
//! today*, and how many of them still count is asked of the store each time — a run that ended
//! in [`AnalysisStatus::Failed`] is not one of them. That makes the rule fall out of the data
//! rather than needing a refund path, which matters because there is nowhere to refund *to*:
//! the API and the worker are separate processes, and a run that fails does so in the other
//! one.
//!
//! # What is stored
//!
//! A keyed hash of the address, and the ids it started, in memory, for one day.
//!
//! The key comes from [`RandomState`] — the operating system's randomness, fresh per process —
//! so the table cannot be read back to an address, and a restart makes yesterday's hashes
//! meaningless. It is a keyed hash rather than a cryptographic commitment, which is the right
//! strength for a number that never leaves memory and is never written down.
//!
//! **A restart resets the counts.** That is the honest cost of not putting this in the
//! database, and it is proportionate: the cap exists to stop a URL being drained by strangers,
//! not to be an accounting record. If it ever needs to survive a restart, it needs a table —
//! and an address in that table, which is a privacy decision rather than a schema one.
//!
//! [`AnalysisStatus::Failed`]: landscape_core::AnalysisStatus::Failed
//!
//! [`DEPLOY.md`]: https://github.com/larry94555/competitive-landscape/blob/main/docs/DEPLOY.md
//! [`RandomState`]: std::collections::hash_map::RandomState

use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use landscape_core::AnalysisId;

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

/// What each address started today.
#[derive(Debug, Default)]
struct Today {
    on: NaiveDate,
    /// Ids rather than a number, because whether one still counts is a question about the
    /// analysis and can change after it was started.
    started: HashMap<u64, Vec<AnalysisId>>,
    /// One lock per address, so a decision about it cannot overlap another.
    ///
    /// **Deciding is not one step.** Reading what an address started, asking the store which
    /// of those still count, enqueueing and remembering the new id are four, with an `await`
    /// between each - so without this, twenty requests arriving together all read the same
    /// empty list and all pass a limit of two. Review reproduced exactly that: nine of twenty
    /// were accepted.
    ///
    /// Per address rather than one lock for everybody, because the work being serialised
    /// includes store reads and one visitor should not wait behind another's.
    gates: HashMap<u64, Arc<tokio::sync::Mutex<()>>>,
}

/// Which runs each anonymous address has started today.
#[derive(Debug)]
pub struct Cap {
    limit: usize,
    /// Per process, from the operating system. Never logged, never stored.
    keys: RandomState,
    today: Mutex<Today>,
}

impl Today {
    /// Move to `today` if it is newer, and say whether this is the day being held.
    ///
    /// **The clock only goes forward.** Each request captures the date *before* it waits for
    /// its address's gate and for the store, so one admitted a second before midnight can
    /// finish a second after it — and the version that reset on any difference let that
    /// request wind the day back and clear what the new day had already recorded. The next
    /// request then rolled forward into an empty day and was given a fresh allowance. Review
    /// reproduced it through the public API: record an id on the 7th, record a stale one on
    /// the 6th, and the 7th's id is gone.
    ///
    /// A stale caller is now ignored rather than allowed to rewrite history. It costs at most
    /// one uncounted run per address per midnight — bounded, and in the direction of letting
    /// somebody through rather than losing the record of everybody.
    ///
    /// One place, because four methods need it and four copies of a date comparison is four
    /// chances for one of them to keep yesterday.
    fn on_day(&mut self, today: NaiveDate) -> bool {
        if today > self.on {
            self.on = today;
            self.started.clear();
            self.gates.clear();
        }
        today == self.on
    }
}

impl Cap {
    /// A cap of `limit` runs a day.
    #[must_use]
    pub fn of(limit: usize) -> Self {
        Self {
            limit,
            keys: RandomState::new(),
            // Any date: the first request replaces it, because the day is compared rather
            // than assumed. Starting from a real clock read here would be one more thing that
            // cannot be tested without waiting.
            today: Mutex::new(Today::default()),
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

    /// The lock to hold while deciding about this address.
    ///
    /// Held across the whole sequence — read, reconcile, enqueue, record — because any gap
    /// between them is a gap two requests can both fit through.
    #[must_use]
    pub fn gate_for(&self, client: &str, today: NaiveDate) -> Arc<tokio::sync::Mutex<()>> {
        let Ok(mut state) = self.today.lock() else {
            // A poisoned lock means another thread panicked holding it. A fresh gate serialises
            // nothing, which is the same failing-open this type does everywhere else: a guard
            // rail that takes the site down is worse than one that lets a request past.
            return Arc::new(tokio::sync::Mutex::new(()));
        };
        if !state.on_day(today) {
            // Yesterday's request, finishing late. A gate of its own serialises it against
            // nothing, which is right: there is nothing left of its day to protect.
            return Arc::new(tokio::sync::Mutex::new(()));
        }
        Arc::clone(state.gates.entry(self.keys.hash_one(client)).or_default())
    }

    /// Replace what this address is holding, after the store has been asked about each id.
    ///
    /// **Failed ids are dropped here rather than skipped every time.** They are free, so an
    /// address can accumulate them without limit — and each one would then cost a store read
    /// on every later request, turning `n` failures into `n²` reads across a day and growing
    /// this map for as long as the process lives. Review found it. Only ids that still count
    /// are kept, which bounds the list by the work in flight.
    pub fn keep(&self, client: &str, today: NaiveDate, still: Vec<AnalysisId>) {
        let Ok(mut state) = self.today.lock() else {
            return;
        };
        if !state.on_day(today) {
            return;
        }
        let key = self.keys.hash_one(client);
        if still.is_empty() {
            state.started.remove(&key);
        } else {
            state.started.insert(key, still);
        }
    }

    /// Whether this cap applies to this request at all.
    ///
    /// `None` means the request did not arrive through a proxy — see the module documentation
    /// — and a limit of zero is read as "no cap", because refusing everybody is a plausible
    /// thing to configure by accident and never a thing to configure on purpose.
    #[must_use]
    pub const fn applies_to(&self, client: Option<&str>) -> bool {
        client.is_some() && self.limit > 0
    }

    /// The analyses this address started today, oldest first.
    ///
    /// **Ids, not a count.** How many of them still count against the day is a question about
    /// each analysis — a failed one costs nothing — and the caller is the one that can ask.
    ///
    /// `today` is an argument rather than a clock read, so "it resets tomorrow" is a test
    /// rather than a promise.
    pub fn started_today(&self, client: &str, today: NaiveDate) -> Vec<AnalysisId> {
        let Ok(mut state) = self.today.lock() else {
            // A poisoned lock means another thread panicked holding it. Refusing every request
            // afterwards would turn one panic into an outage; the cap is a guard rail, and a
            // guard rail that fails closed on the whole site is worse than one that fails open.
            return Vec::new();
        };
        if !state.on_day(today) {
            return Vec::new();
        }
        state
            .started
            .get(&self.keys.hash_one(client))
            .cloned()
            .unwrap_or_default()
    }

    /// Remember that this address started this analysis.
    ///
    /// **Called after the store accepted it**, so a prompt that was refused and an enqueue that
    /// failed both cost nothing — which is what `PRODUCT_SPEC.md` §2.1 asks for and what the
    /// first version of this got wrong.
    pub fn record(&self, client: &str, today: NaiveDate, started: AnalysisId) {
        let Ok(mut state) = self.today.lock() else {
            return;
        };
        if !state.on_day(today) {
            return;
        }
        state
            .started
            .entry(self.keys.hash_one(client))
            .or_default()
            .push(started);
    }
}

/// The instant this day's allowance comes back: the next midnight, UTC.
///
/// **Stated rather than implied.** `UI_FLOWS.md` §2.2 requires the refusal to say *when* it
/// resets, and "come back tomorrow" is not that: west of UTC the allowance returns later the
/// same local day, and east of it, sooner than the word suggests.
#[must_use]
pub fn resets_after(now: DateTime<Utc>) -> DateTime<Utc> {
    (now.date_naive() + chrono::Days::new(1))
        .and_hms_opt(0, 0, 0)
        .map_or(now, |midnight| midnight.and_utc())
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

    fn an_id() -> AnalysisId {
        AnalysisId::new()
    }

    #[test]
    fn what_an_address_started_today_comes_back() {
        let cap = Cap::of(2);
        let first = an_id();
        cap.record("198.51.100.7", day(6), first);
        assert_eq!(cap.started_today("198.51.100.7", day(6)), vec![first]);
    }

    #[test]
    fn one_address_does_not_see_another_one_s_runs() {
        // The failure that would make this worse than having no cap: one visitor exhausting
        // the day for everybody who arrives after them.
        let cap = Cap::of(2);
        cap.record("198.51.100.7", day(6), an_id());
        assert!(cap.started_today("203.0.113.9", day(6)).is_empty());
    }

    #[test]
    fn tomorrow_starts_again() {
        let cap = Cap::of(1);
        cap.record("198.51.100.7", day(6), an_id());
        assert!(cap.started_today("198.51.100.7", day(7)).is_empty());
    }

    #[test]
    fn a_new_day_forgets_yesterday_rather_than_accumulating() {
        // The memory half of the same rule. A map that only ever grew would be a slow leak on
        // a box with 24GB and no restarts.
        let cap = Cap::of(2);
        cap.record("198.51.100.7", day(6), an_id());
        cap.record("203.0.113.9", day(6), an_id());
        cap.record("192.0.2.1", day(7), an_id());
        let held = cap.today.lock().expect("not poisoned").started.len();
        assert_eq!(held, 1, "yesterday's addresses are still being counted");
    }

    #[test]
    fn a_request_that_did_not_come_through_a_proxy_is_not_capped() {
        // A laptop. Two a day would make the application unusable to whoever is building it,
        // and the abuse this exists for arrives over the internet.
        assert!(!Cap::of(1).applies_to(None));
    }

    #[test]
    fn a_limit_of_zero_is_read_as_no_cap_rather_than_no_service() {
        assert!(!Cap::of(0).applies_to(Some("198.51.100.7")));
        assert!(Cap::of(1).applies_to(Some("198.51.100.7")));
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
    fn a_forged_header_lands_in_the_same_bucket_as_the_honest_one() {
        // The same point, as the behaviour rather than the parse: both spellings of the same
        // client have to be one address, or one machine has as many allowances as it likes.
        let cap = Cap::of(1);
        let honest = client_in(Some("198.51.100.7")).expect("an address");
        let forged = client_in(Some("10.0.0.99, 198.51.100.7")).expect("an address");
        let started = an_id();
        cap.record(&honest, day(6), started);
        assert_eq!(cap.started_today(&forged, day(6)), vec![started]);
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

    #[test]
    fn a_request_that_crossed_midnight_does_not_wipe_the_new_day() {
        // **Review's reproduction, exactly.** Every request captures the date before it waits
        // for its address's gate and for the store, so one admitted a second before midnight
        // can finish a second after it. Resetting on any difference let that request wind the
        // day back and clear what the new day had recorded - and the next request rolled
        // forward into an empty day and was handed a fresh allowance.
        let cap = Cap::of(2);
        let todays = an_id();
        cap.record("198.51.100.7", day(7), todays);
        cap.record("198.51.100.7", day(6), an_id());

        assert_eq!(
            cap.started_today("198.51.100.7", day(7)),
            vec![todays],
            "a request from yesterday erased today"
        );
    }

    #[test]
    fn a_late_reconciliation_from_yesterday_changes_nothing() {
        // The same boundary through the other writer. `keep` replaces a list, so a stale one
        // would not merely add - it would overwrite the new day with the old day's survivors.
        let cap = Cap::of(2);
        let todays = an_id();
        cap.record("198.51.100.7", day(7), todays);
        cap.keep("198.51.100.7", day(6), vec![an_id(), an_id()]);

        assert_eq!(cap.started_today("198.51.100.7", day(7)), vec![todays]);
    }

    #[test]
    fn yesterday_reads_as_empty_rather_than_as_today() {
        // A stale reader must not be handed the new day's list either: it would count runs
        // against an allowance that has already been reset, and refuse somebody who is owed
        // one.
        let cap = Cap::of(2);
        cap.record("198.51.100.7", day(7), an_id());
        assert!(cap.started_today("198.51.100.7", day(6)).is_empty());
    }

    #[test]
    fn the_allowance_comes_back_at_the_next_midnight_utc() {
        // `UI_FLOWS.md` §2.2 asks the refusal to say *when*, so the instant has to be a value
        // rather than a word. Late in the day and early in it must both land on the same
        // boundary, or the sentence is worse than "tomorrow" was.
        let late = "2026-08-06T23:59:00Z"
            .parse::<DateTime<Utc>>()
            .expect("a real instant");
        let early = "2026-08-06T00:01:00Z"
            .parse::<DateTime<Utc>>()
            .expect("a real instant");
        let midnight = "2026-08-07T00:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("a real instant");
        assert_eq!(resets_after(late), midnight);
        assert_eq!(resets_after(early), midnight);
    }
}
