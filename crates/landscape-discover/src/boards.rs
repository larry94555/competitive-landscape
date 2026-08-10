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

/// Every board root the company's own page **links to**, in the order it links to them.
///
/// The URLs are reduced to `https://<host>/<slug>`: a careers page links to individual vacancies,
/// and each of `jobs.ashbyhq.com/Linear/069c…/application` and `jobs.ashbyhq.com/Linear/0c7c…` is
/// the same board seen through a different door. Reading dozens of vacancy pages to learn what
/// one board page lists is the impolite way to find out.
#[must_use]
pub fn named_by(html: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for at in url_starts(html) {
        let Some(board) = board_root(&html[at..]) else {
            continue;
        };
        if !found.contains(&board) {
            found.push(board);
        }
    }
    found.truncate(MOST_BOARDS);
    found
}

/// The fields whose value is *"the address of this job"*.
///
/// **A quoted string is not a link either**, which is review's second pass on the same rule: a
/// `<script>` assigning `const competitor = "https://jobs.ashbyhq.com/SomebodyElse/role"` puts a
/// board URL at the start of a quoted value, and so does a *"similar jobs at other companies"*
/// widget. Neither is this company linking to its own board.
///
/// So a URL counts only as the value of an HTML `href`, or of one of these — the exact keys the
/// three real careers pages use, and nothing wider. **`url` is deliberately absent**: it is what
/// a widget listing somebody else's vacancies would use too, and a key that cannot tell the two
/// apart is not evidence.
const JOB_URL_KEYS: [&str; 4] = ["absolute_url", "jobUrl", "applyUrl", "jobPostingUrl"];

/// Where in the page a URL **is the value of a link**.
///
/// **Review found the first version scanning the raw HTML for a host.** Any occurrence counted:
/// prose, a tracking blob, a *"similar jobs at other companies"* widget, a redirect parameter —
/// so `jobs.ashbyhq.com/SomebodyElse` appearing anywhere admitted somebody else's board and
/// published their vacancies under this company's name. That is the exact failure the *found,
/// never guessed* rule exists to prevent, arriving through a string nobody linked.
///
/// The test is that a quote comes **immediately** before the scheme, which is what `href="…"`
/// and `{"url":"…"}` both look like and what `href="/out?to=https://…"` does not. Asked this way
/// round rather than by pairing quotes across the document, because pairing goes out of phase
/// the moment a page nests JSON inside JSON — and `vercel.com/careers` does exactly that, with
/// its board links inside `{"jobs":[{"absolute_url":"https://…"}]}` written as an escaped string
/// within another string.
///
/// **An unquoted attribute is not read.** HTML5 permits `href=https://…`, no applicant tracker
/// emits one, and allowing `=` as an opener would let every `?to=` redirect back in.
fn url_starts(html: &str) -> Vec<usize> {
    let bytes = html.as_bytes();
    let mut starts = Vec::new();
    for (at, _) in html.char_indices() {
        if at == 0 {
            continue;
        }
        if !matches!(bytes[at - 1], b'"' | b'\'') {
            continue;
        }
        if named_as_a_link(&html[..at - 1]) && after_scheme(&html[at..]).is_some() {
            starts.push(at);
        }
    }
    starts
}

/// Whether what comes before this value says it is a link to a job.
///
/// An `href=`, or a [`JOB_URL_KEYS`] field. Read backwards through whatever escaping the page
/// applied: `href=` + quote, `\"jobUrl\":` + quote and `"absolute_url":` + quote are the same
/// shape once the backslashes a nested JSON string carries are stepped over.
fn named_as_a_link(before: &str) -> bool {
    let before = before.trim_end_matches('\\').trim_end();

    if let Some(head) = before.strip_suffix('=') {
        let name = head.trim_end();
        let from = name.len()
            - name
                .chars()
                .rev()
                .take_while(char::is_ascii_alphanumeric)
                .count();
        return name[from..].eq_ignore_ascii_case("href");
    }

    let Some(head) = before.strip_suffix(':') else {
        return false;
    };
    let head = head.trim_end().trim_end_matches('"').trim_end_matches('\\');
    let from = head.len()
        - head
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
    JOB_URL_KEYS
        .iter()
        .any(|key| head[from..].eq_ignore_ascii_case(key))
}

/// The board a value points at, when the value **is** a URL to one.
///
/// Parsed rather than matched: scheme, then host, then one path segment, each checked in turn.
/// A bare substring is what the first version used, and a host that merely appears inside a
/// longer one — `jobs.lever.co.evil.test`, `evil-jobs.lever.co` — is not a board this page links
/// to.
///
/// `\/` is accepted wherever `/` is: a careers page built as a single-page application carries
/// its links inside escaped JSON, so `https:\/\/jobs.ashbyhq.com\/Linear` is the real shape on
/// `linear.app/careers`.
fn board_root(value: &str) -> Option<String> {
    let rest = after_scheme(value)?;
    let host_end = rest.find(ENDS_A_PART).unwrap_or(rest.len());
    let host = &rest[..host_end];
    if !BOARD_HOSTS.contains(&host) {
        return None;
    }
    let path = separator(&rest[host_end..])?;
    let slug: String = path
        .chars()
        .take_while(|c| !ENDS_A_PART.contains(c) && !c.is_whitespace())
        .collect();
    // A bare host with no slug is the tracker's own marketing site, not a board.
    if slug.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{slug}"))
}

