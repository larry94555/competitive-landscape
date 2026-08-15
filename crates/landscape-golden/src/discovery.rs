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

use landscape_search::candidates::{describe, from_results, Found};
use landscape_search::competitors::{assemble, Aside};
use landscape_search::Hit;

/// How many queries a description sends per round.
///
/// **Taken from production, not written again.** It was `= 3`, a second copy of a public
/// constant — so adding a query would have left these fixtures scoring against the old
/// denominator with their shape check still passing. That is the duplicated-rule mistake this
/// repository has a register entry about, committed in a file about measuring mistakes.
pub const QUERIES: usize = landscape_search::candidates::IDEA_QUERIES;

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
    /// Frozen front pages, keyed by URL, as markdown.
    ///
    /// **Without these the scorer stops at the ranking**, which is what the first version of
    /// this file did — and review found that it therefore could not see PR 4 at all. Raising
    /// `SHARED_WORDS` changes `assemble`, `assemble` reads what `describe` found on a page, and
    /// with no pages neither runs. A fixture set that cannot move when the change it is named
    /// for lands is not a measurement.
    ///
    /// A URL absent here is a page that could not be fetched, which is
    /// `Vocabulary::Unreadable` — a state worth having in the set.
    pub pages: Vec<(&'static str, &'static str)>,
    /// The page behind a **returned URL**, as markdown — the evidence PR 3 needs and today's
    /// pipeline never sees.
    ///
    /// **Kept apart from `pages` because they are read at different moments.** `pages` holds
    /// front pages, which `describe` fetches *after* candidates have been merged by domain.
    /// These are the pages of the URLs themselves, which an identity rule has to read *before*
    /// the merge — the inversion PR 3 has to make. Putting them in one list would hide that the
    /// order is the hard part.
    ///
    /// **Every URL here must appear in `results`**, and
    /// `no_page_is_evidence_the_engine_never_returned` asserts it. A fixture that supplies a
    /// page production never fetches is an alibi, which this repository has three register
    /// entries about.
    pub product_pages: Vec<(&'static str, &'static str)>,
    /// The **publisher** pages a query returned, as markdown, keyed by URL.
    ///
    /// **The market's own writing, which the pipeline used to throw away.** These hosts are on
    /// `NOT_A_COMPANY`: never candidates, and until `landscape_search::literature` never read
    /// either. What a guide *links to* is what it says is in this market, and two independent
    /// guides naming a company is the corroboration cross-query agreement cannot give a
    /// specialist.
    ///
    /// Every URL here must appear in `results`, exactly as `product_pages` must, and the same
    /// test says so.
    pub guide_pages: Vec<(&'static str, &'static str)>,
    /// The market's words, as `assemble` receives them.
    ///
    /// Fixture data, like `expected`: one careful reading of what this market is called, written
    /// where it can be argued with. Production derives it from the engine's titles; deriving it
    /// here would make the fixture depend on the step above the one being measured.
    pub words: Vec<&'static str>,
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
            words: vec!["project", "management", "design", "agency"],
            pages: vec![
                ("https://www.microsoft.com/", "# Microsoft

Cloud, computers, project management and more."),
                ("https://asana.com/", "# Asana

Project management for teams, including design agency work."),
                // **The impostor's page, and why one word is not a test.** It shares `project`
                // and sells a board game. `SHARED_WORDS = 1` admits it; that is what PR 4 has
                // to move, and it can only be seen with this page in the set.
                ("https://projectplusgame.com/", "# Project Plus

A board game about project deadlines. Two to six players."),
                ("https://www.workamajig.com/", "# Workamajig

Project management built for a creative design agency."),
                ("https://www.notion.so/", "# Notion

Notes, docs and project management in one place."),
            ],
            // **The three invariants an identity rule has to satisfy, on URLs this market's
            // queries really returned.** Two locales of one product, and two products of one
            // suite. No path-shaped rule does both; see the identity test.
            product_pages: vec![
                ("https://www.microsoft.com/en-us/microsoft-365/project/project-management-software",
                 "# Microsoft Project

Project management software."),
                ("https://www.microsoft.com/en-gb/microsoft-365/project/project-management-software",
                 "# Microsoft Project

Project management software."),
                ("https://www.microsoft.com/en-us/microsoft-365/teams/group-chat-software",
                 "# Microsoft Teams

Group chat and meetings."),
                // The only URL this market returned for Notion, so `split` reads it rather than
                // the front page - and the candidate is the product, not the company.
                ("https://www.notion.so/product/projects", "# Notion Projects

Project management inside Notion, for a design agency or any other team."),
            ],
            guide_pages: vec![
                // **What a guide links to is what it says is in this market.** Two of them
                // name Workamajig - which one query returned, and which the ranking alone
                // refuses. That is cause 3, and it is the reader's actual complaint.
                ("https://www.g2.com/categories/project-management", "# Best Project Management Software

1. [Asana](https://asana.com/)
2. [Workamajig](https://www.workamajig.com/) - for creative agencies
3. [Microsoft Project](https://www.microsoft.com/en-us/microsoft-365/project/project-management-software)"),
                ("https://www.capterra.com/project-management-software/", "# Project Management Software

Compare [Asana](https://asana.com/), [Workamajig](https://www.workamajig.com/) and [Notion](https://www.notion.so/), and read [our rivals at G2](https://www.g2.com/categories/project-management)."),
            ],
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
            words: vec!["spreadsheet", "finance"],
            pages: vec![
                ("https://www.microsoft.com/", "# Microsoft

Excel is the spreadsheet finance teams use."),
                ("https://www.google.com/", "# Google

Sheets is a spreadsheet for teams."),
                ("https://www.airtable.com/", "# Airtable

A spreadsheet-database for operations and finance."),
            ],
            // One product across two locales **and** across its own pricing page - the case
            // `LastSegment` splits.
            product_pages: vec![
                ("https://www.microsoft.com/en-us/microsoft-365/excel", "# Microsoft Excel

The spreadsheet."),
                ("https://www.microsoft.com/de-de/microsoft-365/excel", "# Microsoft Excel

Die Tabellenkalkulation."),
                ("https://www.microsoft.com/en-us/microsoft-365/excel/pricing", "# Microsoft Excel

Plans and pricing."),
                ("https://www.google.com/sheets/about/", "# Google Sheets

The spreadsheet finance teams share."),
                ("https://www.google.com/sheets/about/pricing", "# Google Sheets

Plans and pricing."),
            ],
            guide_pages: vec![],
        },
        Market {
            id: "keyword-impostor",
            // **Four words, because three could not judge the change this fixture is for.**
            // `enough_words` asks for half the market rounded down, so a three-word market asks
            // for one - exactly what it asked for before PR 4. The fixture the roadmap named as
            // that PR's judge could not have moved, which is the same defect PR 2 was corrected
            // for: a case that cannot fail is not a measurement.
            prompt: "time tracking for independent consultants",
            results: vec![
                vec![
                    hit("https://www.harvestapp.com/", "Harvest - time tracking"),
                    hit("https://trackingtimemusic.com/", "TrackingTime - the drum machine"),
                ],
                vec![
                    hit("https://www.harvestapp.com/pricing", "Harvest pricing"),
                    hit("https://trackingtimemusic.com/kits", "TrackingTime kits"),
                    hit("https://timezonecheck.com/", "Timezone Check"),
                ],
                vec![
                    hit("https://toggl.com/track/", "Toggl Track"),
                    hit("https://www.harvestapp.com/", "Harvest - time tracking"),
                    hit("https://timezonecheck.com/", "Timezone Check"),
                ],
            ],
            expected: vec!["harvestapp.com", "toggl.com"],
            impostors: vec![
                (
                    "trackingtimemusic.com",
                    "a drum machine; shares tracking and time, sells neither",
                ),
                // **The case this PR needed and the set did not have.** It shares exactly one
                // word - `time` - so it passes a bar of one and fails a bar of two. Without it
                // the only impostor here shares nothing, and a rule that moves the bar from one
                // to two would have had nothing to prove itself against.
                (
                    "timezonecheck.com",
                    "a world clock; shares time and nothing else",
                ),
            ],
            words: vec!["time", "tracking", "independent", "consultants"],
            pages: vec![
                ("https://www.harvestapp.com/", "# Harvest

Time tracking and invoicing for consultants."),
                ("https://toggl.com/", "# Toggl Track

Time tracking for teams and consultants."),
                ("https://trackingtimemusic.com/", "# TrackingTime

A drum machine. Sequencing, kits and swing."),
                ("https://timezonecheck.com/", "# Timezone Check

What time is it anywhere. Plan a meeting across the world."),
            ],
            // Toggl's only appearance is a product page, so it is read instead of the front
            // page. It is `Uncorroborated` either way - one query - and this is the case that
            // proves reading a product page does not on its own admit anybody.
            product_pages: vec![("https://toggl.com/track/", "# Toggl Track

Time tracking for teams and consultants.")],
            guide_pages: vec![],
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
            words: vec!["invoicing", "freelance", "translators"],
            pages: vec![
                ("https://www.freshbooks.com/", "# FreshBooks

Invoicing and accounting for freelance work."),
                ("https://quickbooks.intuit.com/", "# QuickBooks

Invoicing and books for small business."),
                ("https://www.protemos.com/", "# Protemos

Invoicing and project management for freelance translators."),
            ],
            // **Two vendors, one product name.** Both pages call the thing they sell
            // *Invoicing*, and they are two different companies - so a rule that keys on the
            // declared name alone merges FreshBooks into Intuit. A generic product noun on two
            // domains is not a contrived case: it is what *Projects*, *Inbox* and *Analytics*
            // look like across any market.
            product_pages: vec![
                ("https://www.freshbooks.com/invoice", "# Invoicing

Send an invoice, get paid."),
                ("https://quickbooks.intuit.com/invoicing/", "# Invoicing

Create and track invoices."),
            ],
            guide_pages: vec![],
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
            words: vec!["customer", "support", "helpdesk"],
            pages: vec![
                ("https://www.zendesk.com/", "# Zendesk

Customer support and helpdesk software."),
                // **Deliberately absent: `helpscout.com`.** A page that could not be fetched is
                // `Vocabulary::Unreadable`, and a set with no unreadable page in it cannot tell
                // *we could not check* from *we checked and it failed*.
            ],
            product_pages: vec![],
            guide_pages: vec![
                // Almost the whole result set is guides. Three agree about Zendesk and two about
                // Help Scout - whose front page still cannot be read, so it stays `Unread`
                // rather than being admitted on their word. Being listed is not being legible.
                ("https://www.g2.com/categories/help-desk", "# Best Help Desk Software

1. [Zendesk](https://www.zendesk.com/)
2. [Help Scout](https://www.helpscout.com/)"),
                ("https://www.capterra.com/help-desk-software/", "# Help Desk Software

[Zendesk](https://www.zendesk.com/) and [Help Scout](https://www.helpscout.com/) lead the category."),
                ("https://www.trustradius.com/help-desk", "# Help Desk | TrustRadius

[Zendesk](https://www.zendesk.com/) is the one most teams start with."),
            ],
        },
    ]
}

/// What the pipeline did with one market.
#[derive(Debug, Clone)]
pub struct Scored {
    pub id: &'static str,
    /// The hosts in the finished set, best first. **What a reader would be shown.**
    pub returned: Vec<String>,
    /// Expected hosts that came back.
    pub found: Vec<&'static str>,
    /// Expected hosts that did not.
    pub missed: Vec<&'static str>,
    /// Impostors that came back, with why each is one.
    pub admitted: Vec<(&'static str, &'static str)>,
    /// Every candidate that reached the **fit test**, and how many of the market's words its
    /// page used.
    ///
    /// **The input to PR 4's question, kept apart from its answer.** A candidate excluded for
    /// being uncorroborated never reaches the fit test at all, so counting it against a fit rule
    /// would score the rule for a decision some other rule made. These are the ones the rule
    /// actually decides, taken from the set: a member's `Because::Found { shares }` and an
    /// `Aside::ElsewhereEntirely { used }` are the same number on either side of the bar.
    pub at_the_fit_test: Vec<(String, usize)>,
    /// What each company in the set is **called**, by domain.
    ///
    /// **The number PR 3 moves, and recall cannot see it.** Naming a suite *Microsoft* when
    /// three queries returned two Microsoft Project pages and one Teams page is the second of
    /// the four causes, and it changes no host in `returned` — so a scorecard of recall and
    /// impostors would have reported that change as doing nothing at all.
    pub named: Vec<(String, String)>,
    /// Every host that was excluded, with **the typed reason**, not its sentence.
    ///
    /// **The half a count cannot show.** Two changes can both raise recall by one and leave the
    /// set aside for entirely different reasons, and only one of them is an improvement.
    ///
    /// **Typed, because review found the count version green through a real regression.** An
    /// expected company moving from `Uncorroborated` to `Unread` is a different defect with a
    /// different fix, and a length or a sentence hides it — a sentence doubly so, since a
    /// wording change would then fail a test about discovery.
    pub set_aside: Vec<(String, Aside)>,
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

/// Run the real pipeline over a fixture, from the hits to the set a reader would see.
///
/// **The real functions at every stage, not a copy of any of them.** `from_results` ranks,
/// `describe` reads each candidate's frozen page, and `assemble` admits or excludes on
/// `SHARED_WORDS`. A scorer that stopped at the ranking — which the first version of this
/// did — cannot see PR 4 at all, because `SHARED_WORDS` lives in the stage it skipped.
pub async fn score(market: &Market) -> Scored {
    let words: Vec<String> = market.words.iter().map(|w| (*w).to_owned()).collect();

    // **One closure, two kinds of page, and the order is the point.** `split` asks for the URL a
    // query returned; `describe` asks for a domain's front page. An exact URL match answers the
    // first, and a host match against a root URL answers the second — so a product page can
    // never be handed back as a front page, which would let the fixture prove a merge on
    // evidence the pipeline does not have.
    let fetch = |url: String| async move {
        if let Some((_, page)) = market
            .product_pages
            .iter()
            .chain(market.guide_pages.iter())
            .find(|(at, _)| *at == url)
        {
            return Some((*page).to_owned());
        }
        // **Only a front page is answered from `pages`, and only for a front-page request.**
        // Matching every URL on a known host against its front page would hand `split` a page
        // production would never have for that URL - the fixture answering a question it was
        // not asked, which is the alibi this file already has a test against.
        if landscape_search::candidates::depth_of(&url) > 0 {
            return None;
        }
        let host = host_of(&url);
        market
            .pages
            .iter()
            .find(|(at, _)| host_of(at) == host)
            .map(|(_, page)| (*page).to_owned())
    };

    let found: Vec<Found> = from_results(&market.results, QUERIES);
    let sources = landscape_search::candidates::Sources {
        asked: QUERIES,
        guides: landscape_search::literature::read(&market.results),
    };
    let named = landscape_search::literature::named_in(&market.results, &fetch).await;
    let found = landscape_search::literature::admit(found, &named, sources);
    let found = landscape_search::products::split(found, sources, &market.results, &fetch).await;
    let described = describe(&found, &words, fetch).await;

    let set = assemble(
        described,
        sources,
        &words,
        landscape_search::competitors::Evidence::ADescribedMarket,
    );
    let returned: Vec<String> = set
        .members
        .iter()
        .map(|m| m.candidate.canonical_domain.clone())
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
        set_aside: set
            .set_aside
            .iter()
            .map(|(c, why)| (c.canonical_domain.clone(), why.clone()))
            .collect(),
        at_the_fit_test: set
            .members
            .iter()
            .filter_map(|m| match &m.because {
                landscape_search::competitors::Because::Found { shares, .. } => {
                    Some((m.candidate.canonical_domain.clone(), shares.len()))
                }
                landscape_search::competitors::Because::Named => None,
            })
            .chain(set.set_aside.iter().filter_map(|(c, why)| match why {
                Aside::ElsewhereEntirely { used, .. } => {
                    Some((c.canonical_domain.clone(), used.len()))
                }
                _ => None,
            }))
            .collect(),
        named: set
            .members
            .iter()
            .map(|m| {
                (
                    m.candidate.canonical_domain.clone(),
                    m.candidate.name.clone(),
                )
            })
            .collect(),
        returned,
    }
}

/// The registrable host of a URL, or the URL itself when it will not parse.
///
/// The real rule, so the fixture's page lookup groups the way the pipeline does.
fn host_of(url: &str) -> String {
    landscape_fetch::Target::parse(url).ok().map_or_else(
        || url.to_owned(),
        |t| landscape_search::candidates::registrable(&t.host),
    )
}

/// One way of deciding a company is in the market a reader described.
///
/// **PR 4's open question, made runnable, exactly as [`Identity`] was PR 3's.** The roadmap
/// offers three ways to move the fit test off one word and says to settle them against this set
/// rather than by picking a number. These are the four that were written down; `Fit::needed` is
/// each one's answer to *how many of the market's words must a front page use*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// What ran before PR 4: one word, whatever the market.
    AnyWord,
    /// A flat two.
    TwoWords,
    /// Half the market's words, rounded **up**.
    HalfRoundedUp,
    /// Half the market's words, rounded **down**, never below one. What runs now.
    HalfRoundedDown,
}

impl Fit {
    /// How many of the market's words this rule asks a front page to use.
    #[must_use]
    pub fn needed(self, market: usize) -> usize {
        match self {
            Self::AnyWord => 1,
            Self::TwoWords => 2,
            Self::HalfRoundedUp => market.div_ceil(2).max(1),
            // The real rule, called rather than copied: a golden set carrying its own version of
            // the thing it scores measures itself.
            Self::HalfRoundedDown => landscape_search::competitors::enough_words(market),
        }
    }

    /// Every rule, so a test cannot score three of four and call it a comparison.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::AnyWord,
            Self::TwoWords,
            Self::HalfRoundedUp,
            Self::HalfRoundedDown,
        ]
    }
}

/// What one rule would do to one market: the impostors it admits and the expected companies it
/// loses.
///
/// **Only candidates that reached the fit test are counted.** A company excluded for having one
/// query behind it is not a company any fit rule turned away, and charging it to one would make
/// every rule look equally bad at a job none of them were doing.
#[must_use]
pub fn under(rule: Fit, market: &Market, scored: &Scored) -> (Vec<String>, Vec<String>) {
    let needed = rule.needed(market.words.len());
    let passes = |host: &str| {
        scored
            .at_the_fit_test
            .iter()
            .find(|(h, _)| h == host)
            .is_some_and(|(_, used)| *used >= needed)
    };
    let reached = |host: &str| scored.at_the_fit_test.iter().any(|(h, _)| h == host);
    let admitted = market
        .impostors
        .iter()
        .map(|(host, _)| (*host).to_owned())
        .filter(|host| passes(host))
        .collect();
    let lost = market
        .expected
        .iter()
        .map(|host| (*host).to_owned())
        .filter(|host| reached(host) && !passes(host))
        .collect();
    (admitted, lost)
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
    /// The vendor's domain **and** the name a page declares about itself — the roadmap's
    /// fifth candidate, and now what production runs.
    ///
    /// **One line, because the rule moved into the crate it judges.** This was implemented here
    /// first, to answer whether it works at all before `landscape-search` was changed. It does,
    /// so it lives in [`landscape_search::products::declared_for`] and this delegates — a golden
    /// set carrying its own copy of a rule measures itself, which is the defect that put four
    /// causes in front of a reader with 1026 tests passing.
    ///
    /// **The domain is half the key, and dropping it was a real defect.** An earlier version
    /// returned the bare heading, so two unrelated vendors whose products are both called
    /// *Invoicing* keyed the same and merged into one company — a worse failure than the
    /// domain-collapse it was written to fix, since at least that one never crossed a vendor
    /// boundary. `specialist-in-one-article` holds exactly that pair.
    #[must_use]
    pub fn declared_for(url: &str, page: &str) -> Option<String> {
        landscape_search::products::declared_for(url, page)
    }

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
