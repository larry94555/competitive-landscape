//! What a page has already been read to say, so the second reader does not pay the model again.
//!
//! `ROADMAP.md` calls the pair of caches *"the highest-leverage cache in the system"*. The fetch
//! half landed one row ago and stopped the second reader paying **somebody else's server**. This
//! is the half that stops them paying **ours**: a page a model has already read is the expensive
//! one, and re-reading it is where a run spends nearly all of its time.
//!
//! `landscape cost` counts what a run will ask a model, without asking one. Two real sites,
//! measured on 2026-08-09:
//!
//! ```text
//! landscape cost https://linear.app      as this run reads them   16 calls
//! landscape cost https://plausible.io    as this run reads them   14 calls
//! ```
//!
//! **That whole column is what the second reader of the same company used to pay again.** The
//! fetch cache took their fetches to zero and left every one of those calls in place.
//!
//! # The content is part of the key, so a hit cannot be stale
//!
//! [`Extractions`] has **no TTL**, and that is not an oversight — it is the difference between
//! this cache and [`landscape_fetch::cache`]. A page cache answers *"what is at this URL"*, a
//! question whose answer changes without warning, so it needs a window. This one answers *"what
//! does this exact text say about this question"*, and text that has changed produces a
//! different key and therefore a miss. A stale hit is not bounded here; it is impossible.
//!
//! That is also why the whole markdown is the key rather than a hash of it. A 64-bit collision
//! is unlikely and its consequence is the one failure this pipeline exists to prevent — one
//! page's facts attached to another page's URL, cited, and internally consistent. The memory
//! that costs is bounded in [`MAX_MEMO_BYTES`]; the wrong claim would not have been.
//!
//! # Everything in here was produced by one extractor, and that is checked
//!
//! `PROMPT_VERSION` covers the wording of a prompt, and [`crate::EXTRACTION_VERSION`] covers the
//! decoding settings and the shape of the schemas around it. **Neither covers the model.** The
//! worker outlives `llama-server`, so that server can restart with a different model at the same
//! `LLAMA_URL`, and review pointed out what happens next: the next analysis reuses the previous
//! model's answers and the report labels them with the client's current address. One model's
//! words attributed to another, cited and internally consistent — the failure this pipeline
//! exists to prevent, arriving through the cache.
//!
//! So the memory is **scoped** rather than keyed: see [`Extractions::serving`]. Everything held
//! belongs to one model identity, and learning that the identity changed empties it. Scoping
//! rather than keying is what lets a hit cost nothing at all — a key holding the model's name
//! would mean asking the model who it is before answering from memory, which is exactly the
//! request the next section says a hit must not make.
//!
//! # A failure is never remembered
//!
//! Only a [`Settled::Complete`] outcome is kept. A model error, or a run somebody stopped
//! watching, produces a partial answer, and remembering one would replay a transient outage for
//! the life of the process — the defect the page cache was corrected for one row earlier, in a
//! different unit. See [`crate::Settled`].

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::NaiveDate;
use landscape_discover::probes::Answers;

use crate::{Outcome, Settled};

/// How much of a process's memory the remembered extractions may take.
///
/// **Bytes and entries both**, for the reason `landscape_fetch::cache` documents at length: a cap
/// in one unit has a hole shaped exactly like the other. The markdown of a page is the bulk of an
/// entry and varies from a paragraph to a megabyte.
pub const MAX_MEMO_BYTES: usize = 16 * 1024 * 1024;

/// How many extractions may be remembered, whatever they weigh.
pub const MAX_MEMOS: usize = 1_024;

/// What one entry costs beyond the text in it — the map's slot, the key, the vectors.
///
/// Deliberately approximate. Its job is to make an entry with an empty page still cost
/// something, which is what stops the byte budget failing to bound the count.
const OVERHEAD: usize = 512;

