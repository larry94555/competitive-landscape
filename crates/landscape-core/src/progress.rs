//! How far a run has got, in the units the run actually knows.
//!
//! A reader who submits an idea waits four to eight minutes on the target hardware. Until this
//! existed the page said `Reading public web pages…` for all of it — one sentence that is as
//! true at eight seconds as at eight minutes, so a reader could not tell a run that was nearly
//! finished from one that had barely started, or either from one that had died.
//!
//! # The denominator has to be real
//!
//! The temptation is a bar that fills smoothly from nothing to done, because that is what a
//! progress bar looks like. It would be a lie, and a specific one: **for the first stretch of a
//! run nothing knows how much work there is.** Working out which companies a description means
//! is a search, a fetch of each candidate's front page and a gate's verdict, and none of those
//! can say in advance how many there will be.
//!
//! What *is* known:
//!
//! | | known when |
//! |---|---|
//! | how many companies this run covers | as soon as the set is resolved, before any of it is read |
//! | how many pages one company will be read for | when `landscape_analyze::order::plan` picks them, before the first is fetched |
//! | how many pages have been read | continuously |
//!
//! So this carries a **phase always** and a **fraction only once there is one**. Before the
//! plan exists [`Progress::pages`] is `None`, and the page shows a named phase and an
//! indeterminate bar rather than a number nobody computed. A reader is better served by *"still
//! working out which companies you mean"* than by `7%`, because the first is true.
//!
//! # It must never go backwards
//!
//! A bar that retreats reads as a failure even when the run is healthy, and this pipeline has
//! already paid for that lesson once: `BENCHMARKS.md` Run 16 is a section that grew and then
//! shrank in front of a reader, and `landscape/src/progress.rs` exists because two concurrent
//! writes could put an older report on screen than the one already seen.
//!
//! [`Progress::fraction`] is therefore defined so that it cannot decrease as a run proceeds,
//! and [`Progress::fraction_never_goes_backwards`] is the test that says so over a whole
//! simulated run rather than at a handful of points.

use serde::{Deserialize, Serialize};

/// A count of finished things out of a known total.
///
/// `of` is never invented. Every construction site has something that already knows the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Counted {
    pub done: usize,
    pub of: usize,
}

impl Counted {
    #[must_use]
    pub const fn new(done: usize, of: usize) -> Self {
        Self { done, of }
    }

    /// How much of this is finished, or `None` when there is nothing to be finished.
    ///
    /// **Zero out of zero is not zero percent.** A company whose plan holds no page at all is a
    /// company with nothing to wait for, and reporting `0%` for it would leave a bar stuck at
    /// the bottom for the one case that is already over.
    #[must_use]
    pub fn share(self) -> Option<f32> {
        if self.of == 0 {
            return None;
        }
        // A `done` past `of` would be a bug elsewhere, and clamping is the behavior that keeps
        // a reader's bar inside its box while it is being found.
        #[allow(clippy::cast_precision_loss)]
        Some((self.done.min(self.of) as f32) / (self.of as f32))
    }
}

/// What a run is doing right now.
///
/// Named for what a reader would say, not for the function that is executing. `Discovering` is
/// *"finding the pages worth reading"*; the reader does not care that it is a robots fetch and
/// six probes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Finding which pages of this company are worth reading. No page count exists yet.
    ///
    /// The default, because it is where every run starts and because defaulting to any later
    /// phase would claim work that has not happened.
    #[default]
    Discovering,
    /// Reading them. This is the phase with a real fraction, and the long one.
    Reading,
    /// Asking a search engine about the questions the company's own pages did not answer.
    Searching,
    /// Merging what every company produced into one report.
    Assembling,
}

impl Phase {
    /// The sentence a reader is shown. Present tense, no jargon, no ellipsis on the finished
    /// one because it is not still happening.
    #[must_use]
    pub const fn wording(self) -> &'static str {
        match self {
            Self::Discovering => "Finding the pages worth reading",
            Self::Reading => "Reading public web pages",
            Self::Searching => "Searching for what their own pages did not say",
            Self::Assembling => "Putting the report together",
        }
    }
}

