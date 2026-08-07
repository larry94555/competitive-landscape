//! The queries, written down rather than improvised.
//!
//! `FACT_CHECKING.md` §3.3: *"templated queries, one small set per section, so retrieval is
//! reproducible and auditable rather than model-improvised. The query set is versioned like
//! a prompt and recorded on the analysis, so a retrieval regression is attributable."*
//!
//! Both halves of that matter and they are easy to separate by accident:
//!
//! - **Templated** means a model never writes a query. A model that improvises retrieval
//!   produces a report nobody can reproduce, because the same prompt tomorrow reads
//!   different pages for reasons no log records.
//! - **Versioned** means [`QUERY_SET`] is stamped on the run. When a section that used to
//!   fill starts coming back empty, the question *"did the queries change?"* has an answer
//!   instead of an argument.
//!
//! # Why the set is short
//!
//! One query per unanswered question. Not three, not a fan-out per synonym. Every query is
//! a round trip before any page has been read, against a 90–180 second budget that
//! [`landscape_discover`] has already spent fifteen seconds of on politeness. Breadth here
//! is bought with the only currency the reader notices.

use landscape_discover::probes::Answers;

/// The version of the query set, stamped on a run.
///
/// **Raise this whenever a template below changes**, including a change that only looks
/// cosmetic — a dropped `OR` branch changes what comes back. It is a date plus a counter
/// rather than a hash, because the thing a person asks is *"which set ran on the day that
/// report was wrong"*.
pub const QUERY_SET: &str = "2026-08-07.1";

/// The most hits worth taking from one query.
///
/// A search engine will happily return a hundred, and a hundred URLs is not more coverage —
/// it is the same eight pages plus ninety-two aggregator pages about them. The cap is here
/// rather than at the call site so a misconfigured or hostile engine cannot decide how much
/// work this process does.
pub const HITS_PER_QUERY: usize = 5;

/// One query, and what it was asked for.
///
/// The template is carried beside the text so a log line says *which* template produced a
/// disappointing result, rather than leaving somebody to guess it from the interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// What goes to the engine.
    pub text: String,
    /// The question this is trying to answer.
    pub answers: Answers,
    /// The uninterpolated form, for the log and for the audit.
    pub template: &'static str,
}

/// The template for each question, in the shape `FACT_CHECKING.md` §3.3 gives.
///
/// `{}` is the subject's name. Every one of them names the company, because a query that
/// does not is a query about the whole web.
const fn template_for(answers: Answers) -> &'static str {
    match answers {
        Answers::Pricing => r#"{} pricing plans"#,
        Answers::Features => r#"{} features"#,
        Answers::Changes => r#"{} changelog OR "release notes""#,
        Answers::Identity => r#"{} founded OR headquarters OR "about us""#,
        Answers::Trust => r#"{} "SOC 2" OR "ISO 27001" OR security compliance"#,
        Answers::Direction => r#"{} careers OR hiring OR funding"#,
    }
}

/// Queries for the questions discovery could not answer, and for no others.
///
/// `unanswered` is what it says: pass the questions that came back empty. Passing every
/// question would work and would be wrong — it would search for a pricing page on a site
/// whose pricing page we already read, which is a round trip spent to be told something we
/// know.
///
/// An empty or blank `name` yields no queries at all. Interpolating one would send
/// `pricing plans` to a search engine, which returns the internet.
#[must_use]
pub fn for_questions(name: &str, unanswered: &[Answers]) -> Vec<Query> {
    let quoted = quote(name);
    if quoted.is_empty() {
        return Vec::new();
    }

    let mut seen: Vec<Answers> = Vec::new();
    let mut out = Vec::new();
    for answers in unanswered {
        // A caller assembling this list from two places can repeat a question. Asking the
        // same thing twice costs a round trip and returns the same page.
        if seen.contains(answers) {
            continue;
        }
        seen.push(*answers);
        let template = template_for(*answers);
        out.push(Query {
            text: template.replacen("{}", &quoted, 1),
            answers: *answers,
            template,
        });
    }
    out
}

