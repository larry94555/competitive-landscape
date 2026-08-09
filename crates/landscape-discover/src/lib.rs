//! Finding the pages worth reading about a company we have already identified.
//!
//! `FACT_CHECKING.md` §3.3's pipeline, in order:
//!
//! ```text
//! resolve entity → canonical domain
//!    ├─ structured probes   deterministic, cheap, primary sources
//!    ├─ sitemap.xml         what exists, at paths nobody would guess
//!    ├─ llms.txt            what the site says is worth reading
//!    └─ admission control → rank → cap at 8
//! ```
//!
//! **Search is deliberately absent.** §3.3 puts probes first and is explicit that *"search
//! fills gaps; it does not lead"*. Search also needs a self-hosted SearXNG, which is
//! infrastructure this project does not have yet — so building the deterministic half first
//! is both the specified order and the only one available.
//!
//! # Everything here is a Primary source
//!
//! Every URL this produces is on the subject's own domain, which is
//! [`Disposition::Primary`] — the only class permitted to set a value in a comparison table
//! ([`Disposition::may_set_a_table_value`]). That is the strongest reason to exhaust probes
//! before search: search returns other people's pages, and other people's pages can be
//! reported but never tabulated.
//!
//! # What it costs
//!
//! Politeness is one request per host per second, so **discovery is dominated by its own
//! rate limit**: fourteen probes plus a sitemap is fifteen-odd seconds before a single page
//! has been read for content. Against a 90–180s budget that is a real share, and it is why
//! [`probes::in_order`] is prioritised and why [`Discovered::stopped_early`] exists.
//!
//! [`Disposition::Primary`]: landscape_core::Disposition::Primary
//! [`Disposition::may_set_a_table_value`]: landscape_core::Disposition::may_set_a_table_value

pub mod listings;
pub mod locale;
pub mod probes;
pub mod rank;

use landscape_fetch::{FetchError, Fetcher};
use rank::{Candidate, Via};

/// One path we tried, and what came back.
///
/// The outcome is carried, not just the URL. `PRODUCT_SPEC.md` §4's coverage note reads
/// *"/changelog (404), /releases (404), blog (90d)"* — **the numbers are the note**. A list of
/// paths with no outcomes beside them says we typed some URLs; a list with outcomes says what
/// the company does and does not publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub url: String,
    pub outcome: Outcome,
}

/// What a tried path answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 200, and the page exists.
    Answered,
    /// A status that is not 200. The number is kept because `404` and `403` mean different
    /// things to a reader deciding whether to look themselves.
    Status(u16),
    /// `robots.txt` said no. **Not a failure** — the system working, and a fact about the
    /// company rather than about us.
    Disallowed,
    /// The request did not complete.
    Unreachable,
    /// **We did not ask.** This run had already spent its allowance of requests to strangers —
    /// see `landscape_fetch::budget`. Kept apart from [`Self::Unreachable`] because a reader
    /// deciding whether to look themselves acts differently on *"their server did not answer"*
    /// and *"we stopped asking"*, and collapsing the two would blame a site for our own bound.
    NotAsked,
}

impl Outcome {
    /// How it reads in a coverage note.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Answered => "200".to_owned(),
            Self::Status(code) => code.to_string(),
            Self::Disallowed => "robots".to_owned(),
            Self::Unreachable => "unreachable".to_owned(),
            Self::NotAsked => "not asked".to_owned(),
        }
    }
}

/// What discovery found, and what it cost.
#[derive(Debug, Clone, Default)]
pub struct Discovered {
    /// The admitted pages, capped and ordered.
    pub sources: Vec<Candidate>,
    /// Every path tried, so a thin result is falsifiable rather than mysterious.
    ///
    /// This is the same discipline as `Section::not_found`: a negative nobody can check is
    /// not a finding. A reader who sees two sources should be able to see that eleven
    /// other paths were tried and answered nothing.
    pub checked: Vec<Checked>,
    /// True when the probe budget ran out before the list did.
    pub stopped_early: bool,
}

impl Discovered {
    /// A summary a person can read.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(out, "\n{:<58} answers      found via", "source");
        let rule = "-".repeat(88);
        let _ = writeln!(out, "{rule}");
        for c in &self.sources {
            let _ = writeln!(
                out,
                "{:<58} {:<12} {}",
                trim(&c.url, 56),
                c.answers.name(),
                c.via.name()
            );
        }
        let _ = writeln!(out, "{rule}");
        let _ = writeln!(
            out,
            "{} source(s) admitted from {} path(s) checked",
            self.sources.len(),
            self.checked.len()
        );
        if self.stopped_early {
            let _ = writeln!(
                out,
                "probe budget reached; lower-priority paths were not tried"
            );
        }
        if self.sources.is_empty() {
            let _ = writeln!(
                out,
                "\nNothing found. That is a finding, not an error - every path above was tried."
            );
        }
        out
    }
}

