//! The applicant-tracking board a company's own careers page says its roles are on.
//!
//! `ROADMAP.md` has listed **public ATS boards** among the structured probes since Phase 1 was
//! written, and they were the last of that list still to come. The reason they are worth having
//! is visible the moment you look at a real careers page: three of the four checked while this
//! was written — `linear.app/careers`, `front.com/jobs`, `helpscout.com/company/careers/` — put
//! nothing on their own page but a link to somebody else's board.
//!
//! # Found, never guessed
//!
//! **Every board here is one the company's own page linked to.** Nothing constructs
//! `jobs.ashbyhq.com/<the company's name>` and hopes, and that restraint is the whole safety
//! argument: a guessed slug that belongs to somebody else would put another company's vacancies
//! on this company's report, cited, and internally consistent — the entity-resolution failure
//! `FACT_CHECKING.md` §3.1 exists to prevent, arriving through a URL nobody typed.
//!
//! A guess is also unnecessary. The link is right there, and a company that has not published
//! one has not told us where its roles are.
//!
//! # A board is not the company's own server
//!
//! An admitted board is [`landscape_core::Disposition::Attributed`], not `Primary`. The bytes
//! come from a stranger's host — but authorship is not in doubt, because the subject's own page
//! is what named it. That is precisely what `Attributed` is for, and it keeps the rule that
//! `Primary` means *the company's own domain* intact rather than widened by a feature.
//!
//! # Which hosts, and why so few
//!
//! Only boards whose public URL is `<known host>/<company slug>`, so a link can be reduced to
//! the board's root by taking one path segment. Hosts that put the company in a **subdomain** —
//! `acme.recruitee.com`, `acme.bamboohr.com` — are deliberately absent: every subdomain of those
//! hosts is a different company, so a typo in a link is another company's board rather than a
//! 404, and the cost of being wrong is the failure above.

/// The hosts whose boards are addressed as `<host>/<slug>`.
///
/// Matched on the whole host, so `jobs.lever.co` is on the list and `notjobs.lever.co.evil.test`
/// is not. Short on purpose: each entry is a claim that a URL shape means what we think, and a
/// list of forty is forty chances to be wrong about somebody else's site.
pub const BOARD_HOSTS: [&str; 6] = [
    "jobs.ashbyhq.com",
    "boards.greenhouse.io",
    "job-boards.greenhouse.io",
    "jobs.lever.co",
    "apply.workable.com",
    "jobs.smartrecruiters.com",
];

/// How many boards one page may contribute.
///
/// A careers page links to the board once per vacancy — `front.com/jobs` has dozens — and they
/// all reduce to one root, so this bounds the pathological case rather than the ordinary one: a
/// page that names several genuinely different boards is a page we have misread.
pub const MOST_BOARDS: usize = 2;

/// Every board root the company's own page points at, in the order it points at them.
///
/// The URLs are reduced to `https://<host>/<slug>`: a careers page links to individual
/// vacancies, and each of `jobs.ashbyhq.com/Linear/069c…/application` and
/// `jobs.ashbyhq.com/Linear/0c7c…` is the same board seen through a different door. Reading
/// dozens of vacancy pages to learn what one board page lists is the impolite way to find out.
#[must_use]
pub fn named_by(html: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for host in BOARD_HOSTS {
        let mut from = 0usize;
        while let Some(at) = html[from..].find(host).map(|i| from + i) {
            from = at + host.len();
            if !starts_a_host(html, at) {
                continue;
            }
            let Some(board) = root_of(host, &html[from..]) else {
                continue;
            };
            if !found.contains(&board) {
                found.push(board);
            }
        }
    }
    found.truncate(MOST_BOARDS);
    found
}

/// Whether the match at `at` is a whole host rather than the tail of a longer one.
///
/// **`boards.greenhouse.io` is a substring of `job-boards.greenhouse.io`**, and two of the six
/// entries below are exactly that pair — so a plain search finds one board twice, under two
/// names, and spends two of a page's allowance on the same vacancies. The same rule is what
/// keeps `evil-jobs.lever.co` from being read as `jobs.lever.co`.
fn starts_a_host(html: &str, at: usize) -> bool {
    let Some(before) = html[..at].chars().next_back() else {
        return true;
    };
    !(before.is_alphanumeric() || before == '-' || before == '.')
}