/// The subject's name as a phrase the engine will keep together.
///
/// `help scout` unquoted matches every page containing both words; `"help scout"` matches
/// the company. The quotes are the difference between a query about a company and a query
/// about two English words.
///
/// **The name is stripped of its own quotes first.** A subject called `The "Real" Co` would
/// otherwise close the phrase early and hand the rest of itself to the engine as operators —
/// the same shape as an injection, arriving through a field a stranger typed.
fn quote(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| *c != '"' && !c.is_control())
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("\"{trimmed}\"")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_question_discovery_answered_is_not_searched_for() {
        // The rule of the whole crate: search fills gaps. A site whose /pricing answered 200
        // must not cost a round trip to be told it has a pricing page.
        let queries = for_questions("Linear", &[Answers::Changes]);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].answers, Answers::Changes);
        assert!(
            !queries.iter().any(|q| q.answers == Answers::Pricing),
            "an answered question was searched for anyway: {queries:?}"
        );
    }

    #[test]
    fn a_company_whose_probes_found_everything_costs_no_searches() {
        assert!(for_questions("Linear", &[]).is_empty());
    }

    #[test]
    fn every_question_has_a_template_and_every_template_names_the_company() {
        // A template that drops the name searches the web for `pricing plans`, which returns
        // the internet. Checked for all six rather than the ones a caller happens to pass.
        let all = [
            Answers::Pricing,
            Answers::Features,
            Answers::Changes,
            Answers::Identity,
            Answers::Trust,
            Answers::Direction,
        ];
        let queries = for_questions("Help Scout", &all);
        assert_eq!(queries.len(), all.len());
        for q in &queries {
            assert!(
                q.text.contains("\"Help Scout\""),
                "{:?} does not name the company: {}",
                q.answers,
                q.text
            );
            assert!(
                q.template.contains("{}"),
                "{:?} has no interpolation point",
                q.answers
            );
        }
    }

    #[test]
    fn a_two_word_name_is_kept_together() {
        let queries = for_questions("Help Scout", &[Answers::Pricing]);
        // Unquoted, this matches every page with both words on it.
        assert_eq!(queries[0].text, "\"Help Scout\" pricing plans");
    }

    #[test]
    fn a_name_cannot_close_the_phrase_and_add_its_own_operators() {
        // The injection shape, arriving through the box a stranger types into.
        let queries = for_questions(r#"Acme" site:evil.test "#, &[Answers::Pricing]);
        assert_eq!(
            queries[0].text.matches('"').count(),
            2,
            "the phrase is no longer one phrase: {}",
            queries[0].text
        );
        assert!(
            queries[0].text.starts_with("\"Acme site:evil.test\""),
            "{}",
            queries[0].text
        );
    }

    #[test]
    fn a_blank_name_asks_nothing() {
        // Not an empty query — no query. `pricing plans` on its own is a valid search and a
        // meaningless one, and it would still cost the round trip.
        assert!(for_questions("", &[Answers::Pricing]).is_empty());
        assert!(for_questions("   ", &[Answers::Pricing]).is_empty());
        assert!(for_questions("\"\"", &[Answers::Pricing]).is_empty());
    }

    #[test]
    fn the_same_question_twice_is_asked_once() {
        let queries = for_questions("Linear", &[Answers::Pricing, Answers::Pricing]);
        assert_eq!(queries.len(), 1);
    }

    #[test]
    fn the_query_set_is_versioned_so_a_regression_is_attributable() {
        // Not a formatting rule for its own sake: the whole value of the constant is that a
        // report can be traced to the set that produced it, and an empty or unchanging
        // string cannot do that.
        assert!(QUERY_SET.starts_with("2026-"), "{QUERY_SET}");
        assert!(QUERY_SET.contains('.'), "no counter in {QUERY_SET}");
    }

    #[test]
    fn the_hit_cap_bounds_what_one_query_can_cost() {
        // Compared through bindings so the assertions are about the constants rather than
        // being folded away as literal comparisons — the same reason `rank.rs` does it.
        let (per_query, reading_budget) = (HITS_PER_QUERY, landscape_discover::rank::CAP_RUNG_0);
        assert!(per_query > 0, "a cap of zero searches for nothing");
        assert!(
            per_query <= reading_budget,
            "one query may not fill the whole reading budget on its own"
        );
    }
}
