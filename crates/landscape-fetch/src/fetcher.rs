//! The part that actually goes and gets a page.
//!
//! Everything else in this crate decides *whether* to fetch. This decides *how*, and its
//! job is to make sure the decisions are the ones actually enforced — a guard that runs and
//! is then bypassed by the HTTP client is worse than no guard, because it reads like safety.
//!
//! Three properties are load-bearing and each has a test:
//!
//! 1. **Redirects are followed by hand.** Automatic following would check the first URL and
//!    connect to the last. Each hop re-enters the whole check.
//! 2. **The connection is pinned to the verified address.** `reqwest`'s `resolve_to_addrs`
//!    overrides DNS for one host, so the socket goes where the guard looked and a second
//!    lookup cannot answer differently.
//! 3. **The size cap is enforced while streaming.** Checking `Content-Length` alone trusts
//!    a stranger's arithmetic; a body is abandoned mid-flight once it exceeds the cap.

use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use crate::limits::{Pacer, DEFAULT_DELAY};
use crate::robots::{self, Rules};
use crate::{FetchError, Page, Target, MAX_BYTES, MAX_REDIRECTS, TIMEOUT, USER_AGENT};

/// Fetches pages, remembering what each host has told us and what it already said.
///
/// **One per process, not one per analysis.** Everything remembered here — the pages, the
/// `robots.txt` rules, the per-host delay — is remembered *by this object*, so a second
/// `Fetcher` shares none of it. Three of them used to be built inside a single run, which meant
/// one company's `robots.txt` was fetched three times and a page read by the description pass
/// was read again by the analysis pass. The cache is only worth having if the thing holding it
/// outlives the question that filled it.
#[derive(Debug)]
pub struct Fetcher {
    pacer: Mutex<Pacer>,
    robots: Mutex<robots::Cache>,
    pages: Mutex<crate::cache::Cache>,
    /// Off for tests that must not reach the network, and for a `--ignore-robots` that does
    /// not exist and should not: the flag would be used, and the commitment is the product.
    obey_robots: bool,
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetcher {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pacer: Mutex::new(Pacer::new()),
            robots: Mutex::new(robots::Cache::new()),
            pages: Mutex::new(crate::cache::Cache::new()),
            obey_robots: true,
        }
    }

    /// How many pages are held, and how many bytes they take.
    ///
    /// For the diagnostic. A cache nobody can see the size of is one nobody notices growing.
    #[must_use]
    pub fn cached(&self) -> (usize, usize) {
        self.pages.lock().map_or((0, 0), |c| (c.len(), c.bytes()))
    }

    /// Fetch one page, following redirects by hand and re-checking every hop.
    ///
    /// # Errors
    /// Any refusal, or a transport failure.
    pub async fn get(&self, url: &str) -> Result<Page, FetchError> {
        let mut target = Target::parse(url)?;

        // **Before the guard, robots and the pacer, and each is fine to skip for one reason:
        // nothing is sent.** The guard stops us reaching an address, robots stops us requesting
        // a path, the pacer stops us asking too often — none of them protects anything when no
        // request leaves this process.
        //
        // What makes that sound rather than convenient is the invariant on the way in: only a
        // page we were *allowed* to fetch is ever stored, because the insert below happens after
        // both have passed. A disallowed path is an error and is never in here to be served.
        if let Some(page) = self.pages.lock().ok().and_then(|c| c.get(url)) {
            tracing::debug!(url, "served from the page cache");
            return Ok(page);
        }

        for _ in 0..=MAX_REDIRECTS {
            let addr = self.approve(&target).await?;

            if self.obey_robots {
                let rules = self.rules_for(&target).await;
                if !rules.allows(&target.path) {
                    return Err(FetchError::RobotsDisallowed {
                        host: target.host.clone(),
                        path: target.path.clone(),
                    });
                }
                self.pace(&target.host, rules.crawl_delay()).await;
            }

            let response = self.send(&target, addr).await?;
            let status = response.status().as_u16();

            // 3xx with a Location: check the next hop from the top rather than trusting it.
            if (300..400).contains(&status) {
                let Some(location) = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned)
                else {
                    return Err(FetchError::Transport("redirect with no location".into()));
                };
                target = resolve_relative(&target, &location)?;
                continue;
            }

            let etag = header(&response, "etag");
            let last_modified = header(&response, "last-modified");
            let body = read_capped(response).await?;

            let page = Page {
                url: target.url(),
                status,
                body,
                etag,
                last_modified,
                fetched_at: chrono::Utc::now(),
            };
            // **Keyed on what was asked for, stored after every check has passed.** The second
            // half is the invariant the read above rests on; the first is because two callers
            // asking the same thing is the case this exists for, and they ask with the URL they
            // have rather than with the one a redirect landed on.
            if let Ok(mut cache) = self.pages.lock() {
                cache.insert(url.to_owned(), page.clone());
            }
            return Ok(page);
        }
        Err(FetchError::TooManyRedirects)
    }

    /// Resolve the host and approve every address it answers with.
    async fn approve(&self, target: &Target) -> Result<IpAddr, FetchError> {
        // A bare IP in the URL still goes through the guard — it is the case an attacker
        // reaches for first, and skipping DNS is not a reason to skip the check.
        if let Ok(ip) = target.host.parse::<IpAddr>() {
            return crate::approve_all(&[ip]);
        }

        let host_port = format!("{}:{}", target.host, target.port);
        let addrs: Vec<IpAddr> = tokio::net::lookup_host(host_port)
            .await
            .map_err(|e| FetchError::Transport(e.to_string()))?
            .map(|sa| sa.ip())
            .collect();

        crate::approve_all(&addrs)
    }

    /// This host's `robots.txt`, fetched once and remembered.
    async fn rules_for(&self, target: &Target) -> Rules {
        if let Ok(cache) = self.robots.lock() {
            if let Some(rules) = cache.get(&target.host) {
                return rules.clone();
            }
        }

        let rules = self.fetch_robots(target).await;
        if let Ok(mut cache) = self.robots.lock() {
            cache.insert(target.host.clone(), rules.clone());
        }
        rules
    }

    async fn fetch_robots(&self, target: &Target) -> Rules {
        let robots_target = Target {
            path: "/robots.txt".to_owned(),
            ..target.clone()
        };
        // The robots file is fetched through the same guard as everything else. It is a
        // URL from the same stranger, and exempting it would be a hole shaped exactly like
        // the thing the guard exists for.
        let Ok(addr) = self.approve(&robots_target).await else {
            return Rules::restrictive();
        };

        match self.send(&robots_target, addr).await {
            Ok(response) => {
                let status = response.status().as_u16();
                if let Some(decided) = Rules::from_status(status) {
                    return decided;
                }
                match read_capped(response).await {
                    Ok(body) => Rules::parse(&body, USER_AGENT),
                    // A robots.txt we could not read is one we must not assume permits us.
                    Err(_) => Rules::restrictive(),
                }
            }
            // Unreachable is not permission.
            Err(_) => Rules::restrictive(),
        }
    }

    /// Wait out this host's turn, then record that we are going again.
    async fn pace(&self, host: &str, crawl_delay: Option<Duration>) {
        let wait = self
            .pacer
            .lock()
            .map(|p| p.wait_for(host))
            .unwrap_or(Duration::ZERO);
        if wait > Duration::ZERO {
            tokio::time::sleep(wait).await;
        }
        if let Ok(mut p) = self.pacer.lock() {
            p.record(host, crawl_delay.unwrap_or(DEFAULT_DELAY));
        }
    }

    /// One request, with the connection pinned to the address the guard approved.
    async fn send(&self, target: &Target, addr: IpAddr) -> Result<reqwest::Response, FetchError> {
        // A client per request, because `resolve_to_addrs` is a builder-level override and
        // this is what makes the pinning real. Wasteful, and the waste is bounded by our
        // own rate limit — one request per host per second is not a connection-pool
        // problem. Worth revisiting only if a profile says so.
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&target.host, &[SocketAddr::new(addr, target.port)])
            .build()
            .map_err(|e| FetchError::Transport(e.to_string()))?;

        client
            .get(target.url())
            .send()
            .await
            .map_err(|e| FetchError::Transport(e.to_string()))
    }
}