/// `<host>` plus one path segment, or `None` when what follows is not a board.
///
/// The segment ends at the first character that cannot be in a path: a quote or an angle bracket
/// from the surrounding HTML, whitespace, a query or fragment — and a **backslash**, because a
/// careers page built as a single-page application carries its links inside escaped JSON and
/// `jobs.ashbyhq.com/Linear\` is what a naive reader takes away from one.
fn root_of(host: &str, after: &str) -> Option<String> {
    // `\/` as well as `/`: the separator itself is escaped inside a JSON blob.
    let path = after
        .strip_prefix('/')
        .or_else(|| after.strip_prefix(r"\/"))?;
    let slug: String = path
        .chars()
        .take_while(|c| {
            !matches!(c, '/' | '?' | '#' | '"' | '\'' | '<' | '>' | '\\' | ' ')
                && !c.is_whitespace()
        })
        .collect();
    // A bare host with no slug is the ATS's own marketing site, not a board.
    if slug.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{slug}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_vacancy_link_reduces_to_the_board_it_is_on() {
        // The exact shapes on three real careers pages the day this was written. A careers page
        // links to vacancies, and every one of them is a door into the same board.
        let html = concat!(
            r#"<a href="https://jobs.ashbyhq.com/Linear/069c4628-88d7-4e4d-b393-c996fc7f3076">Engineer</a>"#,
            r#"<a href="https://jobs.ashbyhq.com/Linear/0c7c2e26-0a98-42cf-a47c-9a3999fb513b/application">Apply</a>"#,
        );
        assert_eq!(named_by(html), vec!["https://jobs.ashbyhq.com/Linear"]);
    }

    #[test]
    fn a_link_inside_escaped_json_stops_at_the_backslash() {
        // `linear.app/careers` is a single-page application and carries its links inside a JSON
        // blob, so the raw HTML holds `jobs.ashbyhq.com/Linear\/069c…`. A reader that took the
        // backslash as part of the slug would ask for a board nobody has.
        let html = r#"{"url":"https:\/\/jobs.ashbyhq.com\/Linear\/069c4628\/application"}"#;
        assert_eq!(named_by(html), vec!["https://jobs.ashbyhq.com/Linear"]);
    }

    #[test]
    fn every_board_shape_this_list_claims_to_know() {
        for (host, link) in [
            (
                "jobs.ashbyhq.com",
                "https://jobs.ashbyhq.com/helpscout/form/future-openings",
            ),
            (
                "boards.greenhouse.io",
                "https://boards.greenhouse.io/acme/jobs/4012",
            ),
            (
                "job-boards.greenhouse.io",
                "https://job-boards.greenhouse.io/acme/jobs/4012",
            ),
            ("jobs.lever.co", "https://jobs.lever.co/acme/1234-5678"),
            (
                "apply.workable.com",
                "https://apply.workable.com/acme/j/ABCDEF/",
            ),
            (
                "jobs.smartrecruiters.com",
                "https://jobs.smartrecruiters.com/acme/74000-engineer",
            ),
        ] {
            let html = format!(r#"<a href="{link}">Apply</a>"#);
            let found = named_by(&html);
            assert_eq!(found.len(), 1, "{host}: {found:?}");
            assert!(
                found[0].starts_with(&format!("https://{host}/")),
                "{host}: {found:?}"
            );
            assert_eq!(
                found[0].matches('/').count(),
                3,
                "{host}: not a root: {found:?}"
            );
        }
    }

    #[test]
    fn a_host_we_do_not_know_is_not_a_board() {
        // Not a shortcoming — a refusal. Admitting an arbitrary off-site link is how a page on
        // somebody else's site becomes evidence about this company, and the whole list exists to
        // keep that to URL shapes we have actually looked at.
        let html = r#"<a href="https://careers.example.com/roles">Roles</a>
                      <a href="https://www.linkedin.com/company/acme/jobs/">LinkedIn</a>"#;
        assert!(named_by(html).is_empty());
    }

    #[test]
    fn a_lookalike_host_is_not_the_board_it_looks_like() {
        // `jobs.lever.co.evil.test` contains `jobs.lever.co`, and a naive `contains` would hand
        // a stranger the standing this list grants. The slug reader stops at the first `/`, so
        // what comes out is the real host or nothing.
        let html = r#"<a href="https://jobs.lever.co.evil.test/acme">Apply</a>"#;
        let found = named_by(html);
        assert!(
            found.is_empty() || found[0] == "https://jobs.lever.co/.evil.test",
            "{found:?}"
        );
    }

    #[test]
    fn the_ats_marketing_site_is_not_a_board() {
        for html in [
            r#"<a href="https://jobs.ashbyhq.com">Powered by Ashby</a>"#,
            r#"<a href="https://jobs.ashbyhq.com/">Powered by Ashby</a>"#,
            r#"<p>We use jobs.ashbyhq.com for applications.</p>"#,
        ] {
            assert!(named_by(html).is_empty(), "{html}");
        }
    }

    #[test]
    fn a_page_that_names_a_hundred_vacancies_still_names_one_board() {
        let html = (0..100)
            .map(|i| format!(r#"<a href="https://jobs.ashbyhq.com/frontcareers/{i}">Role {i}</a>"#))
            .collect::<String>();
        assert_eq!(
            named_by(&html),
            vec!["https://jobs.ashbyhq.com/frontcareers"]
        );
    }

    #[test]
    fn a_page_naming_more_boards_than_we_believe_is_capped() {
        // Several genuinely different boards on one careers page is a page we have misread, so
        // the cap is on the pathological case. Each is a request to somebody else's server.
        let html = (0..6)
            .map(|i| format!(r#"<a href="https://jobs.lever.co/acme{i}/1">Role</a>"#))
            .collect::<String>();
        assert_eq!(named_by(&html).len(), MOST_BOARDS);
    }

    #[test]
    fn nothing_is_ever_guessed_from_the_company_name() {
        // **The safety property, stated as a test.** A page that mentions an ATS without linking
        // to a board yields nothing: the alternative is constructing a slug from the company's
        // name, and a slug that belongs to somebody else puts their vacancies on this report.
        let html = "<h1>Careers at Acme</h1><p>We hire through Greenhouse and Lever.</p>";
        assert!(named_by(html).is_empty());
    }
}
