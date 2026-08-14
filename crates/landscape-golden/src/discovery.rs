//! The golden set for **discovery**: which companies a description resolves to.
//!
//! `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 2. There has been a golden set for
//! *extraction* since Phase 0 — ten subjects, frozen pages, scored against a model — and none
//! at all for the step before it. That is why four independent causes produced **Microsoft** and
//! **projectplusgame.com** for *"project management for a small design agency"* with 1026 tests
//! passing over them.
//!
//! # What a fixture is, and what it is not
//!
//! Each [`Market`] is a description, the hits three canned queries return, and the companies a
//! careful person expects. **The hits are written here rather than recorded from an engine**,
//! which is the whole point: they are deterministic, they run on a laptop with no `SEARX_URL`,
//! and they can be *shaped* to hold one failure mode each.
//!
//! **What that measures is the logic, not the world.** A fixture cannot tell you whether the
//! real engine returns these URLs for this prompt. It tells you what this pipeline does with a
//! result set of a known shape, which is the thing every change in that roadmap alters. The live
//! half — pointing the same prompts at a real engine — is [`crate::discovery`]'s `--ignored`
//! companion in `against_an_engine.rs`.
//!
//! **The expected lists are a judgment, written down where it can be argued with.** They are
//! not a fact about the market; they are one careful reading of it, in a table somebody can
//! disagree with by editing a line.
//!
//! # Why the fixtures look the way they do
//!
//! Each one isolates a cause from `PRODUCT_IDEA_RESULTS_LOGIC.md` §4, so a change can be scored
//! against the defect it claims to fix rather than against an average:
//!
//! | Fixture | The case it holds |
//! |---|---|
//! | `project-management-for-agencies` | The reported failure, whole: a suite, an impostor and a specialist |
//! | `one-product-many-urls` | One product arriving as four URL shapes — the corroboration trap |
//! | `keyword-impostor` | A domain that shares a word with the prompt and sells something else |
//! | `specialist-in-one-article` | The right answer, returned by exactly one query |
//! | `publisher-heavy` | A result set that is almost entirely review sites |

use landscape_search::candidates::{from_results, Found};
use landscape_search::Hit;

/// How many queries a description sends per round. Mirrors `candidates::IDEA_QUERIES`, and the
/// fixtures below carry exactly this many result sets.
pub const QUERIES: usize = 3;

/// One description, what the engine returns for it, and what a careful person expects.
#[derive(Debug, Clone)]
pub struct Market {
    /// Stable across wording changes; it ends up in the scorecard a person reads.
    pub id: &'static str,
    /// What a reader types.
    pub prompt: &'static str,
    /// One result list per query, in the order the queries are sent.
    pub results: Vec<Vec<Hit>>,
    /// The companies that belong in the answer, as the host a reader would recognize.
    ///
    /// **Recall is measured against this**, and it is deliberately short: a set of five that
    /// contains the three that matter is a good answer, and a set of five that contains none of
    /// them is the failure this exists to catch.
    pub expected: Vec<&'static str>,
    /// Hosts that must **not** be in the answer, with the reason each one is a trap.
    ///
    /// Kept apart from *"anything not expected"* because precision alone cannot tell a
    /// defensible extra from a nonsense one, and these are the nonsense ones.
    pub impostors: Vec<(&'static str, &'static str)>,
}

/// A hit, written the way an engine would return one.
fn hit(url: &str, title: &str) -> Hit {
    Hit {
        url: url.to_owned(),
        title: title.to_owned(),
        snippet: String::new(),
    }
}