/// How far a run has got.
///
/// Rides on [`crate::Report`] because that is the only thing the worker and the API already
/// share — see `landscape-api::events`, which polls the row rather than adding a broker. A
/// second channel for this would be a second thing to run and lose messages through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Progress {
    pub phase: Phase,
    /// Companies wholly finished, out of the companies this run set out to cover.
    pub companies: Counted,
    /// Pages read for the company being read now, out of the pages its plan holds.
    ///
    /// **`None` until that plan exists.** Discovery has to finish before anything can say how
    /// many pages there will be, and inventing a number for that window is the one thing this
    /// module is written to prevent.
    pub pages: Option<Counted>,
}

impl Progress {
    /// A run that has started on a set of companies and not yet planned any reading.
    #[must_use]
    pub const fn starting(companies: usize) -> Self {
        Self {
            phase: Phase::Discovering,
            companies: Counted::new(0, companies),
            pages: None,
        }
    }

    /// A run that is over. Everything it set out to cover is covered.
    #[must_use]
    pub const fn finished(companies: usize) -> Self {
        Self {
            phase: Phase::Assembling,
            companies: Counted::new(companies, companies),
            pages: None,
        }
    }

    /// How much of the whole run is done, between `0.0` and `1.0`.
    ///
    /// **Companies are weighted equally, and that is an assumption worth stating.** One may hold
    /// nine pages and the next three, so the bar moves at different speeds through each. The
    /// alternative is a denominator that grows as each company's plan is made — a bar that
    /// *retreats* when the next company turns out to be larger, which is the failure this must
    /// not have. Equal weighting is the only division available before the work is done, and it
    /// is monotonic.
    ///
    /// `None` when the run covers no companies, which is not a run.
    #[must_use]
    pub fn fraction(&self) -> Option<f32> {
        let of = self.companies.of;
        if of == 0 {
            return None;
        }
        // The company being read now counts for the share of its own pages that are read. A
        // phase past `Reading` has finished those pages whatever the count says, so it counts
        // whole - otherwise the bar would stall through search and assembly, which is exactly
        // when a reader is most likely to think it has hung.
        let within = match self.phase {
            Phase::Discovering => 0.0,
            Phase::Reading => self.pages.and_then(Counted::share).unwrap_or(0.0),
            Phase::Searching | Phase::Assembling => 1.0,
        };
        #[allow(clippy::cast_precision_loss)]
        let done = self.companies.done.min(of) as f32;
        #[allow(clippy::cast_precision_loss)]
        let total = of as f32;
        // `within` belongs to the company *after* the finished ones, so it is only added while
        // one is unfinished. Adding it at the end would put the bar past its own box.
        let carried = if self.companies.done < of {
            within
        } else {
            0.0
        };
        Some(((done + carried) / total).clamp(0.0, 1.0))
    }

