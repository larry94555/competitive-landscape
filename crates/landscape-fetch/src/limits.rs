//! One request at a time per host, with a gap between them.
//!
//! A report reads several pages from the same company. Fetched as fast as the network
//! allows, that is a small burst against one server — indistinguishable from the beginning
//! of a scrape, and enough to get us blocked by anyone watching.
//!
//! **Per-host, not global.** A global limit would slow a report reading ten different sites
//! for no reason, while still permitting ten requests at once to a single one. The load
//! that matters is the load on somebody else's server.
//!
//! The delay is a floor between *starts*, so a slow response does not add to it: waiting
//! two seconds after a request that already took three is politeness aimed at nobody.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The gap between requests to one host, when the site does not ask for a different one.
///
/// One second is the value most crawler documentation suggests and most operators expect.
/// A site naming a `Crawl-delay` overrides it — upward or downward — because a stated wish
/// beats our default in both directions.
pub const DEFAULT_DELAY: Duration = Duration::from_secs(1);

/// When each host may next be asked for something.
#[derive(Debug, Default)]
pub struct Pacer {
    next_allowed: HashMap<String, Instant>,
}

impl Pacer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How long to wait before starting a request to this host.
    ///
    /// Returns the duration rather than sleeping, so the decision is testable without a
    /// clock and the caller keeps control of *how* it waits — a worker may have something
    /// better to do than block.
    #[must_use]
    pub fn wait_for(&self, host: &str) -> Duration {
        self.next_allowed.get(host).map_or(Duration::ZERO, |at| {
            at.saturating_duration_since(Instant::now())
        })
    }

    /// Record that a request to this host is starting now.
    ///
    /// `delay` is the site's `Crawl-delay` where it stated one, and [`DEFAULT_DELAY`]
    /// otherwise.
    pub fn record(&mut self, host: impl Into<String>, delay: Duration) {
        self.next_allowed
            .insert(host.into(), Instant::now() + delay);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_host_we_have_never_asked_is_not_made_to_wait() {
        assert_eq!(Pacer::new().wait_for("example.com"), Duration::ZERO);
    }

    #[test]
    fn a_second_request_to_the_same_host_waits() {
        let mut p = Pacer::new();
        p.record("example.com", DEFAULT_DELAY);
        let wait = p.wait_for("example.com");
        assert!(wait > Duration::ZERO, "expected a wait, got {wait:?}");
        assert!(wait <= DEFAULT_DELAY);
    }

    #[test]
    fn one_slow_host_does_not_hold_up_another() {
        // The reason this is per-host. A report reading ten sites should not serialise
        // because one of them asked for a long delay.
        let mut p = Pacer::new();
        p.record("slow.example", Duration::from_secs(60));
        assert_eq!(p.wait_for("other.example"), Duration::ZERO);
    }

    #[test]
    fn a_site_asking_for_longer_gets_longer() {
        let mut p = Pacer::new();
        p.record("polite.example", Duration::from_secs(10));
        assert!(p.wait_for("polite.example") > DEFAULT_DELAY);
    }

    #[test]
    fn the_wait_expires_rather_than_accumulating() {
        // saturating_duration_since is what makes this true: once the moment has passed the
        // answer is zero, not a negative number wrapping into a very long sleep.
        let mut p = Pacer::new();
        p.record("example.com", Duration::ZERO);
        assert_eq!(p.wait_for("example.com"), Duration::ZERO);
    }
}
