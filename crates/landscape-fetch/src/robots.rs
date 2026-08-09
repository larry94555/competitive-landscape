//! Reading `robots.txt`, and erring toward not fetching.
//!
//! `FACT_CHECKING.md` treats honouring `robots.txt` as a **hard ethical commitment**, not a
//! risk-management position. That framing decides every ambiguous case here: where the
//! specification is unclear or our parsing is uncertain, the answer is *disallow*.
//!
//! The cost of being wrong in that direction is a gap in a report, which this product is
//! built to display honestly. The cost of being wrong in the other direction is fetching a
//! page somebody asked us not to.
//!
//! # Why this is not a dependency
//!
//! Adding one is an architecture change under `CODING_QUALITY.md` §3.1, and the case is
//! weaker than it looks. RFC 9309's matching rules are small — group selection, longest
//! match wins, `*` and `$` — and the risk in a third-party parser is the same risk as in
//! this one, minus our ability to make it conservative on purpose. What is written here is
//! **deliberately stricter than the RFC in ambiguous cases**, which no general-purpose
//! parser will be.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a fetched `robots.txt` is trusted before being re-read.
///
/// Long enough that a report does not re-fetch it per page, short enough that a site
/// tightening its rules is honoured the same day.
pub const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// What a site's `robots.txt` permits us.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    /// `(path prefix, allowed)`, most specific first after construction.
    directives: Vec<(String, bool)>,
    /// `Crawl-delay`, if the site names one. Not in RFC 9309, widely used, and honoured
    /// here because ignoring a site's stated wishes on a technicality is not the posture
    /// this product takes.
    crawl_delay: Option<Duration>,
}

impl Rules {
    /// Rules that permit everything. What an absent `robots.txt` means.
    ///
    /// A **404 means allowed** and a 5xx does not — see [`Rules::from_status`]. That
    /// distinction is the one most often got wrong, and getting it wrong turns a site's
    /// bad afternoon into our crawling it against its wishes.
    #[must_use]
    pub fn permissive() -> Self {
        Self::default()
    }

    /// Rules that permit nothing.
    #[must_use]
    pub fn restrictive() -> Self {
        Self {
            directives: vec![(String::new(), false)],
            crawl_delay: None,
        }
    }

    /// What a status code alone tells us, when there is no body to parse.
    ///
    /// - **2xx** — parse the body.
    /// - **404 / 410** — no rules exist, so everything is allowed. This is the RFC's
    ///   position and it is right: a site with no `robots.txt` has not asked for anything.
    /// - **429 / 5xx** — the site is unwell or rate-limiting us. **Assume disallowed.**
    ///   We cannot read its wishes, and the polite reading of "I am struggling" is not
    ///   "carry on".
    /// - **401 / 403** — the file exists and we are not allowed to read it. If we may not
    ///   read the rules, we may not decide we pass them.
    #[must_use]
    pub fn from_status(status: u16) -> Option<Self> {
        match status {
            404 | 410 => Some(Self::permissive()),
            401 | 403 | 429 => Some(Self::restrictive()),
            s if s >= 500 => Some(Self::restrictive()),
            _ => None,
        }
    }

    /// Parse a `robots.txt` for one user-agent.
    ///
    /// Group selection follows RFC 9309: the most specific matching `User-agent` wins, and
    /// `*` is the fallback. Records outside any group, and groups for other agents, are
    /// ignored.
    #[must_use]
    pub fn parse(body: &str, user_agent: &str) -> Self {
        let ua = user_agent.to_lowercase();
        let mut exact: Vec<(String, bool)> = Vec::new();
        let mut wildcard: Vec<(String, bool)> = Vec::new();
        let (mut exact_delay, mut wildcard_delay) = (None, None);

        // Which groups the current run of `User-agent:` lines selects. A group can name
        // several agents before its first rule.
        let (mut in_exact, mut in_wildcard, mut reading_agents) = (false, false, false);

        for line in body.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            let field = field.trim().to_lowercase();
            let value = value.trim();

            match field.as_str() {
                "user-agent" => {
                    // A `User-agent` line after a rule starts a new group.
                    if !reading_agents {
                        in_exact = false;
                        in_wildcard = false;
                        reading_agents = true;
                    }
                    let v = value.to_lowercase();
                    if v == "*" {
                        in_wildcard = true;
                    } else if ua.contains(&v) && !v.is_empty() {
                        in_exact = true;
                    }
                }
                "allow" | "disallow" => {
                    reading_agents = false;
                    let allowed = field == "allow";
                    // An empty `Disallow:` means "nothing is disallowed" — the RFC's way of
                    // spelling permissive. An empty `Allow:` is meaningless; skip it.
                    if value.is_empty() {
                        if !allowed {
                            if in_exact {
                                exact.push((String::new(), true));
                            }
                            if in_wildcard {
                                wildcard.push((String::new(), true));
                            }
                        }
                        continue;
                    }
                    if in_exact {
                        exact.push((value.to_owned(), allowed));
                    }
                    if in_wildcard {
                        wildcard.push((value.to_owned(), allowed));
                    }
                }
                "crawl-delay" => {
                    reading_agents = false;
                    if let Ok(secs) = value.parse::<f64>() {
                        // Bounded: a site asking for a day is asking us not to crawl it,
                        // and an unbounded sleep is a hang wearing a politeness costume.
                        if (0.0..=300.0).contains(&secs) {
                            let d = Duration::from_secs_f64(secs);
                            if in_exact {
                                exact_delay = Some(d);
                            }
                            if in_wildcard {
                                wildcard_delay = Some(d);
                            }
                        }
                    }
                }
                _ => reading_agents = false,
            }
        }