    /// The percentage a reader is shown, rounded toward the honest side.
    ///
    /// **Rounded down, and never `100` before the run is over.** A bar that reads `100%` while a
    /// page is still being read is the same defect as a section that arrives and then changes:
    /// it tells somebody they can stop waiting, and they cannot.
    #[must_use]
    pub fn percent(&self) -> Option<u8> {
        let fraction = self.fraction()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = (fraction * 100.0).floor() as u8;
        let over = self.companies.done >= self.companies.of;
        Some(if whole >= 100 && !over { 99 } else { whole })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    // Panicking IS how a test reports failure. The lint stays denied everywhere else.

    use super::*;

    #[test]
    fn nothing_out_of_nothing_has_no_share() {
        assert_eq!(Counted::new(0, 0).share(), None);
    }

    #[test]
    fn a_plan_with_no_pages_is_not_zero_percent_of_anything() {
        // The distinction the whole module rests on: "no work" and "no work done" are
        // different, and only one of them should hold a bar at the bottom.
        assert_eq!(Counted::new(0, 0).share(), None);
        assert_eq!(Counted::new(0, 4).share(), Some(0.0));
    }

    #[test]
    fn a_run_over_no_companies_reports_nothing() {
        assert_eq!(Progress::starting(0).fraction(), None);
    }

    #[test]
    fn discovery_has_no_fraction_of_its_own_and_says_so() {
        let p = Progress::starting(3);
        assert_eq!(p.pages, None, "no plan exists yet, so no page count may");
        assert_eq!(p.percent(), Some(0));
    }

    #[test]
    fn reading_counts_pages_within_the_company_being_read() {
        let p = Progress {
            phase: Phase::Reading,
            companies: Counted::new(1, 2),
            pages: Some(Counted::new(2, 4)),
        };
        // One of two companies done, and half of the second's pages.
        assert_eq!(p.percent(), Some(75));
    }

    #[test]
    fn search_and_assembly_do_not_stall_the_bar() {
        // The window a reader is most likely to read as a hang: the pages are finished and the
        // company is not, so a page-only fraction would sit still for the whole of it.
        let reading = Progress {
            phase: Phase::Reading,
            companies: Counted::new(0, 2),
            pages: Some(Counted::new(4, 4)),
        };
        let searching = Progress {
            phase: Phase::Searching,
            ..reading
        };
        assert_eq!(reading.percent(), Some(50));
        assert_eq!(searching.percent(), Some(50));
    }

    #[test]
    fn a_finished_run_is_a_hundred_and_an_unfinished_one_never_is() {
        assert_eq!(Progress::finished(3).percent(), Some(100));
        // Every page of the only company read, and the company not yet closed out. The
        // arithmetic says 100; the run is not over, so the reader is not told it is.
        let nearly = Progress {
            phase: Phase::Assembling,
            companies: Counted::new(0, 1),
            pages: Some(Counted::new(9, 9)),
        };
        assert_eq!(nearly.percent(), Some(99));
    }

    #[test]
    fn a_count_past_its_total_is_whole_and_not_more_than_whole() {
        // **Asserted on `share` itself, because `fraction` clamps too.** The mutation harness
        // found the first version of this: removing the guard here changed nothing any test
        // could see, since the outer clamp hid it - and `share` is public, so a caller reading
        // it directly would have got `3.0` and drawn a bar three times the width of its box.
        assert_eq!(Counted::new(12, 4).share(), Some(1.0));
    }

    #[test]
    fn a_page_count_past_its_plan_stays_inside_the_box() {
        let p = Progress {
            phase: Phase::Reading,
            companies: Counted::new(0, 1),
            pages: Some(Counted::new(12, 4)),
        };
        assert_eq!(p.fraction(), Some(1.0));
        assert_eq!(
            p.percent(),
            Some(99),
            "still running, so still not finished"
        );
    }

    #[test]
    fn fraction_never_goes_backwards() {
        // A whole run, company by company and page by page, including the two things that made
        // an earlier draft retreat: a later company holding more pages than the one before it,
        // and the reset to `None` while the next company is being discovered.
        let plans = [3usize, 9, 1];
        let companies = plans.len();
        let mut seen = 0.0f32;
        let mut check = |p: Progress| {
            let Some(now) = p.fraction() else {
                panic!("a run over companies has a fraction: {p:?}");
            };
            assert!(
                now >= seen - f32::EPSILON,
                "went backwards: {seen} then {now} at {p:?}"
            );
            seen = now;
        };

        check(Progress::starting(companies));
        for (index, pages) in plans.iter().enumerate() {
            // Discovery for this company: the previous company's page count is gone and the
            // next one does not exist.
            check(Progress {
                phase: Phase::Discovering,
                companies: Counted::new(index, companies),
                pages: None,
            });
            for read in 0..=*pages {
                check(Progress {
                    phase: Phase::Reading,
                    companies: Counted::new(index, companies),
                    pages: Some(Counted::new(read, *pages)),
                });
            }
            for phase in [Phase::Searching, Phase::Assembling] {
                check(Progress {
                    phase,
                    companies: Counted::new(index, companies),
                    pages: Some(Counted::new(*pages, *pages)),
                });
            }
        }
        check(Progress::finished(companies));
        assert!((seen - 1.0).abs() < f32::EPSILON, "a finished run is whole");
    }

    #[test]
    fn every_phase_has_a_sentence_a_reader_could_read() {
        for phase in [
            Phase::Discovering,
            Phase::Reading,
            Phase::Searching,
            Phase::Assembling,
        ] {
            let said = phase.wording();
            assert!(!said.is_empty(), "{phase:?} says nothing");
            assert!(
                !said.ends_with('.') && !said.ends_with('…'),
                "{phase:?} punctuates its own sentence: the page decides that, not this"
            );
        }
    }
}