impl Discovered {
    /// What was tried on behalf of one question, as a coverage note repeats it.
    ///
    /// A path is attributed to the question its *shape* suggests, using the same classifier
    /// that admits pages — so `/changelog (404)` appears under *changes* whether it answered
    /// or not. `/llms.txt` and `/sitemap.xml` belong to no question and appear under none;
    /// they are listed once, for the run.
    #[must_use]
    pub fn attempts_for(&self, question: probes::Answers) -> Vec<landscape_core::Attempt> {
        self.checked
            .iter()
            .filter(|c| probes::guess(path_of(&c.url)) == Some(question))
            .map(|c| landscape_core::Attempt {
                path: path_of(&c.url).to_owned(),
                outcome: c.outcome.name(),
                // From the URL that was actually tried, so it cannot disagree with the path
                // beside it. Two companies both contribute `/pricing (404)`, and merged they
                // are one indistinguishable list without this.
                subject: origin_of(&c.url),
            })
            .collect()
    }

    /// The coverage of one question: what was admitted, what was tried, and what came out.
    ///
    /// `pages_read` and `facts` are the caller's to supply — discovery finds pages and does
    /// not open them, and the difference between *found* and *read* is one of the four
    /// silences [`landscape_core::Coverage::note`] distinguishes.
    #[must_use]
    pub fn coverage(
        &self,
        question: probes::Answers,
        pages_read: usize,
        facts: usize,
    ) -> landscape_core::Coverage {
        landscape_core::Coverage {
            question: question.name().to_owned(),
            sources: self
                .sources
                .iter()
                .filter(|s| s.answers == question)
                .map(|s| s.url.clone())
                .collect(),
            pages_read,
            facts,
            attempts: self.attempts_for(question),
        }
    }
}

