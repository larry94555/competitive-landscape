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
    pub checked: Vec<String>,
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

/// How many probes to spend before giving up on guessing.
///
/// The whole list is 14, and a site that answered nothing on the first several is a site
/// whose paths we are not going to guess. Stopping is cheaper than being thorough about a
/// bad hypothesis, and the sitemap is the better route on such a site anyway.
pub const PROBE_BUDGET: usize = 14;

/// Run discovery against one canonical domain.
///
/// `origin` is a scheme and host — `https://example.com`.
pub async fn discover(fetcher: &Fetcher, origin: &str) -> Discovered {
    let origin = origin.trim_end_matches('/').to_owned();
    let mut found: Vec<Candidate> = Vec::new();
    let mut checked: Vec<String> = Vec::new();

    // 1. What the site says is worth reading. One request, and the best evidence there is.
    let llms_url = format!("{origin}/llms.txt");
    checked.push(llms_url.clone());
    if let Ok(page) = fetcher.get(&llms_url).await {
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
    for listed in sitemap_urls(fetcher, &origin, &mut checked).await {
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
        checked.push(url.clone());
        match fetcher.get(&url).await {
            // A 200 means the path exists. Anything else — including a 404 dressed as a
            // soft landing page — is not evidence that it does.
            Ok(page) if page.status == 200 => found.push(Candidate {
                url,
                answers: probe.answers,
                via: Via::Probe,
            }),
            Ok(_) => {}
            // A refusal is not a failure of discovery. robots.txt saying no is the system
            // working, and the URL stays in `checked` so a reader can see it was tried.
            Err(FetchError::RobotsDisallowed { .. } | FetchError::Refused(_)) => {}
            Err(_) => {}
        }
    }

    Discovered {
        sources: rank::admit(found, rank::CAP_RUNG_0),
        checked,
        stopped_early,
    }
}

/// Sitemap URLs, following one level of index.
async fn sitemap_urls(
    fetcher: &Fetcher,
    origin: &str,
    checked: &mut Vec<String>,
) -> Vec<listings::Listed> {
    let root = format!("{origin}/sitemap.xml");
    checked.push(root.clone());

    let Ok(page) = fetcher.get(&root).await else {
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
        checked.push(url.clone());
        if let Ok(child) = fetcher.get(&url).await {
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

    #[test]
    fn a_thin_result_still_reports_what_was_tried() {
        // The same discipline as a "not found" report section: a negative nobody can check
        // is not a finding. Two sources out of fifteen paths is informative; two sources
        // out of nowhere is just disappointing.
        let d = Discovered {
            sources: vec![],
            checked: vec!["https://e.com/pricing".into(), "https://e.com/plans".into()],
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
            checked: vec!["https://e.com/pricing".into()],
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
            checked: vec!["https://e.com/pricing".into()],
            stopped_early: true,
        };
        assert!(d.render().contains("probe budget reached"));
    }

    #[test]
    fn the_probe_budget_covers_the_whole_list() {
        // If the budget were smaller than the list, the lowest-priority probes would never
        // run at all and their presence in the list would be decoration.
        assert!(PROBE_BUDGET >= probes::PROBES.len());
    }
}