/// Everything that decides what an extractor will say.
///
/// **Every input, not the interesting ones.** A cache key that omits an input is a cache that
/// answers a question it was not asked, and the failure is silent: the wrong facts, correctly
/// cited, on a page nobody will re-read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Read {
    /// The prompt set. Bumping [`crate::PROMPT_VERSION`] must miss everything, because a
    /// re-worded prompt is a different question.
    pub prompts: u32,
    /// Everything else about how the answer is produced — see [`crate::EXTRACTION_VERSION`].
    /// A prompt can be word-for-word identical and still mean something different at a
    /// different temperature or under a different schema.
    pub extraction: u32,
    /// Which of the six is being asked.
    pub question: Answers,
    /// The page's address. The prompts embed it, so two pages with identical text and different
    /// URLs are two different questions.
    pub url: String,
    /// The day. `Answers::Changes` resolves relative dates against it, so yesterday's answer is
    /// not today's.
    pub today: NaiveDate,
    /// The text itself. See the module docs for why this is the text and not a digest of it.
    pub markdown: String,
}

impl Read {
    /// Everything that decides the answer, gathered in one place.
    ///
    /// **A constructor rather than five fields filled in at the call site**, because the call
    /// site is `read_one` and no test can drive it — see [`remembering`]. A field forgotten or
    /// pinned to a constant there is a cache answering a question it was not asked, and the
    /// failure is silent: the wrong facts, correctly cited, on a page nobody will re-read.
    pub(crate) fn of(question: Answers, url: &str, markdown: &str, today: NaiveDate) -> Self {
        Self {
            prompts: crate::PROMPT_VERSION,
            extraction: crate::EXTRACTION_VERSION,
            question,
            url: url.to_owned(),
            today,
            markdown: markdown.to_owned(),
        }
    }

    fn cost(&self) -> usize {
        OVERHEAD + self.url.len() + self.markdown.len()
    }
}

fn weight(key: &Read, outcome: &Outcome) -> usize {
    key.cost()
        + outcome.summary.len()
        + outcome.details.iter().map(String::len).sum::<usize>()
        + outcome
            .claims
            .iter()
            .map(|c| c.text.len() + c.quote.len() + 64)
            .sum::<usize>()
}

/// Extractions this process has already paid for.
///
/// **One per process, held beside the `Fetcher`.** Everything in here is remembered by this
/// object, so a second one shares none of it — the same property that made three `Fetcher`s in
/// one run a defect rather than an inefficiency.
#[derive(Debug, Default)]
pub struct Extractions {
    held: Mutex<Held>,
}

#[derive(Debug, Default)]
struct Held {
    by_read: HashMap<Read, Entry>,
    /// Whose answers these are. `None` until a model has been identified — an empty memory
    /// belongs to nobody.
    scope: Option<String>,
    bytes: usize,
    /// Insertion order. A counter rather than a clock: eviction needs a total order, and two
    /// inserts inside one tick of a coarse clock would tie.
    next: u64,
}

#[derive(Debug)]
struct Entry {
    outcome: Outcome,
    order: u64,
    cost: usize,
}

impl Extractions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What this exact page already said about this exact question, if anything.
    #[must_use]
    pub(crate) fn get(&self, key: &Read) -> Option<Outcome> {
        let held = self.held.lock().ok()?;
        held.by_read.get(key).map(|e| e.outcome.clone())
    }

    /// Remember an answer, if it is a whole one.
    ///
    /// Returns whether it was kept, which is what the tests assert against and what the
    /// diagnostic counts.
    pub(crate) fn remember(&self, key: Read, outcome: &Outcome) -> bool {
        // **A partial answer is not an answer.** A model error or an abandoned run would
        // otherwise be replayed to every later reader for the life of the process, and nothing
        // would ever go back and ask again.
        if outcome.settled != Settled::Complete {
            return false;
        }
        let Ok(mut held) = self.held.lock() else {
            return false;
        };
        held.insert(key, outcome.clone());
        true
    }

    /// Declare which model these answers belong to, forgetting everything if it has changed.
    ///
    /// Returns whether anything was forgotten, which is what the diagnostic reports and what the
    /// tests assert against.
    ///
    /// **Called once per analysis, never on the hit path.** Everything remembered is the work of
    /// one model, so a swap invalidates all of it at once and no key has to carry the model's
    /// name — which is what keeps a hit free of any request at all. See the module docs.
    ///
    /// An identity we could not read is never passed here, so an unreachable server forgets
    /// nothing: a question we could not ask is not evidence that the answer changed.
    pub fn serving(&self, identity: &str) -> bool {
        let Ok(mut held) = self.held.lock() else {
            return false;
        };
        if held.scope.as_deref() == Some(identity) {
            return false;
        }
        let had = !held.by_read.is_empty();
        held.by_read.clear();
        held.bytes = 0;
        held.scope = Some(identity.to_owned());
        had
    }

    /// How many extractions are held, and how many bytes they take.
    ///
    /// For the diagnostic, and for the tests about eviction. A cache nobody can see the size of
    /// is one nobody notices growing.
    #[must_use]
    pub fn held(&self) -> (usize, usize) {
        self.held
            .lock()
            .map_or((0, 0), |h| (h.by_read.len(), h.bytes))
    }
}