fn header(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Read a body, giving up once it exceeds the cap.
///
/// Streamed rather than `.text()`: `Content-Length` is a stranger's claim about a stranger's
/// body, and a server that lies about it — or omits it — decides how much of our memory to
/// use. The cap is enforced against bytes actually received.
async fn read_capped(mut response: reqwest::Response) -> Result<String, FetchError> {
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| FetchError::Transport(e.to_string()))?
    {
        if buf.len() + chunk.len() > MAX_BYTES {
            return Err(FetchError::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    // Lossy on purpose: a page in an encoding we misread is still worth extracting from,
    // and refusing the whole page over one bad byte helps nobody.
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Work out where a `Location` header points.
///
/// # Errors
/// If the result is not a URL we would fetch.
pub fn resolve_relative(from: &Target, location: &str) -> Result<Target, FetchError> {
    let location = location.trim();
    if location.contains("://") {
        return Target::parse(location);
    }
    if let Some(rest) = location.strip_prefix("//") {
        let scheme = if from.https { "https" } else { "http" };
        return Target::parse(&format!("{scheme}://{rest}"));
    }
    if location.starts_with('/') {
        return Target::parse(&format!("{}{}", from.origin(), location));
    }
    // Relative to the current directory.
    let base = from.path.rsplit_once('/').map_or("/", |(dir, _)| dir);
    Target::parse(&format!("{}{}/{}", from.origin(), base, location))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod serving_from_memory {
    //! What the cache is worth, and the one thing it must never do.
    //!
    //! # Why there is no test over a real socket
    //!
    //! A test server binds `127.0.0.1`, and [`crate::guard`] refuses loopback — deliberately,
    //! absolutely, and with no flag to turn it off, for the same reason `--ignore-robots` does
    //! not exist: *the flag would be used, and the commitment is the product*. So the number
    //! this feature exists to change, **requests arriving at a stranger's server**, cannot be
    //! counted from inside this repository. That is a limit worth stating rather than a gap
    //! worth papering over, and it is stated in `BENCHMARKS.md` too.
    //!
    //! What can be established here is stronger than it looks, because the guard's refusal is
    //! the instrument: a loopback URL that comes back as a **page** can only have come from
    //! memory, since every path that reaches the network refuses it first.

    use super::*;

    fn page(url: &str, body: &str) -> Page {
        Page {
            url: url.to_owned(),
            status: 200,
            body: body.to_owned(),
            etag: None,
            last_modified: None,
            fetched_at: "2026-08-01T09:00:00Z".parse().unwrap(),
        }
    }

    /// Put a page in as a successful fetch would have, without one.
    fn seed(fetcher: &Fetcher, url: &str, body: &str) {
        fetcher
            .pages
            .lock()
            .unwrap()
            .insert(url.to_owned(), page(url, body));
    }

    #[tokio::test]
    async fn a_page_already_held_is_returned_without_a_request() {
        // **The guard is the proof.** `127.0.0.1` is refused by every path that would send
        // anything, so a page coming back at all means nothing was sent.
        let fetcher = Fetcher::new();
        let url = "http://127.0.0.1:9/pricing";
        seed(&fetcher, url, "<h1>Pricing</h1>");

        let served = fetcher.get(url).await.expect("served from memory");
        assert_eq!(served.body, "<h1>Pricing</h1>");
        assert_eq!(served.url, url, "a hit lost where the bytes came from");
    }

    #[tokio::test]
    async fn a_page_served_from_memory_says_when_it_was_actually_read() {
        // A claim's `as_of` comes from this. A cached page that restamped itself would make a
        // report dated today out of bytes read an hour ago, and say so nowhere.
        let fetcher = Fetcher::new();
        let url = "http://127.0.0.1:9/pricing";
        seed(&fetcher, url, "body");
        let served = fetcher.get(url).await.expect("served from memory");
        assert_eq!(
            served.fetched_at.to_rfc3339(),
            "2026-08-01T09:00:00+00:00",
            "the second read claimed to be newer than the bytes it returned"
        );
    }

    #[tokio::test]
    async fn a_refusal_leaves_nothing_behind_to_be_served_later() {
        // **The invariant the whole ordering rests on.** The cache is read before the guard and
        // before robots, which is only sound because nothing that failed either can be in it.
        let fetcher = Fetcher::new();
        let refused = fetcher.get("http://127.0.0.1:9/private").await;
        assert!(refused.is_err(), "loopback was fetched: {refused:?}");
        assert_eq!(
            fetcher.cached(),
            (0, 0),
            "a refusal was remembered, so the next attempt would be served it"
        );
    }

    #[tokio::test]
    async fn what_is_held_is_visible() {
        // A cache nobody can see the size of is one nobody notices growing.
        let fetcher = Fetcher::new();
        assert_eq!(fetcher.cached(), (0, 0));
        seed(&fetcher, "http://127.0.0.1:9/a", "1234");
        assert_eq!(fetcher.cached(), (1, 4));
    }

    #[tokio::test]
    async fn two_fetchers_share_nothing_which_is_why_the_worker_holds_one() {
        // The cache lives on the `Fetcher`, so this is the failure mode the worker had: three
        // of them inside one run, each paying for the same pages. Pinned so that going back to
        // a fetcher per pass fails here rather than quietly costing somebody else the requests.
        let one = Fetcher::new();
        seed(&one, "http://127.0.0.1:9/a", "held");
        let other = Fetcher::new();
        assert!(
            other.get("http://127.0.0.1:9/a").await.is_err(),
            "a second fetcher was served the first one's memory"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn target(url: &str) -> Target {
        Target::parse(url).expect("test url parses")
    }

    #[test]
    fn an_absolute_redirect_is_followed_as_written() {
        let next = resolve_relative(&target("https://a.example/x"), "https://b.example/y")
            .expect("resolves");
        assert_eq!(next.host, "b.example");
        assert_eq!(next.path, "/y");
    }

    #[test]
    fn a_root_relative_redirect_stays_on_the_same_host() {
        let next = resolve_relative(&target("https://a.example/x/y"), "/z").expect("resolves");
        assert_eq!(next.host, "a.example");
        assert_eq!(next.path, "/z");
    }

    #[test]
    fn a_directory_relative_redirect_resolves_against_the_directory() {
        let next =
            resolve_relative(&target("https://a.example/docs/page"), "other").expect("resolves");
        assert_eq!(next.path, "/docs/other");
    }

    #[test]
    fn a_protocol_relative_redirect_keeps_the_scheme() {
        let next =
            resolve_relative(&target("https://a.example/x"), "//b.example/y").expect("resolves");
        assert!(next.https);
        assert_eq!(next.host, "b.example");
    }

    #[test]
    fn a_redirect_to_a_scheme_we_do_not_fetch_is_refused_at_the_hop() {
        // The point of re-checking every hop: the first URL was fine and this one is not.
        assert!(matches!(
            resolve_relative(&target("https://a.example/x"), "file:///etc/passwd"),
            Err(FetchError::UnsupportedScheme { .. })
        ));
    }

    #[tokio::test]
    async fn a_url_naming_a_private_address_is_refused_before_any_request() {
        // No network reached: the guard runs on the parsed host, so this returns without a
        // socket being opened. That is what makes the test safe to run in CI.
        let f = Fetcher::new();
        for url in [
            "http://127.0.0.1/admin",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.1/",
            "http://[::1]/",
        ] {
            match f.get(url).await {
                Err(FetchError::Refused(_)) => {}
                other => panic!("{url} should have been refused, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_scheme_we_do_not_fetch_never_reaches_the_network() {
        let f = Fetcher::new();
        assert!(matches!(
            f.get("file:///etc/passwd").await,
            Err(FetchError::UnsupportedScheme { .. })
        ));
    }
}