/// Where one part of a URL stops: a separator, a query, a fragment, or the quote that ends the
/// value it sits in.
const ENDS_A_PART: [char; 6] = ['/', '\\', '?', '#', '"', '\''];

/// What follows `https://`, `http://`, their escaped forms, or a protocol-relative `//`.
fn after_scheme(value: &str) -> Option<&str> {
    for prefix in [
        "https://",
        "http://",
        r"https:\/\/",
        r"http:\/\/",
        "//",
        r"\/\/",
    ] {
        if let Some(rest) = value.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

/// The path after a separator, in either the plain or the escaped spelling.
fn separator(rest: &str) -> Option<&str> {
    rest.strip_prefix('/').or_else(|| rest.strip_prefix(r"\/"))
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
        let html = r#"{"jobUrl":"https:\/\/jobs.ashbyhq.com\/Linear\/069c4628\/application"}"#;
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
    fn a_board_url_the_page_did_not_link_to_is_not_a_link() {
        // **Review found the first version scanning the raw HTML for a host**, so any occurrence
        // counted — and `jobs.ashbyhq.com/SomebodyElse` appearing anywhere admitted somebody
        // else's board and published their vacancies under this company's name. Every shape
        // below is a real way that string turns up on a page nobody linked it from.
        for html in [
            // Prose.
            "<p>We post our roles through jobs.ashbyhq.com/SomebodyElse these days.</p>",
            // A redirect parameter: the value starts with the redirector, not with the board.
            r#"<a href="/out?to=https://jobs.ashbyhq.com/SomebodyElse">Apply</a>"#,
            r#"<a href="https://links.example.com/r?u=https://jobs.lever.co/SomebodyElse">go</a>"#,
            // A tracking blob, unquoted, in a script.
            "<script>track({dest: https://jobs.ashbyhq.com/SomebodyElse});</script>",
            // **A quoted string is not a link.** A script assigning a competitor's board to a
            // variable, and a "similar jobs at other companies" widget, both put a board URL at
            // the start of a quoted value — and neither is this company linking to its own.
            r#"<script>const competitor = "https://jobs.ashbyhq.com/SomebodyElse/role";</script>"#,
            r#"{"similar":[{"url":"https://jobs.ashbyhq.com/SomebodyElse/role"}]}"#,
            r#"{"company":"SomebodyElse","site":"https://jobs.lever.co/SomebodyElse"}"#,
            // A value that is not quoted at all. Not valid JSON and not an attribute anybody
            // writes, but the key beside it is one we trust, so the quote is what says the URL
            // is the value rather than something that follows it.
            r#"{"absolute_url": https://jobs.ashbyhq.com/SomebodyElse/role}"#,
            // A host that merely ends with one of ours.
            r#"<a href="https://x-jobs.lever.co/SomebodyElse">Apply</a>"#,
            r#"<a href="https://jobs.lever.co.evil.test/SomebodyElse">Apply</a>"#,
        ] {
            assert!(named_by(html).is_empty(), "{html}");
        }
    }

    #[test]
    fn a_link_is_read_however_the_page_spells_one() {
        // The three spellings that appear on the real pages this was built against, plus the
        // protocol-relative form a stylesheet-era template still emits.
        for html in [
            r#"<a href="https://jobs.ashbyhq.com/Linear/069c">Apply</a>"#,
            r#"<a href='http://jobs.ashbyhq.com/Linear/069c'>Apply</a>"#,
            r#"{"jobUrl":"https:\/\/jobs.ashbyhq.com\/Linear\/069c"}"#,
            r#"{"applyUrl":"https://jobs.ashbyhq.com/Linear/069c"}"#,
            r#"\"absolute_url\":\"https://jobs.ashbyhq.com/Linear/069c\""#,
            r#"<a href="//jobs.ashbyhq.com/Linear/069c">Apply</a>"#,
        ] {
            assert_eq!(
                named_by(html),
                vec!["https://jobs.ashbyhq.com/Linear"],
                "{html}"
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
        // a stranger the standing this list grants. The host is parsed to its end and compared
        // whole, so a suffix or a prefix is a different host and not a board.
        for html in [
            r#"<a href="https://jobs.lever.co.evil.test/acme">Apply</a>"#,
            r#"<a href="https://evil-jobs.lever.co/acme">Apply</a>"#,
        ] {
            assert!(named_by(html).is_empty(), "{html}");
        }
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