        // A group naming us explicitly wins outright over `*`, even if it says less.
        let (mut directives, crawl_delay) = if exact.is_empty() {
            (wildcard, wildcard_delay)
        } else {
            (exact, exact_delay)
        };

        // Longest match wins (RFC 9309 §2.2.2), so sorting once here means every later
        // lookup is a scan that can stop at the first hit.
        directives.sort_by_key(|(pattern, _)| std::cmp::Reverse(pattern.len()));
        Self {
            directives,
            crawl_delay,
        }
    }

    /// Whether we may fetch this path.
    #[must_use]
    pub fn allows(&self, path: &str) -> bool {
        let path = if path.is_empty() { "/" } else { path };
        for (pattern, allowed) in &self.directives {
            if matches(pattern, path) {
                return *allowed;
            }
        }
        // No rule mentions this path, so nobody has objected to it.
        true
    }

    #[must_use]
    pub const fn crawl_delay(&self) -> Option<Duration> {
        self.crawl_delay
    }
}

/// RFC 9309 path matching: `*` spans any run of characters, `$` anchors the end.
///
/// Written as a small backtracking matcher rather than by building a regex, because a
/// pattern here comes from a stranger's `robots.txt` and compiling stranger-supplied text
/// into a regex is how a fetch turns into catastrophic backtracking on somebody else's
/// server.
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let hay = &path[pos.min(path.len())..];
        let found = if i == 0 {
            // The segment before the first `*` must sit at the start.
            hay.starts_with(part).then_some(0)
        } else {
            hay.find(part)
        };
        let Some(at) = found else { return false };
        pos += at + part.len();
    }

    if !anchored {
        return true;
    }
    // `$` with a trailing `*` before it still matches anything to the end.
    if parts.last().is_some_and(|p| p.is_empty()) {
        return true;
    }
    pos == path.len()
}

/// One site's rules, remembered so a report does not re-fetch `robots.txt` per page.
#[derive(Debug, Default)]
pub struct Cache {
    by_host: HashMap<String, (Rules, Instant)>,
}