/// Ask what this page already said, and remember the answer if it is a whole one.
///
/// **The decision lives here rather than inline in the read loop**, and the placement is
/// deliberate. `landscape_analyze::read_one` cannot be driven by a test: it needs a page, a page
/// needs a fetch, and a test server binds loopback, which the address guard refuses absolutely.
/// A rule written inside it is a rule a mutation deleting it survives —
/// `landscape_fetch::cache::insert_allowed` exists for the same reason and was moved there after
/// the harness said so. Here a test passes a counter and watches whether the model was asked.
pub(crate) async fn remembering<F, Fut>(
    memo: &Extractions,
    key: Read,
    extract: F,
) -> Option<Outcome>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Option<Outcome>>,
{
    if let Some(known) = memo.get(&key) {
        tracing::debug!(url = key.url, "extraction served from memory");
        return Some(known);
    }
    // **`None` means the extractor could never be asked** — the model is down — and is neither
    // an answer nor a failed one. Nothing is remembered, and the caller says so on the report.
    let fresh = extract().await?;
    memo.remember(key, &fresh);
    Some(fresh)
}

impl Held {
    fn insert(&mut self, key: Read, outcome: Outcome) {
        let cost = weight(&key, &outcome);
        if let Some(old) = self.by_read.remove(&key) {
            self.bytes -= old.cost;
        }
        // One page too large to sit beside anything else is declined rather than allowed to
        // empty the cache and then not fit. `landscape_fetch::robots` learned this the hard way:
        // an argument that the case cannot arise is not a substitute for the branch.
        if cost > MAX_MEMO_BYTES {
            return;
        }
        while self.bytes + cost > MAX_MEMO_BYTES || self.by_read.len() >= MAX_MEMOS {
            let Some(oldest) = self
                .by_read
                .iter()
                .min_by_key(|(_, e)| e.order)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(gone) = self.by_read.remove(&oldest) {
                self.bytes -= gone.cost;
            }
        }
        self.bytes += cost;
        self.next += 1;
        self.by_read.insert(
            key,
            Entry {
                outcome,
                order: self.next,
                cost,
            },
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::Finding;
    use landscape_core::Confidence;

    fn day() -> NaiveDate {
        "2026-08-09".parse().unwrap()
    }

    fn read(url: &str, markdown: &str) -> Read {
        Read::of(Answers::Pricing, url, markdown, day())
    }

    fn outcome(text: &str, settled: Settled) -> Outcome {
        Outcome {
            claims: vec![Finding {
                text: text.to_owned(),
                quote: "$10 per month".to_owned(),
                confidence: Confidence::High,
                as_of: None,
            }],
            summary: "1 plan found in 1 window".to_owned(),
            details: Vec::new(),
            window_words: 40,
            settled,
        }
    }

    #[test]
    fn a_page_read_twice_is_extracted_once() {
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        assert!(memo.get(&key).is_none(), "nothing has been read yet");

        assert!(memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete)));
        let again = memo.get(&key).expect("the second reader should not pay");
        assert_eq!(again.claims[0].text, "Pro costs $10");
        assert_eq!(again.summary, "1 plan found in 1 window");
    }

    #[test]
    fn a_page_that_changed_is_read_again() {
        // **The property that replaces a TTL.** The text is the key, so an edited page is a
        // different question rather than a stale answer waiting for a window to close.
        let memo = Extractions::new();
        let before = read("https://a.example/pricing", "# Pricing\nPro $10");
        memo.remember(before, &outcome("Pro costs $10", Settled::Complete));

        let after = read("https://a.example/pricing", "# Pricing\nPro $12");
        assert!(
            memo.get(&after).is_none(),
            "an edited page was answered from what the old one said"
        );
    }

