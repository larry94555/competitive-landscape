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

use reqwest::header::HeaderMap;

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
    pub async fn get(&self, url: &str, budget: &crate::Budget) -> Result<Page, FetchError> {
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

        // **What we still hold and may only need to have confirmed.** An entry past its hour is
        // not worthless: if the origin gave us an `ETag` or a `Last-Modified`, the question
        // *"has this changed?"* can be asked in a request whose answer carries no body at all.
        // Taken here, before anything is sent, and carried through the loop so a `304` needs
        // nothing from the cache that another thread could have evicted in the meantime.
        let held = self.pages.lock().ok().and_then(|c| c.stale(url));

        for hop in 0..=MAX_REDIRECTS {
            let addr = self.approve(&target).await?;

            if self.obey_robots {
                let rules = self.rules_for(&target, budget).await?;
                if !rules.allows(&target.path) {
                    return Err(FetchError::RobotsDisallowed {
                        host: target.host.clone(),
                        path: target.path.clone(),
                    });
                }
                self.pace(&target.host, rules.crawl_delay()).await;
            }

            let asking = conditional_on(hop, held.as_ref());
            let response = self.send(&target, addr, asking, budget).await?;
            let status = response.status().as_u16();
            let headers = response.headers().clone();

            match answer_to(status) {
                // **The cheapest answer an origin can give**, and the reason this is a `match`
                // on a type rather than two `if`s in a particular order — see [`answer_to`].
                Answer::NotModified => {
                    let (confirmed, stored_for) = confirmed(asking, chrono::Utc::now())?;
                    if let Ok(mut cache) = self.pages.lock() {
                        // **Under the policy it was already subject to**, not under whatever the
                        // `304` happens to repeat. A `304` may omit an unchanged `Cache-Control`,
                        // and reading it alone turns an origin's `max-age=30` into our hour.
                        cache.insert_revalidated(
                            url.to_owned(),
                            confirmed.clone(),
                            stored_for,
                            Said::read(&headers).freshness(),
                            confirmed.fetched_at,
                        );
                    }
                    tracing::debug!(url, "the origin says it has not changed");
                    return Ok(confirmed);
                }
                // A Location: check the next hop from the top rather than trusting it.
                Answer::Redirect => {
                    let Some(location) = headers
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned)
                    else {
                        return Err(FetchError::Transport("redirect with no location".into()));
                    };
                    target = resolve_relative(&target, &location)?;
                    continue;
                }
                Answer::Body => {}
            }

            let etag = header(&headers, "etag");
            let last_modified = header(&headers, "last-modified");
            // **What the origin says we may do with it.** Read before the body, because it comes
            // off the same response and forgetting it is how a `no-store` gets kept. Read, and
            // nothing more: which of these fields is list-based, and what they all mean, are both
            // decided where a test can reach them.
            let said = Said::read(&headers);
            let body = read_capped(response).await?;

            let page = Page {
                url: target.url(),
                status,
                body,
                etag,
                last_modified,
                fetched_at: chrono::Utc::now(),
            };
            // **Keyed on what was asked for, stored after every check has passed, and only for
            // as long as the origin allows.** The middle clause is the invariant the read above
            // rests on. The first is because two callers asking the same thing is the case this
            // exists for, and they ask with the URL they have rather than the one a redirect
            // landed on. The last is review's: a cache that argues it belongs beside
            // `robots.txt` cannot ignore the header a publisher states that in.
            if let Ok(mut cache) = self.pages.lock() {
                let held = cache.insert_allowed(
                    url.to_owned(),
                    page.clone(),
                    said.freshness(),
                    page.fetched_at,
                );
                tracing::debug!(url, held, "considered for the page cache");
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
    ///
    /// # Errors
    /// Only when the run has spent its allowance. Every other failure becomes
    /// [`Rules::restrictive`] — *unreachable is not permission* — but **being out of allowance
    /// is not the site's doing**, and reporting it as `robots.txt says no` would blame a
    /// stranger for our own bound and put a wrong sentence on a report.
    async fn rules_for(
        &self,
        target: &Target,
        budget: &crate::Budget,
    ) -> Result<Rules, FetchError> {
        if let Ok(cache) = self.robots.lock() {
            if let Some(rules) = cache.get(&target.host) {
                return Ok(rules.clone());
            }
        }

        let rules = self.fetch_robots(target, budget).await?;
        if let Ok(mut cache) = self.robots.lock() {
            cache.insert(target.host.clone(), rules.clone());
        }
        Ok(rules)
    }

    async fn fetch_robots(
        &self,
        target: &Target,
        budget: &crate::Budget,
    ) -> Result<Rules, FetchError> {
        let robots_target = Target {
            path: "/robots.txt".to_owned(),
            ..target.clone()
        };
        // The robots file is fetched through the same guard as everything else. It is a
        // URL from the same stranger, and exempting it would be a hole shaped exactly like
        // the thing the guard exists for.
        let Ok(addr) = self.approve(&robots_target).await else {
            return Ok(Rules::restrictive());
        };

        match self.send(&robots_target, addr, None, budget).await {
            Ok(response) => {
                let status = response.status().as_u16();
                if let Some(decided) = Rules::from_status(status) {
                    return Ok(decided);
                }
                Ok(match read_capped(response).await {
                    Ok(body) => Rules::parse(&body, USER_AGENT),
                    // A robots.txt we could not read is one we must not assume permits us.
                    Err(_) => Rules::restrictive(),
                })
            }
            Err(spent @ FetchError::BudgetSpent { .. }) => Err(spent),
            // Unreachable is not permission.
            Err(_) => Ok(Rules::restrictive()),
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
    ///
    /// **Every request that leaves this process passes through here**, which is why the
    /// allowance is spent here rather than in [`Self::get`]: a redirect hop is a request and a
    /// `robots.txt` is a request, and a bound counting only the pages a caller named would
    /// bound the thing nobody was worried about.
    async fn send(
        &self,
        target: &Target,
        addr: IpAddr,
        asking: Option<&crate::cache::Stale>,
        budget: &crate::Budget,
    ) -> Result<reqwest::Response, FetchError> {
        if !budget.spend() {
            return Err(FetchError::BudgetSpent {
                limit: budget.spent() + budget.left(),
            });
        }
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

        let mut request = client.get(target.url());
        for (name, value) in asking.map(asking_whether_it_changed).unwrap_or_default() {
            request = request.header(name, value);
        }
        request
            .send()
            .await
            .map_err(|e| FetchError::Transport(e.to_string()))
    }
}

/// What one response said about being kept.
///
/// **A type rather than four locals in [`Fetcher::get`], and that is the point.** Which field is
/// list-based and which is single-valued is a decision, and a decision made inside `get` is one
/// no test can reach — the address guard refuses loopback, so nothing drives that function. The
/// mutation harness said so about exactly this: *a directive on a second `Cache-Control` line is
/// never read* came back MISSED while the call lived up there. Here it is one call from an
/// assertion, and `get`'s share is a single line with no choice in it.
#[derive(Debug, Default)]
struct Said {
    cache_control: Option<String>,
    expires: Option<String>,
    date: Option<String>,
    age: Option<String>,
}

impl Said {
    /// Read the four fields, each in the way its own definition requires.
    fn read(headers: &HeaderMap) -> Self {
        Self {
            // `Cache-Control` is list-based and may arrive split across field lines.
            cache_control: combined(headers, "cache-control"),
            // The other three are single-valued: an instant, an instant, and a number.
            expires: header(headers, "expires"),
            date: header(headers, "date"),
            age: header(headers, "age"),
        }
    }

    fn freshness(&self) -> crate::cache::Freshness<'_> {
        crate::cache::Freshness {
            cache_control: self.cache_control.as_deref(),
            expires: self.expires.as_deref(),
            date: self.date.as_deref(),
            age: self.age.as_deref(),
        }
    }
}

/// One field's value, for the fields that have exactly one.
///
/// Takes the map rather than the response so a test can build one — the same reason
/// [`crate::cache::Cache::insert_allowed`] owns the storage decision. Nothing in this file can
/// be driven over a socket.
/// What a response's status means for the loop that is reading it.
///
/// **A type rather than two `if`s in the right order.** `304` is a `3xx`, so a redirect branch
/// written first swallows it: the cheapest answer an origin can give becomes *"redirect with no
/// location"*, and a conditional GET is worse than no conditional GET. Ordering is a property of
/// how the code happens to be laid out; this is a property of the status, and the `match` on it
/// has **no wildcard arm**, so a fourth kind of answer is a build error rather than a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// `304`. Nothing was sent back; what we already hold is current.
    NotModified,
    /// `3xx` with somewhere to go.
    Redirect,
    /// Everything else, including the failures: a `404` and a `503` are pages with a status.
    Body,
}

fn answer_to(status: u16) -> Answer {
    match status {
        304 => Answer::NotModified,
        300..=399 => Answer::Redirect,
        _ => Answer::Body,
    }
}

/// Whether this hop may carry the validators we hold.
///
/// **The first hop only.** They are the origin's proof about *the URL the caller asked for*; a
/// redirect lands somewhere else, and asking that somewhere else whether it matches another
/// page's `ETag` is a question about nothing — which an origin may well answer `304`, handing
/// back one page's body under another page's URL.
fn conditional_on(hop: usize, held: Option<&crate::cache::Stale>) -> Option<&crate::cache::Stale> {
    if hop == 0 {
        held
    } else {
        None
    }
}

/// The headers that turn a request into *"only if it changed"*.
///
/// **Both, when both exist.** `If-None-Match` is the one an origin ought to honour, and
/// `If-Modified-Since` is what an origin with no `ETag` has to offer — sending each we hold lets
/// the origin use whichever it actually implements rather than making us guess.
///
/// A named function rather than two lines inside [`Fetcher::send`], because `send` opens a
/// socket and nothing in this repository may: the address guard refuses loopback, so a rule
/// written in there is one a mutation deleting it survives. This is one call from an assertion.
fn asking_whether_it_changed(held: &crate::cache::Stale) -> Vec<(&'static str, String)> {
    let mut headers = Vec::new();
    if let Some(etag) = &held.etag {
        headers.push(("if-none-match", etag.clone()));
    }
    if let Some(since) = &held.last_modified {
        headers.push(("if-modified-since", since.clone()));
    }
    headers
}

/// The page a `304` confirms, stamped with the moment it was confirmed.
///
/// **A revalidated page is as of now; a merely unexpired one is not.** That distinction is the
/// whole difference between this and a cache hit: nobody asked about a page still inside its
/// hour, so its `fetched_at` stands and a claim drawn from it dates to the fetch. This one *was*
/// asked about, and the origin said the bytes are current — so the claim is true as of the
/// moment of confirmation, and saying otherwise would understate what is known.
///
/// **The freshness it was stored under comes back with it**, because a `304` may not repeat it
/// and reading the `304` alone turns an origin's `max-age=30` into our hour — see
/// [`crate::cache::Cache::insert_revalidated`]. Returned as a pair rather than looked up again
/// at the call site, so there is no default for the caller to reach for.
///
/// # Errors
/// When nothing was held. A `304` for a request that carried no validator is an origin answering
/// a question nobody asked, and there is no stored body to answer it with.
fn confirmed(
    held: Option<&crate::cache::Stale>,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<(Page, Duration), FetchError> {
    let held = held.ok_or_else(|| {
        FetchError::Transport("304 for a request that carried no validator".into())
    })?;
    Ok((
        Page {
            fetched_at: at,
            ..held.page.clone()
        },
        held.fresh_for,
    ))
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Every field line of a list-based field, combined as HTTP requires.
///
/// **Review found this, and it is a hole with a `no-store` in it.** `Cache-Control` may arrive
/// as several field lines, and their values are defined to combine as if written on one line
/// separated by commas. Reading only the first turns
///
/// ```text
/// Cache-Control: public
/// Cache-Control: no-store
/// ```
///
/// into a bare `public`, and the instruction the origin actually gave is gone before
/// [`crate::cache::storable`] ever sees it. Splitting on commas downstream is not a substitute:
/// the value that must be *found* was never passed along.
///
/// A value that is not readable text is reported as `no-store`. It may have *been* a `no-store`,
/// and the rule in this crate is that a header we cannot read is not permission.
fn combined(headers: &HeaderMap, name: &str) -> Option<String> {
    let mut values: Vec<String> = Vec::new();
    for value in headers.get_all(name) {
        let Ok(text) = value.to_str() else {
            return Some("no-store".to_owned());
        };
        values.push(text.to_owned());
    }
    (!values.is_empty()).then(|| values.join(", "))
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

    /// An allowance nothing in this module is trying to exhaust.
    ///
    /// Named rather than defaulted so a test about the budget stands out from a test that just
    /// needs one — `budget::tests` is where the counting itself is asserted.
    fn plenty() -> crate::Budget {
        crate::Budget::for_one_analysis()
    }

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

        let served = fetcher
            .get(url, &plenty())
            .await
            .expect("served from memory");
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
        let served = fetcher
            .get(url, &plenty())
            .await
            .expect("served from memory");
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
        let refused = fetcher.get("http://127.0.0.1:9/private", &plenty()).await;
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
        let (pages, bytes) = fetcher.cached();
        assert_eq!(pages, 1);
        // Bodies **and** what it costs to hold them: keys, headers, entry overhead. A budget
        // counting only bodies bounded nothing, which review found by filling it with empty
        // responses — so the figure here has to be more than the body, not equal to it.
        assert!(
            bytes > "1234".len(),
            "the body was counted and the rest of the entry was not: {bytes}"
        );
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
            other.get("http://127.0.0.1:9/a", &plenty()).await.is_err(),
            "a second fetcher was served the first one's memory"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod reading_what_the_origin_actually_said {
    //! Header reading, over a `HeaderMap` a test can build rather than a socket it cannot.

    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn lines(name: &str, values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let name: HeaderName = name.parse().unwrap();
        for value in values {
            // `append`, not `insert`: this is the shape being tested.
            headers.append(&name, HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn a_directive_on_a_second_field_line_is_not_lost() {
        // **Review's finding.** `HeaderMap::get` returns the first value, so this response read
        // as a bare `public` and was cached in defiance of the line underneath it.
        let headers = lines("cache-control", &["public", "no-store"]);
        assert_eq!(
            header(&headers, "cache-control").as_deref(),
            Some("public"),
            "the single-value reader is what made this possible; it still does that"
        );
        assert_eq!(
            combined(&headers, "cache-control").as_deref(),
            Some("public, no-store")
        );

        // And the consequence, through the same reader `Fetcher::get` uses — which is why that
        // reader is a type here rather than four lines inside a function no test can drive.
        let now = "2026-08-09T00:00:00Z".parse().unwrap();
        let said = Said::read(&headers);
        assert_eq!(
            crate::cache::storable(200, said.freshness(), now),
            crate::cache::Storable::No,
            "a `no-store` on the second field line was cached anyway"
        );
    }

    #[test]
    fn the_single_valued_fields_are_read_as_themselves() {
        // The other half of `Said::read`: joining these would turn two `Date` lines from a
        // confused proxy into one unparseable string, and an unparseable `Date` refuses to cache
        // at all. Only the list-based field is combined.
        let mut headers = lines(
            "date",
            &[
                "Sat, 08 Aug 2026 23:50:00 GMT",
                "Sat, 08 Aug 2026 22:00:00 GMT",
            ],
        );
        headers.append(
            HeaderName::from_static("age"),
            HeaderValue::from_static("120"),
        );

        let said = Said::read(&headers);
        assert_eq!(
            said.date.as_deref(),
            Some("Sat, 08 Aug 2026 23:50:00 GMT"),
            "two instants were combined into a string that is neither"
        );
        assert_eq!(said.age.as_deref(), Some("120"));
        assert_eq!(said.cache_control, None);

        // And the response is still cacheable, which is the point: a malformed duplicate should
        // not silently cost every page on that origin its place in the cache. Ten minutes old by
        // `Date`, two by `Age`, and the larger wins.
        let now = "2026-08-09T00:00:00Z".parse().unwrap();
        assert_eq!(
            crate::cache::storable(200, said.freshness(), now),
            crate::cache::Storable::For(Duration::from_secs(3600 - 600)),
            "a duplicated `Date` was handled as unreadable"
        );
    }

    #[test]
    fn a_field_we_cannot_read_is_not_permission() {
        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap(),
        );
        assert_eq!(
            combined(&headers, "cache-control").as_deref(),
            Some("no-store"),
            "a value we could not read was treated as though nothing had been said"
        );
    }

    #[test]
    fn a_field_nobody_sent_stays_absent() {
        // Not the same as an empty one: absent means the origin said nothing, and saying nothing
        // is what `FRESH_FOR` exists for.
        assert_eq!(combined(&HeaderMap::new(), "cache-control"), None);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// An allowance nothing in this module is trying to exhaust.
    ///
    /// Named rather than defaulted so a test about the budget stands out from a test that just
    /// needs one — `budget::tests` is where the counting itself is asserted.
    fn plenty() -> crate::Budget {
        crate::Budget::for_one_analysis()
    }

    fn target(url: &str) -> Target {
        Target::parse(url).expect("test url parses")
    }

    fn held(url: &str, body: &str) -> crate::cache::Stale {
        crate::cache::Stale {
            fresh_for: std::time::Duration::from_secs(30),
            page: Page {
                url: url.to_owned(),
                status: 200,
                body: body.to_owned(),
                etag: Some("\"v1\"".to_owned()),
                last_modified: None,
                fetched_at: "2026-08-09T09:00:00Z".parse().unwrap(),
            },
            etag: Some("\"v1\"".to_owned()),
            last_modified: None,
        }
    }

    #[test]
    fn what_we_hold_is_what_the_origin_is_asked_about() {
        // Read but never sent is the shape this fails in: the cache does its half, the request
        // goes out unconditional, and every re-fetch is a full download that was supposed to be
        // a `304`. Nothing about it is visible in the answer.
        let both = crate::cache::Stale {
            last_modified: Some("Sat, 08 Aug 2026 23:00:00 GMT".to_owned()),
            ..held("https://a.example/", "x")
        };
        assert_eq!(
            asking_whether_it_changed(&both),
            vec![
                ("if-none-match", "\"v1\"".to_owned()),
                (
                    "if-modified-since",
                    "Sat, 08 Aug 2026 23:00:00 GMT".to_owned()
                ),
            ]
        );

        // An origin that offers only a date is asked with a date. Sending nothing here would
        // quietly turn every such origin's pages back into full downloads.
        let dated = crate::cache::Stale {
            etag: None,
            last_modified: Some("Sat, 08 Aug 2026 23:00:00 GMT".to_owned()),
            ..held("https://a.example/", "x")
        };
        assert_eq!(
            asking_whether_it_changed(&dated),
            vec![(
                "if-modified-since",
                "Sat, 08 Aug 2026 23:00:00 GMT".to_owned()
            )]
        );

        let tagged = held("https://a.example/", "x");
        assert_eq!(
            asking_whether_it_changed(&tagged),
            vec![("if-none-match", "\"v1\"".to_owned())]
        );
    }

    #[test]
    fn a_304_is_not_a_redirect_with_nowhere_to_go() {
        // **The whole reason this is a type and not two `if`s in a particular order.** `304` is
        // a `3xx`: a redirect branch written first swallows it, and the cheapest answer an
        // origin can give becomes a transport error — a conditional GET worse than none.
        assert_eq!(answer_to(304), Answer::NotModified);
        for redirect in [301, 302, 303, 307, 308] {
            assert_eq!(answer_to(redirect), Answer::Redirect, "{redirect}");
        }
        // Failures are bodies with a status: a 404 page is read, and a 503 is reported.
        for body in [200, 204, 404, 410, 500, 503] {
            assert_eq!(answer_to(body), Answer::Body, "{body}");
        }
    }

    #[test]
    fn a_confirmed_page_is_the_body_we_held_and_the_moment_it_was_confirmed() {
        // **A revalidated page is as of now; a merely unexpired one is not.** Nobody asked about
        // a page still inside its hour, so its `fetched_at` stands. This one was asked about and
        // the origin said the bytes are current, so a claim drawn from it is true as of now —
        // and saying otherwise would understate what is known.
        let stale = held(
            "https://a.example/pricing",
            "# Pricing
Pro $10",
        );
        let at: chrono::DateTime<chrono::Utc> = "2026-08-09T12:00:00Z".parse().unwrap();

        let (page, stored_for) = confirmed(Some(&stale), at).expect("a held page is confirmable");
        assert_eq!(
            stored_for, stale.fresh_for,
            "the policy it was kept under did not travel with it"
        );
        assert_eq!(
            page.body,
            "# Pricing
Pro $10",
            "the held body is the answer"
        );
        assert_eq!(page.url, "https://a.example/pricing");
        assert_eq!(page.etag.as_deref(), Some("\"v1\""));
        assert_eq!(
            page.fetched_at, at,
            "a revalidated page still dates to the fetch"
        );
        assert_ne!(page.fetched_at, stale.page.fetched_at);
    }

    #[test]
    fn a_304_for_a_question_nobody_asked_is_a_failure_rather_than_a_page() {
        // There is no stored body to answer with, so the only honest thing is an error. Handing
        // back an empty page would put a company on a report with nothing under it.
        let at = chrono::Utc::now();
        assert!(confirmed(None, at).is_err());
    }

    #[test]
    fn validators_never_travel_past_the_first_hop() {
        // They are the origin's proof about *the URL the caller asked for*. A redirect lands
        // somewhere else, and asking that somewhere else whether it matches another page's
        // `ETag` is a question about nothing — which an origin may answer `304`, handing back
        // one page's body under another page's URL.
        let stale = held("https://a.example/pricing", "# Pricing");
        assert!(conditional_on(0, Some(&stale)).is_some());
        for hop in 1..=MAX_REDIRECTS {
            assert!(
                conditional_on(hop, Some(&stale)).is_none(),
                "hop {hop} carried a validator for a different URL"
            );
        }
        assert!(
            conditional_on(0, None).is_none(),
            "nothing held, nothing asked"
        );
    }

    #[tokio::test]
    async fn a_run_that_has_spent_its_allowance_says_so_rather_than_blaming_the_site() {
        // **The budget is checked before the request is built**, so this reaches no network at
        // all: the address is a literal, so the guard approves it without DNS, and the first
        // thing `send` does is ask the allowance. Reported as our own bound rather than as
        // `robots.txt says no`, which is what it looked like before `rules_for` learned to
        // propagate this one error and swallow the rest.
        let fetcher = Fetcher::new();
        let spent = crate::Budget::of(0);
        let refused = fetcher.get("http://93.184.216.34/pricing", &spent).await;

        assert!(
            matches!(refused, Err(FetchError::BudgetSpent { limit: 0 })),
            "a spent allowance reported as {refused:?}"
        );
        assert_eq!(spent.spent(), 0, "a refusal was counted as a request");
    }

    #[tokio::test]
    async fn robots_txt_comes_out_of_the_allowance_like_any_other_request() {
        // A bound that counted only the pages a caller named would bound the thing nobody was
        // worried about: a run reaching a hundred hosts through search fetches a hundred
        // `robots.txt` whether or not it reads a page on any of them.
        //
        // Asked of `rules_for` directly, because the property is about *where the allowance is
        // spent* rather than about what a run does with the answer — and because nothing here
        // may reach a network. The address is a literal, so the guard approves it without DNS,
        // and the allowance refuses before a request is built.
        let fetcher = Fetcher::new();
        let target = Target::parse("http://93.184.216.34/pricing").expect("a literal address");
        let spent = crate::Budget::of(0);

        let refused = fetcher.rules_for(&target, &spent).await;
        assert!(
            matches!(refused, Err(FetchError::BudgetSpent { limit: 0 })),
            "reading robots.txt did not go through the allowance: {refused:?}"
        );
    }

    #[tokio::test]
    async fn being_out_of_allowance_is_never_reported_as_the_site_refusing() {
        // Every other failure to read `robots.txt` becomes `Rules::restrictive` — *unreachable
        // is not permission*. This one must not: blaming a stranger for our own bound would put
        // a wrong sentence on a report, and *"the site asks crawlers not to"* is a sentence a
        // reader would act on.
        let fetcher = Fetcher::new();
        let spent = crate::Budget::of(0);
        let refused = fetcher.get("http://93.184.216.34/pricing", &spent).await;
        assert!(
            !matches!(refused, Err(FetchError::RobotsDisallowed { .. })),
            "our own bound was reported as the site's wishes"
        );
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
            match f.get(url, &plenty()).await {
                Err(FetchError::Refused(_)) => {}
                other => panic!("{url} should have been refused, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_scheme_we_do_not_fetch_never_reaches_the_network() {
        let f = Fetcher::new();
        assert!(matches!(
            f.get("file:///etc/passwd", &plenty()).await,
            Err(FetchError::UnsupportedScheme { .. })
        ));
    }
}