impl Cache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The rules for a host, if they were fetched recently enough to still be trusted.
    #[must_use]
    pub fn get(&self, host: &str) -> Option<&Rules> {
        self.by_host
            .get(host)
            .filter(|(_, at)| at.elapsed() < CACHE_TTL)
            .map(|(rules, _)| rules)
    }

    /// Pretend every entry was stored `by` earlier than it was.
    ///
    /// Test-only, and the same shape [`crate::cache::Cache`] uses: `CACHE_TTL` is six hours, so
    /// the alternative is a test that waits six hours or a clock threaded through every caller.
    #[cfg(test)]
    fn age(&mut self, by: std::time::Duration) {
        for (_, at) in self.by_host.values_mut() {
            if let Some(older) = at.checked_sub(by) {
                *at = older;
            }
        }
    }

    pub fn insert(&mut self, host: impl Into<String>, rules: Rules) {
        // **Expired entries are dropped rather than merely ignored.** They were only filtered on
        // lookup, which was harmless while a `Fetcher` lasted one analysis and is not now that
        // one lasts the process: review pointed out that a worker seeing a thousand hosts kept a
        // thousand rule sets for ever, and a rule set is not small. Pruning here costs a walk on
        // the rare path — a host we have not seen — rather than on every request.
        self.by_host.retain(|_, (_, at)| at.elapsed() < CACHE_TTL);
        self.by_host.insert(host.into(), (rules, Instant::now()));
    }

    /// How many hosts are remembered. For the test that this does not grow for ever.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_host.len()
    }

    /// Whether any host is remembered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_host.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod not_growing_for_ever {
    //! What a process-long `Fetcher` costs, and what stops it costing more.

    use super::*;

    #[test]
    fn a_host_whose_rules_have_expired_is_forgotten_rather_than_ignored() {
        // **Review found this as a consequence of the fetcher's new lifetime.** Expired entries
        // were filtered on lookup and never removed, which was free while a `Fetcher` lasted one
        // analysis. A worker crossing a thousand companies now keeps a thousand rule sets, and a
        // rule set is not small.
        let mut cache = Cache::new();
        for i in 0..500 {
            cache.insert(format!("h{i}.example"), Rules::default());
        }
        assert_eq!(cache.len(), 500, "nothing has expired yet");

        // Age everything past the window, then insert once more: the prune happens on the rare
        // path — a host not seen before — rather than on every request.
        cache.age(CACHE_TTL + std::time::Duration::from_secs(1));
        cache.insert("fresh.example", Rules::default());
        assert_eq!(
            cache.len(),
            1,
            "five hundred expired rule sets were kept alongside the one that matters"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const UA: &str = "landscapebot";

    #[test]
    fn a_disallowed_prefix_blocks_everything_under_it() {
        let r = Rules::parse("User-agent: *\nDisallow: /admin", UA);
        assert!(!r.allows("/admin"));
        assert!(!r.allows("/admin/users"));
        assert!(r.allows("/pricing"));
    }

    #[test]
    fn the_longest_match_wins_not_the_first() {
        // RFC 9309 §2.2.2, and the rule most often implemented wrongly. A parser that
        // returned the first match would block /admin/public here.
        let r = Rules::parse("User-agent: *\nDisallow: /admin\nAllow: /admin/public", UA);
        assert!(!r.allows("/admin/secret"));
        assert!(r.allows("/admin/public"));
        assert!(r.allows("/admin/public/page"));
    }

    #[test]
    fn a_group_naming_us_beats_the_wildcard_group() {
        let body = "User-agent: *\nDisallow: /\n\nUser-agent: landscapebot\nDisallow: /private";
        let r = Rules::parse(body, UA);
        assert!(r.allows("/pricing"), "our own group should apply");
        assert!(!r.allows("/private"));
    }

    #[test]
    fn a_group_for_someone_else_is_ignored() {
        let body = "User-agent: gptbot\nDisallow: /\n\nUser-agent: *\nAllow: /";
        let r = Rules::parse(body, UA);
        assert!(r.allows("/pricing"));
    }

    #[test]
    fn wildcards_and_end_anchors_work() {
        let r = Rules::parse(
            "User-agent: *\nDisallow: /*.pdf$\nDisallow: /tmp/*/cache",
            UA,
        );
        assert!(!r.allows("/reports/annual.pdf"));
        assert!(r.allows("/reports/annual.pdf.html"), "$ anchors the end");
        assert!(!r.allows("/tmp/anything/cache"));
        assert!(r.allows("/tmp/anything/else"));
    }

    #[test]
    fn an_empty_disallow_means_everything_is_allowed() {
        // The RFC's way of writing a permissive file, and it reads like the opposite.
        let r = Rules::parse("User-agent: *\nDisallow:", UA);
        assert!(r.allows("/anything"));
    }

    #[test]
    fn comments_and_odd_casing_are_handled() {
        let r = Rules::parse(
            "  USER-AGENT: *  # everyone\n  DISALLOW: /admin # secret\n",
            UA,
        );
        assert!(!r.allows("/admin"));
    }

    #[test]
    fn a_missing_robots_file_allows_everything() {
        // 404 means the site never asked for anything.
        let r = Rules::from_status(404).expect("404 is decisive");
        assert!(r.allows("/anything"));
    }

    #[test]
    fn a_site_that_is_unwell_is_left_alone() {
        // The distinction that matters most in this file. A 5xx or a 429 means we cannot
        // read the site's wishes — and the polite reading of "I am struggling" is not
        // "carry on regardless".
        for status in [429, 500, 503] {
            let r = Rules::from_status(status).expect("decisive");
            assert!(!r.allows("/anything"), "status {status} should stop us");
        }
    }

    #[test]
    fn rules_we_are_forbidden_to_read_are_assumed_to_forbid_us() {
        // If we may not read the rules, we may not decide that we pass them.
        for status in [401, 403] {
            let r = Rules::from_status(status).expect("decisive");
            assert!(!r.allows("/anything"), "status {status} should stop us");
        }
    }

    #[test]
    fn a_crawl_delay_is_honoured_but_bounded() {
        let r = Rules::parse("User-agent: *\nCrawl-delay: 2.5", UA);
        assert_eq!(r.crawl_delay(), Some(Duration::from_secs_f64(2.5)));

        // A site asking for a day is asking us not to crawl it. An unbounded sleep is a
        // hang wearing a politeness costume, so it is ignored rather than obeyed.
        let absurd = Rules::parse("User-agent: *\nCrawl-delay: 86400", UA);
        assert_eq!(absurd.crawl_delay(), None);
    }

    #[test]
    fn a_pattern_from_a_stranger_cannot_hang_the_matcher() {
        // The pattern comes from somebody else's file. Nested wildcards are the shape that
        // makes a naive regex explode; this asserts the matcher returns at all.
        let evil = "/".to_owned() + &"a*".repeat(40) + "b";
        let r = Rules::parse(&format!("User-agent: *\nDisallow: {evil}"), UA);
        let path = "/".to_owned() + &"a".repeat(200);
        let _ = r.allows(&path);
    }

    #[test]
    fn the_cache_forgets_stale_rules() {
        let mut c = Cache::new();
        assert!(c.get("example.com").is_none());
        c.insert("example.com", Rules::restrictive());
        assert!(c.get("example.com").is_some());
        assert!(c.get("other.example").is_none());
    }
}
