//! Fetching public web pages, politely and without being turned into a weapon.
//!
//! This is the first code in the project that talks to an address a stranger chose, which
//! makes it the first code with an attacker. Two concerns run through it:
//!
//! - **[`guard`]** — we must not be usable as a way to reach things inside our own network.
//! - **[`robots`]** — we must not fetch what a site asked us not to. `FACT_CHECKING.md`
//!   treats that as an ethical commitment rather than a risk position, which is why the
//!   parser errs toward *disallow* and a site returning 5xx is left alone.
//!
//! # The order of operations is the design
//!
//! ```text
//! parse URL  →  scheme + port  →  resolve  →  guard every address  →  pin the connection
//!            →  robots.txt (itself guarded and pinned)  →  per-host delay  →  GET
//!            →  size cap  →  redirect? start again from the top
//! ```
//!
//! **Every redirect re-enters at the top.** A URL that passes every check and then redirects
//! to `169.254.169.254` has defeated a guard that only ran once, so automatic redirect
//! following is switched off and each hop is checked as if a stranger had just typed it.

pub mod fetcher;
pub mod guard;
pub mod limits;
pub mod robots;

pub use fetcher::Fetcher;

use std::net::IpAddr;
use std::time::Duration;

use guard::Refusal;

/// How we introduce ourselves.
///
/// A real name and a URL, because a site owner who wants to block us should be able to find
/// out what we are first. Pretending to be a browser would make the `robots.txt` commitment
/// meaningless — we would be asking permission while disguising who is asking.
pub const USER_AGENT: &str = concat!(
    "LandscapeBot/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/larry94555/competitive-landscape)"
);

/// The most we will read from one page.
///
/// A page larger than this is not a pricing page. The cap exists because the alternative is
/// letting a stranger decide how much of our memory to fill — and it is enforced while
/// streaming, not after, so a 5 GB response is abandoned rather than buffered.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Wall clock for one page, including connection and body.
pub const TIMEOUT: Duration = Duration::from_secs(20);

/// How many redirects we will follow before concluding we are being led somewhere.
pub const MAX_REDIRECTS: usize = 5;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("that does not look like a web address")]
    NotAUrl,

    #[error("only http and https are fetched, not {scheme}")]
    UnsupportedScheme { scheme: String },

    #[error("{}", .0.message())]
    Refused(Refusal),

    #[error("that host does not resolve to any address")]
    NoAddress,

    #[error("{host} asks crawlers not to fetch {path}")]
    RobotsDisallowed { host: String, path: String },

    #[error("that page is larger than we read ({MAX_BYTES} bytes)")]
    TooLarge,

    #[error("more than {MAX_REDIRECTS} redirects")]
    TooManyRedirects,

    #[error("could not reach it: {0}")]
    Transport(String),
}

/// A page we were allowed to fetch, and did.
#[derive(Debug, Clone)]
pub struct Page {
    pub url: String,
    pub status: u16,
    pub body: String,
    /// For a conditional GET next time. Sending `If-None-Match` turns most re-fetches into
    /// a 304 with no body, which is politeness and a cache in the same header.
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

/// A URL that has been parsed and had its scheme and port approved.
///
/// Separate from the address check because they fail differently: this one is decidable
/// from the text alone, and telling a reader "we only fetch http and https" needs no DNS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub https: bool,
}

impl Target {
    /// # Errors
    /// If it is not a URL, or not one we fetch.
    pub fn parse(raw: &str) -> Result<Self, FetchError> {
        let raw = raw.trim();
        let (scheme, rest) = raw.split_once("://").ok_or(FetchError::NotAUrl)?;
        let scheme = scheme.to_lowercase();

        let https = match scheme.as_str() {
            "https" => true,
            "http" => false,
            // Named rather than lumped in with "not a URL": `file://` and `gopher://` are
            // deliberate probes, and the message should say we understood and declined.
            other => {
                return Err(FetchError::UnsupportedScheme {
                    scheme: other.to_owned(),
                })
            }
        };

        let (authority, path) = rest
            .split_once('/')
            .map_or((rest, "/".to_owned()), |(a, p)| (a, format!("/{p}")));
        // Credentials in a URL are a redirect trick as often as a convenience, and nothing
        // we fetch needs them.
        let authority = authority.rsplit('@').next().unwrap_or(authority);

        let (host, port) = match authority.rsplit_once(':') {
            // Not a port if it is inside an IPv6 literal.
            Some((h, p)) if !h.contains('[') || h.ends_with(']') => {
                let port = p.parse::<u16>().map_err(|_| FetchError::NotAUrl)?;
                (h, port)
            }
            _ => (authority, if https { 443 } else { 80 }),
        };

        let host = host.trim_matches(|c| c == '[' || c == ']').to_lowercase();
        if host.is_empty() {
            return Err(FetchError::NotAUrl);
        }

        Ok(Self {
            host,
            port,
            path,
            https,
        })
    }

    #[must_use]
    pub fn origin(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        format!("{scheme}://{}:{}", self.host, self.port)
    }