    #[test]
    fn the_key_carries_every_input_the_answer_depends_on() {
        // **`Read::of`, not a literal.** The four fields below are filled in inside `read_one`,
        // which needs a fetched page and therefore cannot be driven from a test at all — so a
        // field pinned to a constant there would be invisible. Built here, each one is one
        // assertion away.
        let key = Read::of(Answers::Trust, "https://a.example/x", "# A", day());
        assert_eq!(
            key.prompts,
            crate::PROMPT_VERSION,
            "a re-worded prompt set would answer from the old prompts"
        );
        assert_eq!(key.question, Answers::Trust);
        assert_eq!(key.url, "https://a.example/x");
        assert_eq!(key.markdown, "# A");
        assert_eq!(key.today, day());

        // And each one on its own changes the question.
        let tomorrow: NaiveDate = "2026-08-10".parse().unwrap();
        for other in [
            Read::of(Answers::Features, "https://a.example/x", "# A", day()),
            Read::of(Answers::Trust, "https://b.example/x", "# A", day()),
            Read::of(Answers::Trust, "https://a.example/x", "# B", day()),
            Read::of(Answers::Trust, "https://a.example/x", "# A", tomorrow),
        ] {
            assert_ne!(key, other, "an input was dropped from the key");
        }
    }

    #[test]
    fn every_input_is_part_of_the_question() {
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete));

        // A different question about the same words.
        let other_question = Read {
            question: Answers::Features,
            ..key.clone()
        };
        assert!(memo.get(&other_question).is_none(), "question ignored");

        // The same words at a different address: the prompts name the URL.
        let other_url = Read {
            url: "https://b.example/pricing".to_owned(),
            ..key.clone()
        };
        assert!(memo.get(&other_url).is_none(), "url ignored");

        // A re-worded prompt set is a different question, whatever the page says.
        let other_prompts = Read {
            prompts: key.prompts + 1,
            ..key.clone()
        };
        assert!(memo.get(&other_prompts).is_none(), "prompt version ignored");

        // Tomorrow. `Answers::Changes` resolves "last week" against this.
        let tomorrow = Read {
            today: "2026-08-10".parse().unwrap(),
            ..key.clone()
        };
        assert!(memo.get(&tomorrow).is_none(), "the day was ignored");
    }

    #[test]
    fn a_model_error_is_not_remembered_as_an_answer() {
        // **The lesson from the row before, in a different unit.** A cached failure is worse
        // than a slow one: it is replayed to every later reader, and nothing goes back to ask.
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");

        let mut failed = outcome("", Settled::Partial);
        failed.claims.clear();
        failed.details = vec!["model error: connection refused".to_owned()];

        assert!(!memo.remember(key.clone(), &failed), "a failure was kept");
        assert!(
            memo.get(&key).is_none(),
            "an outage was written down and replayed"
        );
        assert_eq!(memo.held(), (0, 0));
    }

    #[test]
    fn what_is_held_is_bounded_in_both_units() {
        let memo = Extractions::new();
        for i in 0..(MAX_MEMOS * 2) {
            memo.remember(
                read(&format!("https://a.example/{i}"), "# Pricing\nPro $10"),
                &outcome("Pro costs $10", Settled::Complete),
            );
        }
        let (entries, bytes) = memo.held();
        assert!(entries <= MAX_MEMOS, "held {entries} against {MAX_MEMOS}");
        assert!(bytes <= MAX_MEMO_BYTES, "held {bytes} bytes");
        // Not `> 0`: the URL alone satisfies that, and counting only the strings anybody can see
        // is how a byte budget stops bounding the count.
        assert!(
            bytes >= OVERHEAD,
            "an entry was costed at less than an entry costs"
        );

        assert!(
            memo.get(&read("https://a.example/0", "# Pricing\nPro $10"))
                .is_none(),
            "the oldest survived an eviction it should not have"
        );
        let newest = format!("https://a.example/{}", MAX_MEMOS * 2 - 1);
        assert!(
            memo.get(&read(&newest, "# Pricing\nPro $10")).is_some(),
            "the newest was evicted instead of the oldest"
        );
    }

    #[test]
    fn a_few_large_pages_are_bounded_in_bytes_and_many_small_ones_in_count() {
        // The other unit. Sixty-four kilobytes of markdown is an ordinary documentation page,
        // and four hundred of them is well inside `MAX_MEMOS` while being well outside the
        // byte budget — so a cache bounded only in entries would hold every one of them.
        let big = "x".repeat(64 * 1024);
        let memo = Extractions::new();
        for i in 0..400 {
            memo.remember(
                read(&format!("https://a.example/{i}"), &big),
                &outcome("Pro costs $10", Settled::Complete),
            );
        }
        let (entries, bytes) = memo.held();
        assert!(bytes <= MAX_MEMO_BYTES, "held {bytes} bytes");
        assert!(
            entries < 400,
            "four hundred large pages were all kept, so the byte budget did nothing"
        );
        assert!(
            entries < MAX_MEMOS,
            "this test has to bind on bytes, not on count"
        );
        assert!(entries > 0, "the cache emptied itself instead");
    }

    #[test]
    fn an_entry_with_nothing_in_it_still_costs_something() {
        // Asserted on one tiny entry rather than on a full cache: with the per-entry figure
        // gone, `url.len() + markdown.len()` of a real page clears any threshold, and the
        // budget would silently stop bounding the count. This is the smallest entry there is.
        let memo = Extractions::new();
        memo.remember(
            Read::of(Answers::Pricing, "u", "", day()),
            &outcome("", Settled::Complete),
        );
        let (_, bytes) = memo.held();
        assert!(
            bytes >= OVERHEAD,
            "an entry holding nothing was costed at {bytes} bytes"
        );
    }

    #[test]
    fn one_enormous_page_does_not_empty_the_cache_trying_to_fit() {
        let memo = Extractions::new();
        memo.remember(
            read("https://small.example/", "# Pricing\nPro $10"),
            &outcome("Pro costs $10", Settled::Complete),
        );
        memo.remember(
            read("https://huge.example/", &"x".repeat(MAX_MEMO_BYTES + 1)),
            &outcome("Pro costs $10", Settled::Complete),
        );

        let (entries, bytes) = memo.held();
        assert!(bytes <= MAX_MEMO_BYTES, "held {bytes} bytes");
        assert_eq!(entries, 1, "the cache emptied itself for a page it refused");
        assert!(memo
            .get(&read("https://small.example/", "# Pricing\nPro $10"))
            .is_some());
    }

    #[tokio::test]
    async fn the_second_reader_does_not_pay_the_model() {
        // **The whole feature, with a counter where the model would be.** `read_one` cannot be
        // driven from a test — it needs a fetch, and the guard refuses loopback — so the rule
        // lives in `remembering` and this asks it directly.
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        let asked = std::cell::Cell::new(0usize);

        let first = remembering(&memo, key.clone(), || async {
            asked.set(asked.get() + 1);
            Some(outcome("Pro costs $10", Settled::Complete))
        })
        .await
        .expect("the first reader gets an answer");
        assert_eq!(asked.get(), 1, "the first reader must pay");
        assert_eq!(first.claims[0].text, "Pro costs $10");

        let second = remembering(&memo, key, || async {
            asked.set(asked.get() + 1);
            Some(outcome("Pro costs $10", Settled::Complete))
        })
        .await
        .expect("and so does the second");
        assert_eq!(asked.get(), 1, "the second reader paid the model again");
        assert_eq!(
            second.claims[0].text, "Pro costs $10",
            "the hit lost what the page said"
        );
        assert_eq!(second.summary, "1 plan found in 1 window");
    }

    #[tokio::test]
    async fn a_page_already_read_is_served_while_the_model_is_down() {
        // **Review found this, and it is the cache failing in the hour it exists for.** The
        // readiness gate used to sit above the lookup, so a page whose answer was already held
        // came back as `(no model)` during an outage. The closure below stands for everything
        // that needs the model to be up — the health check and the completion alike — and a hit
        // must not enter it at all.
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete));

        let touched_the_model = std::cell::Cell::new(false);
        let served = remembering(&memo, key.clone(), || async {
            touched_the_model.set(true);
            None // as an unreachable `is_ready()` would leave it
        })
        .await
        .expect("a page already read should not need a model");

        assert!(
            !touched_the_model.get(),
            "a hit asked the model whether it was up"
        );
        assert_eq!(served.claims[0].text, "Pro costs $10");

        // And a page that was *not* already read still reports honestly rather than being
        // remembered as an answer.
        let unread = read("https://a.example/security", "# Security\nSOC 2");
        assert!(
            remembering(&memo, unread.clone(), || async { None })
                .await
                .is_none(),
            "a page nobody could read came back as though it had been"
        );
        assert!(memo.get(&unread).is_none(), "a non-answer was remembered");
    }

    #[tokio::test]
    async fn two_models_at_one_address_do_not_share_answers() {
        // **Review found this too.** The worker outlives `llama-server`, so that server can
        // restart with a different model at the same `LLAMA_URL` — and the report would then
        // carry one model's words labelled with the other's address.
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");

        memo.serving("models/qwen2.5-3b-instruct-q4_k_m.gguf");
        memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete));
        assert!(memo.get(&key).is_some(), "the same model should hit");
        assert!(
            !memo.serving("models/qwen2.5-3b-instruct-q4_k_m.gguf"),
            "the same identity forgot something"
        );
        assert!(
            memo.get(&key).is_some(),
            "a redeclaration threw the memory away"
        );

        assert!(
            memo.serving("models/llama-3.2-3b-instruct-q4_k_m.gguf"),
            "a different model should have emptied it"
        );
        assert!(
            memo.get(&key).is_none(),
            "one model's answer was served as another's"
        );
        assert_eq!(memo.held(), (0, 0), "the bytes went with it");
    }

    #[test]
    fn what_produced_an_answer_is_part_of_the_question() {
        // The static half of the same finding: identical prompts read at a different
        // temperature, or under a different schema, are not the same question.
        let key = Read::of(Answers::Pricing, "https://a.example/x", "# A", day());
        assert_eq!(key.prompts, crate::PROMPT_VERSION);
        assert_eq!(key.extraction, crate::EXTRACTION_VERSION);

        let rewired = Read {
            extraction: key.extraction + 1,
            ..key.clone()
        };
        assert_ne!(key, rewired, "the extraction version was dropped");
    }

    #[tokio::test]
    async fn a_run_that_failed_is_asked_again_rather_than_replayed() {
        // The other half, and the one that matters when the model is having a bad afternoon:
        // a partial answer is reported to the reader who caused it and forgotten immediately.
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        let asked = std::cell::Cell::new(0usize);

        let mut failed = outcome("", Settled::Partial);
        failed.claims.clear();
        failed.details = vec!["model error: connection refused".to_owned()];

        for _ in 0..3 {
            let got = remembering(&memo, key.clone(), || async {
                asked.set(asked.get() + 1);
                Some(failed.clone())
            })
            .await
            .expect("a partial answer is still an answer for this reader");
            assert!(got.claims.is_empty());
        }
        assert_eq!(
            asked.get(),
            3,
            "an outage was written down and replayed instead of being retried"
        );

        // And when it recovers, that answer is the one kept.
        let good = remembering(&memo, key.clone(), || async {
            asked.set(asked.get() + 1);
            Some(outcome("Pro costs $10", Settled::Complete))
        })
        .await
        .expect("the recovered answer");
        assert_eq!(good.claims[0].text, "Pro costs $10");
        assert_eq!(asked.get(), 4);

        remembering(&memo, key, || async {
            asked.set(asked.get() + 1);
            Some(outcome("Pro costs $10", Settled::Complete))
        })
        .await;
        assert_eq!(asked.get(), 4, "the recovered answer was not kept");
    }

    #[test]
    fn re_reading_the_same_page_replaces_rather_than_doubles() {
        let memo = Extractions::new();
        let key = read("https://a.example/pricing", "# Pricing\nPro $10");
        memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete));
        let (_, once) = memo.held();
        memo.remember(key.clone(), &outcome("Pro costs $10", Settled::Complete));
        let (entries, twice) = memo.held();

        assert_eq!(entries, 1);
        assert_eq!(once, twice, "a replaced entry's bytes were counted twice");
    }
}