/// Scheme and host of a URL — the company a path was tried against.
fn origin_of(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

/// The path part of a URL, for a note a reader can retype.
fn path_of(url: &str) -> &str {
    let after_scheme = url.find("://").map_or(0, |i| i + 3);
    url[after_scheme..]
        .find('/')
        .map_or("/", |i| &url[after_scheme + i..])
}

/// How many probes to spend before giving up on guessing.
///
/// The whole list is 14, and a site that answered nothing on the first several is a site
/// whose paths we are not going to guess. Stopping is cheaper than being thorough about a
/// bad hypothesis, and the sitemap is the better route on such a site anyway.
pub const PROBE_BUDGET: usize = 14;

/// Run discovery against one canonical domain.
///
/// `origin` is a scheme and host — `https://example.com`.
pub async fn discover(
    fetcher: &Fetcher,
    budget: &landscape_fetch::Budget,
    origin: &str,
) -> Discovered {
    let origin = origin.trim_end_matches('/').to_owned();
    let mut found: Vec<Candidate> = Vec::new();
    let mut checked: Vec<Checked> = Vec::new();

    // 1. What the site says is worth reading. One request, and the best evidence there is.
    let llms_url = format!("{origin}/llms.txt");
    let llms = fetcher.get(&llms_url, budget).await;
    checked.push(Checked {
        url: llms_url.clone(),
        outcome: outcome_of(&llms),
    });
    if let Ok(page) = llms {
        if page.status == 200 {
            for listed in listings::from_llms_txt(&page.body, &origin) {
                found.push(Candidate {
                    url: listed.url,
                    answers: listed.answers,
                    via: Via::LlmsTxt,
                });
            }
        }
    }

    // 2. What exists. Reaches pages no probe would guess.
    for listed in sitemap_urls(fetcher, budget, &origin, &mut checked).await {
        found.push(Candidate {
            url: listed.url,
            answers: listed.answers,
            via: Via::Sitemap,
        });
    }

    // 3. The guesses, cheapest and most valuable first.
    let mut stopped_early = false;
    for (n, probe) in probes::in_order().into_iter().enumerate() {
        if n >= PROBE_BUDGET {
            stopped_early = true;
            break;
        }
        let url = format!("{origin}{}", probe.path);
        let response = fetcher.get(&url, budget).await;
        checked.push(Checked {
            url: url.clone(),
            outcome: outcome_of(&response),
        });
        // A 200 means the path exists. Anything else — including a 404 dressed as a soft
        // landing page — is not evidence that it does. A refusal is not a failure of
        // discovery either: robots.txt saying no is the system working, and the URL stays in
        // `checked` with the reason so a reader can see it was tried.
        if matches!(&response, Ok(page) if page.status == 200) {
            found.push(Candidate {
                url,
                answers: probe.answers,
                via: Via::Probe,
            });
        }
    }

    // Everything admitted must be on the subject's own site. `FACT_CHECKING.md` §3.3 is the
    // reason: a page on the subject's domain is `Disposition::Primary`, and **only a primary
    // source may set a value in a comparison table**. An `llms.txt` can list anything, and
    // notion.com's lists `linkedin.com/company/notionhq` — which took the identity slot for
    // that company, could not be fetched, and would have been a table value from somebody
    // else's site if it had been.
    //
    // Off-site links are not worthless; they are a different class of evidence, and admitting
    // them here would launder them into the class that outranks everything.
    let host = host_of(&origin).to_owned();
    found.retain(|c| same_site(&host, &c.url));

    Discovered {
        sources: rank::admit(found, rank::CAP_RUNG_0),
        checked,
        stopped_early,
    }
}

/// The host part of an origin or URL, without `www.`.
fn host_of(url: &str) -> &str {
    let after_scheme = url.find("://").map_or(0, |i| i + 3);
    let rest = &url[after_scheme..];
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.strip_prefix("www.").unwrap_or(host)
}

/// Whether a URL is on the subject's own site, subdomains included.
///
/// `docs.example.com` is example.com's documentation and `example.com.evil.test` is not, so
/// the match is on a dot-bounded suffix rather than `contains`.
fn same_site(host: &str, url: &str) -> bool {
    let candidate = host_of(url);
    candidate == host || candidate.ends_with(&format!(".{host}"))
}

/// What one fetch says about the path, for the coverage note.
fn outcome_of(response: &Result<landscape_fetch::Page, FetchError>) -> Outcome {
    match response {
        Ok(page) if page.status == 200 => Outcome::Answered,
        Ok(page) => Outcome::Status(page.status),
        Err(FetchError::RobotsDisallowed { .. } | FetchError::Refused(_)) => Outcome::Disallowed,
        Err(FetchError::BudgetSpent { .. }) => Outcome::NotAsked,
        Err(_) => Outcome::Unreachable,
    }
}

/// Sitemap URLs, following one level of index.
async fn sitemap_urls(
    fetcher: &Fetcher,
    budget: &landscape_fetch::Budget,
    origin: &str,
    checked: &mut Vec<Checked>,
) -> Vec<listings::Listed> {
    let root = format!("{origin}/sitemap.xml");
    let response = fetcher.get(&root, budget).await;
    checked.push(Checked {
        url: root.clone(),
        outcome: outcome_of(&response),
    });

    let Ok(page) = response else {
        return Vec::new();
    };
    if page.status != 200 {
        return Vec::new();
    }

    let nested = listings::nested_sitemaps(&page.body);
    if nested.is_empty() {
        return listings::from_sitemap(&page.body, origin);
    }

    // An index. Read a couple of the children rather than all of them — each is a request,
    // and a site that splits its sitemap into twenty parts is not one whose pricing page
    // we will find by reading all twenty.
    let mut out = Vec::new();
    for url in nested.into_iter().take(2) {
        let child = fetcher.get(&url, budget).await;
        checked.push(Checked {
            url,
            outcome: outcome_of(&child),
        });
        if let Ok(child) = child {
            if child.status == 200 {
                out.extend(listings::from_sitemap(&child.body, origin));
            }
        }
    }
    out
}

fn trim(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::probes::Answers;

    fn checked(url: &str, outcome: Outcome) -> Checked {
        Checked {
            url: url.to_owned(),
            outcome,
        }
    }

    #[test]
    fn a_host_is_compared_without_its_www() {
        assert!(same_site("notion.com", "https://www.notion.com/about"));
        assert!(same_site("notion.com", "https://docs.notion.com/x"));
    }

    #[test]
    fn somebody_elses_site_is_not_a_primary_source() {
        // notion.com's llms.txt lists linkedin.com/company/notionhq, and it took the identity
        // slot for that company. A page we do not control is a different class of evidence,
        // and admitting it here would launder it into the class that outranks everything.
        assert!(!same_site(
            "notion.com",
            "https://www.linkedin.com/company/notionhq"
        ));
        // And a lookalike domain is not a subdomain.
        assert!(!same_site(
            "notion.com",
            "https://notion.com.evil.test/about"
        ));
    }

    #[test]
    fn what_was_tried_is_attributed_to_the_question_it_was_tried_for() {
        let d = Discovered {
            sources: vec![],
            checked: vec![
                checked("https://e.com/changelog", Outcome::Status(404)),
                checked("https://e.com/releases", Outcome::Status(404)),
                checked("https://e.com/pricing", Outcome::Answered),
                checked("https://e.com/llms.txt", Outcome::Status(404)),
            ],
            stopped_early: false,
        };
        let changes = d.attempts_for(Answers::Changes);
        assert_eq!(changes.len(), 2, "{changes:?}");
        assert_eq!(changes[0].path, "/changelog");
        assert_eq!(changes[0].outcome, "404");
        // `/llms.txt` answers no question, so it is attributed to none of them.
        assert_eq!(d.attempts_for(Answers::Pricing).len(), 1);
    }

    #[test]
    fn a_question_with_no_facts_carries_the_paths_that_were_tried() {
        // The whole point: an empty section a reader can check, rather than one they have to
        // trust. PRODUCT_SPEC §4 — "Not 'no changes.'"
        let d = Discovered {
            sources: vec![],
            checked: vec![
                checked("https://e.com/changelog", Outcome::Status(404)),
                checked("https://e.com/releases", Outcome::Disallowed),
            ],
            stopped_early: false,
        };
        let coverage = d.coverage(Answers::Changes, 0, 0);
        assert!(coverage.is_empty());
        let note = coverage.note();
        assert!(note.contains("/changelog (404)"), "{note}");
        assert!(note.contains("/releases (robots)"), "{note}");
    }

    #[test]
    fn a_page_that_was_read_and_yielded_nothing_reads_differently() {
        let d = Discovered {
            sources: vec![Candidate {
                url: "https://e.com/releases".to_owned(),
                answers: Answers::Changes,
                via: Via::Probe,
            }],
            checked: vec![checked("https://e.com/releases", Outcome::Answered)],
            stopped_early: false,
        };
        let note = d.coverage(Answers::Changes, 1, 0).note();
        assert!(note.contains("read 1 page(s)"), "{note}");
    }

    #[test]
    fn a_thin_result_still_reports_what_was_tried() {
        // The same discipline as a "not found" report section: a negative nobody can check
        // is not a finding. Two sources out of fifteen paths is informative; two sources
        // out of nowhere is just disappointing.
        let d = Discovered {
            sources: vec![],
            checked: vec![
                Checked {
                    url: "https://e.com/pricing".into(),
                    outcome: Outcome::Status(404),
                },
                Checked {
                    url: "https://e.com/plans".into(),
                    outcome: Outcome::Status(404),
                },
            ],
            stopped_early: false,
        };
        let rendered = d.render();
        assert!(rendered.contains("2 path(s) checked"));
        assert!(rendered.contains("That is a finding, not an error"));
    }

    #[test]
    fn the_render_names_how_each_source_was_found() {
        // Provenance is the difference between "the site told us" and "we guessed and it
        // answered", and a reader deciding how much to trust a thin report needs it.
        let d = Discovered {
            sources: vec![Candidate {
                url: "https://e.com/pricing".into(),
                answers: Answers::Pricing,
                via: Via::LlmsTxt,
            }],
            checked: vec![checked("https://e.com/pricing", Outcome::Answered)],
            stopped_early: false,
        };
        let rendered = d.render();
        assert!(rendered.contains("llms.txt"));
        assert!(rendered.contains("pricing"));
    }

    #[test]
    fn stopping_early_is_stated_rather_than_hidden() {
        let d = Discovered {
            sources: vec![],
            checked: vec![checked("https://e.com/pricing", Outcome::Answered)],
            stopped_early: true,
        };
        assert!(d.render().contains("probe budget reached"));
    }

    #[test]
    fn a_page_we_did_not_ask_for_is_not_a_site_that_did_not_answer() {
        // **Four outcomes, four different things a reader does next.** *"Unreachable"* sends
        // somebody to check a site that is fine; *"not asked"* tells them the run stopped, which
        // is our doing and is fixable by running it again. Collapsing the two would blame a
        // stranger for our own bound — the same distinction `Disallowed` is kept apart for.
        let spent = outcome_of(&Err(FetchError::BudgetSpent { limit: 64 }));
        assert_eq!(spent, Outcome::NotAsked);
        assert_eq!(spent.name(), "not asked");

        assert_eq!(
            outcome_of(&Err(FetchError::Transport("connection refused".into()))),
            Outcome::Unreachable,
            "a site that really did not answer must still say so"
        );
        assert_eq!(
            outcome_of(&Err(FetchError::RobotsDisallowed {
                host: "e.com".to_owned(),
                path: "/pricing".to_owned(),
            })),
            Outcome::Disallowed
        );
    }

    #[test]
    fn the_probe_budget_covers_the_whole_list() {
        // If the budget were smaller than the list, the lowest-priority probes would never
        // run at all and their presence in the list would be decoration.
        assert!(PROBE_BUDGET >= probes::PROBES.len());
    }
}