/// The set.
///
/// **Five, not ten, and labeled as such.** The extraction set has ten because ten real pages
/// were frozen; these are hand-written result sets, and five that each isolate a cause are worth
/// more than ten that average them. The number should grow when a new failure mode is found —
/// which is the only reason to add one.
#[must_use]
pub fn markets() -> Vec<Market> {
    vec![
        Market {
            id: "project-management-for-agencies",
            prompt: "project management for a small design agency",
            results: vec![
                vec![
                    hit("https://www.microsoft.com/en-us/microsoft-365/project/project-management-software",
                        "Microsoft Project | Project Management Software"),
                    hit("https://asana.com/uses/design-teams", "Asana for Design Teams"),
                    hit("https://www.g2.com/categories/project-management",
                        "Best Project Management Software 2026 | G2"),
                    hit("https://projectplusgame.com/", "Project Plus - the board game about deadlines"),
                ],
                vec![
                    hit("https://www.microsoft.com/en-us/microsoft-365/teams/group-chat-software",
                        "Microsoft Teams | Group Chat Software"),
                    hit("https://asana.com/", "Asana - Manage your team's work"),
                    hit("https://www.capterra.com/project-management-software/",
                        "Project Management Software 2026 | Capterra"),
                    hit("https://projectplusgame.com/rules", "Project Plus - the rules"),
                ],
                vec![
                    hit("https://www.microsoft.com/en-gb/microsoft-365/project/project-management-software",
                        "Microsoft Project | Project Management Software"),
                    hit("https://www.notion.so/product/projects", "Notion Projects"),
                    hit("https://www.workamajig.com/", "Workamajig - project management for creative agencies"),
                ],
            ],
            // Workamajig is the specialist the prompt actually asks for and one query returned
            // it; Asana and Microsoft Project are defensible; the suite page is not the product.
            expected: vec!["asana.com", "microsoft.com", "workamajig.com"],
            impostors: vec![(
                "projectplusgame.com",
                "a board game; shares the word project and nothing else",
            )],
        },
        Market {
            id: "one-product-many-urls",
            prompt: "spreadsheet software for finance teams",
            results: vec![
                vec![
                    hit("https://www.microsoft.com/en-us/microsoft-365/excel", "Microsoft Excel"),
                    hit("https://www.google.com/sheets/about/", "Google Sheets"),
                ],
                vec![
                    hit("https://www.microsoft.com/en-us/microsoft-365/excel/pricing", "Excel pricing"),
                    hit("https://www.google.com/sheets/about/pricing", "Google Sheets pricing"),
                ],
                vec![
                    hit("https://www.microsoft.com/de-de/microsoft-365/excel", "Microsoft Excel"),
                    hit("https://www.airtable.com/", "Airtable"),
                ],
            ],
            expected: vec!["microsoft.com", "google.com", "airtable.com"],
            impostors: vec![],
        },
        Market {
            id: "keyword-impostor",
            prompt: "time tracking for consultants",
            results: vec![
                vec![
                    hit("https://www.harvestapp.com/", "Harvest - time tracking"),
                    hit("https://trackingtimemusic.com/", "TrackingTime - the drum machine"),
                ],
                vec![
                    hit("https://www.harvestapp.com/pricing", "Harvest pricing"),
                    hit("https://trackingtimemusic.com/kits", "TrackingTime kits"),
                ],
                vec![
                    hit("https://toggl.com/track/", "Toggl Track"),
                    hit("https://www.harvestapp.com/", "Harvest - time tracking"),
                ],
            ],
            expected: vec!["harvestapp.com", "toggl.com"],
            impostors: vec![(
                "trackingtimemusic.com",
                "a drum machine; shares tracking and time, sells neither",
            )],
        },
        Market {
            id: "specialist-in-one-article",
            prompt: "invoicing software for freelance translators",
            results: vec![
                vec![
                    hit("https://www.freshbooks.com/", "FreshBooks invoicing"),
                    hit("https://quickbooks.intuit.com/", "QuickBooks"),
                ],
                vec![
                    hit("https://www.freshbooks.com/invoice", "FreshBooks invoices"),
                    hit("https://quickbooks.intuit.com/invoicing/", "QuickBooks invoicing"),
                ],
                // The only query that found the thing the prompt actually asks for.
                vec![hit("https://www.protemos.com/", "Protemos - management for translation businesses")],
            ],
            // **Protemos is expected and today's pipeline cannot return it.** One query is below
            // CORROBORATION, so it scores 0.175 and is excluded as `Aside::Uncorroborated`. This
            // fixture exists to hold that number honest rather than to be passed today.
            expected: vec!["freshbooks.com", "intuit.com", "protemos.com"],
            impostors: vec![],
        },
        Market {
            id: "publisher-heavy",
            prompt: "customer support helpdesk software",
            results: vec![
                vec![
                    hit("https://www.g2.com/categories/help-desk", "Best Help Desk Software | G2"),
                    hit("https://www.capterra.com/help-desk-software/", "Help Desk Software | Capterra"),
                    hit("https://www.reddit.com/r/sysadmin/comments/helpdesk", "Which helpdesk?"),
                    hit("https://www.zendesk.com/service/", "Zendesk for service"),
                ],
                vec![
                    hit("https://www.g2.com/categories/help-desk", "Best Help Desk Software | G2"),
                    hit("https://www.helpscout.com/", "Help Scout"),
                    hit("https://www.zendesk.com/", "Zendesk"),
                ],
                vec![
                    hit("https://www.trustradius.com/help-desk", "Help Desk | TrustRadius"),
                    hit("https://www.helpscout.com/helpdesk/", "Help Scout helpdesk"),
                ],
            ],
            expected: vec!["zendesk.com", "helpscout.com"],
            impostors: vec![],
        },
    ]
}