    #[must_use]
    pub fn url(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        let default = if self.https { 443 } else { 80 };
        if self.port == default {
            format!("{scheme}://{}{}", self.host, self.path)
        } else {
            format!("{scheme}://{}:{}{}", self.host, self.port, self.path)
        }
    }
}

/// Check every address a host resolves to.
///
/// **All of them, not the first.** A hostname with two `A` records — one public, one
/// `127.0.0.1` — is a deliberate attack, and a guard that checks whichever address came
/// back first is a coin flip. Returns the verified address to connect to.
///
/// # Errors
/// The first refusal, or [`FetchError::NoAddress`].
pub fn approve_all(addrs: &[IpAddr]) -> Result<IpAddr, FetchError> {
    let first = *addrs.first().ok_or(FetchError::NoAddress)?;
    for addr in addrs {
        guard::check(*addr).map_err(FetchError::Refused)?;
    }
    Ok(first)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_urls_parse() {
        let t = Target::parse("https://example.com/pricing").expect("parses");
        assert_eq!(t.host, "example.com");
        assert_eq!(t.port, 443);
        assert_eq!(t.path, "/pricing");
        assert!(t.https);
    }

    #[test]
    fn a_missing_path_becomes_root() {
        assert_eq!(
            Target::parse("https://example.com").expect("parses").path,
            "/"
        );
    }

    #[test]
    fn the_host_is_lowercased_so_one_site_is_one_host() {
        // Otherwise EXAMPLE.COM and example.com get separate rate limits and separate
        // robots caches, which is how a "polite" crawler doubles its request rate.
        assert_eq!(
            Target::parse("https://EXAMPLE.com/x").expect("parses").host,
            "example.com"
        );
    }

    #[test]
    fn schemes_we_do_not_fetch_are_named_rather_than_dismissed() {
        // file:// and gopher:// are probes, not typos. Saying we understood and declined is
        // more useful to an honest user and no more useful to a dishonest one.
        for raw in ["file:///etc/passwd", "gopher://x/", "ftp://x/"] {
            match Target::parse(raw) {
                Err(FetchError::UnsupportedScheme { .. }) => {}
                other => panic!("{raw} should be refused by scheme, got {other:?}"),
            }
        }
    }

    #[test]
    fn credentials_in_a_url_are_dropped() {
        // https://example.com@evil.test/ points at evil.test, and a reader skimming it sees
        // example.com. Taking the part after the last @ is what a browser does.
        let t = Target::parse("https://example.com@evil.test/x").expect("parses");
        assert_eq!(t.host, "evil.test");
    }

    #[test]
    fn explicit_ports_are_kept_and_ipv6_literals_survive() {
        let t = Target::parse("http://example.com:8080/x").expect("parses");
        assert_eq!(t.port, 8080);
        assert!(!t.https);

        let v6 = Target::parse("http://[2606:2800::1]:8080/x").expect("parses");
        assert_eq!(v6.host, "2606:2800::1");
        assert_eq!(v6.port, 8080);

        // No port: the colons belong to the address, not to a port number.
        let bare = Target::parse("https://[2606:2800::1]/x").expect("parses");
        assert_eq!(bare.host, "2606:2800::1");
        assert_eq!(bare.port, 443);
    }

    #[test]
    fn rubbish_is_refused() {
        for raw in [
            "",
            "example.com",
            "https://",
            "https://:443/",
            "https://x:notaport/",
        ] {
            assert!(Target::parse(raw).is_err(), "{raw:?} should not parse");
        }
    }

    #[test]
    fn the_url_round_trips_without_a_default_port() {
        assert_eq!(
            Target::parse("https://example.com:443/a")
                .expect("parses")
                .url(),
            "https://example.com/a"
        );
        assert_eq!(
            Target::parse("http://example.com:8080/a")
                .expect("parses")
                .url(),
            "http://example.com:8080/a"
        );
    }

    #[test]
    fn every_address_a_host_resolves_to_is_checked() {
        // The attack this exists for: one public A record and one pointing at loopback.
        // A guard that checked only the first would pass half the time.
        let public: IpAddr = "93.184.216.34".parse().expect("parses");
        let loopback: IpAddr = "127.0.0.1".parse().expect("parses");

        assert!(approve_all(&[public]).is_ok());
        assert!(matches!(
            approve_all(&[public, loopback]),
            Err(FetchError::Refused(Refusal::Loopback))
        ));
        // Order must not matter.
        assert!(matches!(
            approve_all(&[loopback, public]),
            Err(FetchError::Refused(Refusal::Loopback))
        ));
    }

    #[test]
    fn a_host_that_resolves_to_nothing_is_an_error_not_a_pass() {
        assert!(matches!(approve_all(&[]), Err(FetchError::NoAddress)));
    }

    #[test]
    fn our_user_agent_says_who_we_are_and_where_to_complain() {
        // Pretending to be a browser would make the robots.txt commitment meaningless: we
        // would be asking permission while disguising who is asking.
        assert!(USER_AGENT.starts_with("LandscapeBot/"));
        assert!(USER_AGENT.contains("https://"));
    }
}