/// What the pipeline did with one market.
#[derive(Debug, Clone)]
pub struct Scored {
    pub id: &'static str,
    /// The hosts that survived the confidence floor, best first.
    pub returned: Vec<String>,
    /// Expected hosts that came back.
    pub found: Vec<&'static str>,
    /// Expected hosts that did not.
    pub missed: Vec<&'static str>,
    /// Impostors that came back, with why each is one.
    pub admitted: Vec<(&'static str, &'static str)>,
}

impl Scored {
    /// Of what was expected, how much came back.
    #[must_use]
    pub fn recall(&self) -> f64 {
        let of = self.found.len() + self.missed.len();
        if of == 0 {
            return 1.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "single digits")]
        {
            self.found.len() as f64 / of as f64
        }
    }
}

/// Run the real ranking over a fixture's hits.
///
/// **The real functions, not a copy of them.** `from_results` and the confidence floor are what
/// decide the answer in production; a scorer that reimplemented either would measure itself.
#[must_use]
pub fn score(market: &Market) -> Scored {
    let found: Vec<Found> = from_results(&market.results, QUERIES);
    let returned: Vec<String> = found
        .iter()
        .filter(|f| f.confidence >= landscape_core::subject::MINIMUM_CONFIDENCE)
        .map(|f| f.host.clone())
        .collect();
    Scored {
        id: market.id,
        found: market
            .expected
            .iter()
            .copied()
            .filter(|e| returned.iter().any(|r| r == e))
            .collect(),
        missed: market
            .expected
            .iter()
            .copied()
            .filter(|e| !returned.iter().any(|r| r == e))
            .collect(),
        admitted: market
            .impostors
            .iter()
            .copied()
            .filter(|(host, _)| returned.iter().any(|r| r == host))
            .collect(),
        returned,
    }
}

/// One way of deciding that two URLs are the same product.
///
/// **The open question of PR 3, made runnable.** The roadmap lists five rules and the case each
/// one fails; this is the type that lets the fixtures choose between them instead of a document
/// asserting an answer. See `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    /// What runs today: everything under one registrable domain is one candidate.
    Domain,
    /// Domain plus the first path segment.
    FirstSegment,
    /// Domain plus the first segment that is not a locale or a known container.
    FirstMeaningfulSegment,
    /// Domain plus the last path segment.
    LastSegment,
}

/// Segments that name where a page lives rather than what it is.
///
/// **A guess, and the roadmap says so.** A missing entry fails silently, which is exactly the
/// weakness `FirstMeaningfulSegment` is being measured for.
const CONTAINERS: [&str; 8] = [
    "products",
    "product",
    "solutions",
    "software",
    "apps",
    "services",
    "tools",
    "platform",
];

fn is_locale(segment: &str) -> bool {
    // `en`, `en-us`, `de-de`. Two letters, or two-hyphen-two.
    let bytes = segment.as_bytes();
    match bytes.len() {
        2 => segment.chars().all(|c| c.is_ascii_alphabetic()),
        5 => {
            bytes[2] == b'-'
                && segment
                    .chars()
                    .enumerate()
                    .all(|(i, c)| i == 2 || c.is_ascii_alphabetic())
        }
        _ => false,
    }
}

impl Identity {
    /// The key this rule gives a URL. `None` when the URL cannot be parsed.
    #[must_use]
    pub fn key_for(self, url: &str) -> Option<String> {
        let target = landscape_fetch::Target::parse(url).ok()?;
        let host = landscape_search::candidates::registrable(&target.host);
        let path: Vec<&str> = target.path.split('/').filter(|s| !s.is_empty()).collect();
        let segment = match self {
            Self::Domain => None,
            Self::FirstSegment => path.first().copied(),
            Self::FirstMeaningfulSegment => path
                .iter()
                .copied()
                .find(|s| !is_locale(s) && !CONTAINERS.contains(&s.to_lowercase().as_str())),
            Self::LastSegment => path.last().copied(),
        };
        Some(match segment {
            Some(s) => format!("{host}/{}", s.to_lowercase()),
            None => host,
        })
    }
}
