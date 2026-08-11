r"""One dialect, and it is American.

The button said **Analyse**. Nobody had ever chosen a dialect, so the repository had quietly
grown both: `analyse` beside `analysis`, `catalogue` beside `catalog`, `labelled` beside
`labeled`, `normalise` beside `normalize`. Mixed spelling is not a cosmetic problem. A reader
cannot tell a house style from a typo, `grep normalize` misses half the callers, and the first
place it surfaced was the one word on the busiest button in the product.

Sweeping it once fixes today. This gate is what stops tomorrow, because the next `analyse` will
be typed by somebody who has no idea a decision was ever made.

# What is not here, and why

**`analyses` is absent from the list, deliberately.** It is the American plural of *analysis*
(`two analyses a day`, the `analyses` table) *and* the British third-person verb (`it
analyses`). No word list can tell those apart, and the plural is the overwhelming majority
here, so flagging it would make the gate cry wolf several hundred times. The three verb uses
were fixed by hand. A fourth typed later is the one thing this does not catch, and saying so is
better than a check that silently declines to look.

**Same-in-both words are absent too**, and the tempting ones are worth naming: `analysis`,
`analyst`, `cancellation` (double `l` in both), `optimistic`, `emphasis`, `advice`, `service`,
`device`, `axes`. Adding any of them would flag correct text.

# The list is derived, because a hand-written one is always short

The first version listed every inflection by hand and **certified this repository American
while it was not**: review found `generalises`, `generalising`, `characterises`, `criticises`,
`canonicalisation` and `synthesised` still in the tree, green. The map held `generalise` and
`generalised`. One verb has eight forms and a person writing them out gets five.

The regular families are generated from a stem now - `_ise`, `_yse`, `_our`, `_double_l` - so
adding a verb is one entry and every form comes with it. Adding the derivation found **31 more
British spellings** in a tree the previous version had just called clean.

Words the generators produce that are wrong are removed by name below the map, which is a short
list somebody can audit: `analyses`, `vaporise`, `cancellation`.

# How a word is matched

**One word at a time, looked up.** The scanner finds *a word* with a single character class and
asks a dict whether that word is British, so the cost does not depend on how long the list is.
The first version was one alternation of every word, which is O(text x words) - fine at 240
words, and at 1271 it did not finish.

The word pattern splits `camelCase` as well as `snake_case`, which is what the old lookarounds
were for: `normaliseText` gives `normalise` and `Text`. **`aria-labelledby` gives `labelledby`**,
one token that is not in the map, so that attribute is spared structurally rather than by a
lookahead somebody has to get right.

# What is skipped, and how to skip something

  * `crates/landscape-golden/pages/` is other people's frozen HTML. A word changed in one would
    make the golden set a record of what we wish a page had said.
  * **a run of base64 characters longer than 48 is data, not English.** The prototype pages
    embed video and captions as data URIs, and a long enough blob contains every short word
    there is - `greYTK`, `kErb`, `oMOuldq` are all real matches from one. The first version of
    this rule skipped any whitespace-delimited run over 60 characters instead, which is true of
    a blob and **also true of a line of Rust**: a 69-character call site went unswept and
    `cargo test` found it rather than this. What separates data from code is punctuation.
  * **a passage between `american-spelling: off` and `american-spelling: on` is not checked.**
    Some text has to write the British forms to say what it is about - the register entry about
    this sweep, the benchmark run describing it, the comment above the gate in `verify.py`.
    Both markers are comments in Markdown, Python and Rust alike, so nothing shows in a
    rendered page. **An `off` with no `on` fails the gate**, because that is the only way a
    marker turns into a silent hole.

Everything skipped is printed on a passing run. A check that does not mention its blind spots
reads, on every green run, like a check that has none.

    python3 scripts/american_spelling.py
"""

import io
import os
import re
import subprocess
import sys

# British on the left, American on the right. Longest first is not needed here - the regex
# alternation is sorted before use - but every entry must be a word that genuinely differs
# between the dialects, and nothing that merely looks foreign.
BRITISH = {
    'analyse': 'analyze', 'analysed': 'analyzed', 'analysing': 'analyzing',
    'analyser': 'analyzer', 'analysers': 'analyzers',
    'catalogue': 'catalog', 'catalogues': 'catalogs', 'catalogued': 'cataloged',
    'cataloguing': 'cataloging',
    'labelled': 'labeled', 'labelling': 'labeling', 'unlabelled': 'unlabeled',
    'relabelled': 'relabeled', 'relabelling': 'relabeling',
    'behaviour': 'behavior', 'behaviours': 'behaviors', 'behavioural': 'behavioral',
    'judgement': 'judgment', 'judgements': 'judgments', 'judgemental': 'judgmental',
    'normalise': 'normalize', 'normalised': 'normalized', 'normalises': 'normalizes',
    'normalising': 'normalizing', 'normalisation': 'normalization',
    'normaliser': 'normalizer', 'normalisers': 'normalizers',
    'serialise': 'serialize', 'serialised': 'serialized', 'serialises': 'serializes',
    'serialising': 'serializing', 'serialisation': 'serialization',
    'deserialise': 'deserialize', 'deserialised': 'deserialized',
    'deserialises': 'deserializes', 'deserialising': 'deserializing',
    'licence': 'license', 'licences': 'licenses',
    'cancelled': 'canceled', 'cancelling': 'canceling',
    'towards': 'toward', 'amongst': 'among', 'whilst': 'while',
    'recognise': 'recognize', 'recognised': 'recognized', 'recognises': 'recognizes',
    'recognising': 'recognizing', 'recognisable': 'recognizable',
    'unrecognised': 'unrecognized', 'unrecognisable': 'unrecognizable',
    'artefact': 'artifact', 'artefacts': 'artifacts',
    'honour': 'honor', 'honours': 'honors', 'honoured': 'honored', 'honouring': 'honoring',
    'optimise': 'optimize', 'optimised': 'optimized', 'optimising': 'optimizing',
    'optimisation': 'optimization',
    'defence': 'defense', 'offence': 'offense', 'offences': 'offenses',
    'pretence': 'pretense',
    'summarise': 'summarize', 'summarised': 'summarized', 'summarises': 'summarizes',
    'summarising': 'summarizing', 'summariser': 'summarizer',
    'unsummarised': 'unsummarized',
    'authorise': 'authorize', 'authorised': 'authorized', 'authorisation': 'authorization',
    'unauthorised': 'unauthorized',
    'modelled': 'modeled', 'modelling': 'modeling',
    'organise': 'organize', 'organised': 'organized', 'organisation': 'organization',
    'organisations': 'organizations', 'reorganise': 'reorganize',
    'programme': 'program', 'programmes': 'programs',
    'colour': 'color', 'colours': 'colors', 'coloured': 'colored',
    'prioritise': 'prioritize', 'prioritised': 'prioritized', 'prioritising': 'prioritizing',
    'spelt': 'spelled', 'learnt': 'learned', 'dreamt': 'dreamed', 'burnt': 'burned',
    'grey': 'gray', 'greys': 'grays', 'greyed': 'grayed', 'greyscale': 'grayscale',
    'favour': 'favor', 'favours': 'favors', 'favourable': 'favorable',
    'centre': 'center', 'centres': 'centers',
    'initialise': 'initialize', 'initialised': 'initialized',
    'initialisation': 'initialization',
    'travelling': 'traveling', 'traveller': 'traveler', 'travellers': 'travelers',
    'labour': 'labor', 'neighbour': 'neighbor', 'neighbours': 'neighbors',
    'neighbouring': 'neighboring', 'neighbourhood': 'neighborhood',
    'emphasise': 'emphasize', 'emphasised': 'emphasized', 'emphasising': 'emphasizing',
    'quantise': 'quantize', 'quantised': 'quantized', 'quantising': 'quantizing',
    'quantisation': 'quantization',
    'rigour': 'rigor', 'rigours': 'rigors', 'ageing': 'aging',
    'localise': 'localize', 'localised': 'localized', 'localisation': 'localization',
    'capitalise': 'capitalize', 'capitalised': 'capitalized',
    'parameterise': 'parameterize', 'parameterised': 'parameterized',
    'sanitise': 'sanitize', 'sanitised': 'sanitized', 'sanitising': 'sanitizing',
    'visualise': 'visualize', 'visualised': 'visualized',
    'generalise': 'generalize', 'generalised': 'generalized',
    'characterise': 'characterize', 'characterised': 'characterized',
    'criticise': 'criticize', 'criticised': 'criticized',
    'maximise': 'maximize', 'maximised': 'maximized', 'maximising': 'maximizing',
    'minimise': 'minimize', 'minimised': 'minimized', 'minimising': 'minimizing',
    'realise': 'realize', 'realised': 'realized', 'realising': 'realizing',
    'specialise': 'specialize', 'specialised': 'specialized',
    'standardise': 'standardize', 'standardised': 'standardized',
    'familiarise': 'familiarize', 'familiarised': 'familiarized',
    'tokenise': 'tokenize', 'tokenised': 'tokenized',
    'randomise': 'randomize', 'randomised': 'randomized',
    'formalise': 'formalize', 'formalised': 'formalized',
    'finalise': 'finalize', 'finalised': 'finalized',
    'penalise': 'penalize', 'penalised': 'penalized',
    'synchronise': 'synchronize', 'synchronised': 'synchronized',
    'categorise': 'categorize', 'categorised': 'categorized',
    'customise': 'customize', 'customised': 'customized',
    'apologise': 'apologize', 'apologised': 'apologized',
    'utilise': 'utilize', 'utilised': 'utilized',
    'practise': 'practice', 'practised': 'practiced', 'practising': 'practicing',
    'enquire': 'inquire', 'enquiry': 'inquiry', 'enquiries': 'inquiries',
    'fulfil': 'fulfill', 'instalment': 'installment', 'instalments': 'installments',
    'enrolment': 'enrollment', 'skilful': 'skillful', 'wilful': 'willful',
    'marvellous': 'marvelous', 'counsellor': 'counselor',
    'signalling': 'signaling', 'signalled': 'signaled',
    'levelling': 'leveling', 'levelled': 'leveled',
    'fuelling': 'fueling', 'fuelled': 'fueled',
    'sizeable': 'sizable', 'calibre': 'caliber', 'metres': 'meters', 'litres': 'liters',
    'cheque': 'check', 'cheques': 'checks', 'maths': 'math',
    'sceptic': 'skeptic', 'sceptical': 'skeptical', 'sceptics': 'skeptics',
    'aeroplane': 'airplane', 'anticlockwise': 'counterclockwise',
    'jewellery': 'jewelry', 'moustache': 'mustache', 'plough': 'plow',
    'storeys': 'stories', 'tyre': 'tire', 'tyres': 'tires',
    'aluminium': 'aluminum', 'sulphur': 'sulfur', 'mould': 'mold', 'moulded': 'molded',
    'draught': 'draft', 'draughts': 'drafts', 'kerb': 'curb', 'gaol': 'jail',
    'tonne': 'ton', 'tonnes': 'tons', 'flavour': 'flavor', 'flavours': 'flavors',
    'rumour': 'rumor', 'rumours': 'rumors', 'vapour': 'vapor', 'vigour': 'vigor',
    'endeavour': 'endeavor', 'endeavours': 'endeavors', 'saviour': 'savior',
    'splendour': 'splendor', 'harbour': 'harbor', 'tumour': 'tumor', 'odour': 'odor',
    'humour': 'humor', 'armour': 'armor', 'speciality': 'specialty',
}


# ---------------------------------------------------------------- the regular families
#
# **A hand-written list of inflections certified this repository American while it was not.**
# Review found `generalises`, `generalising`, `characterises`, `criticises`,
# `canonicalisation` and `synthesised` still in the tree with the gate green - because the map
# held `generalise` and `generalised` and neither the third person nor the gerund. One verb has
# eight forms, and somebody writing them out by hand gets five.
#
# So the regular families are **generated from a stem**. Adding a verb is one entry and every
# form arrives with it, which is the difference between a rule and a list.


def _ise(stems):
    """`-ise` verbs and everything that grows off them.

    `general` gives generalise, generalises, generalised, generalising, generalisation,
    generalisations, generaliser, generalisers. Some of those are not words - `rasterisation`
    is, `synthesisation` is not - and an entry nothing ever matches costs nothing, where
    deciding case by case which forms a person might type costs exactly what this is fixing.
    """
    out = {}
    for stem in stems:
        for british, american in (
            ('ise', 'ize'), ('ises', 'izes'), ('ised', 'ized'), ('ising', 'izing'),
            ('isation', 'ization'), ('isations', 'izations'),
            ('iser', 'izer'), ('isers', 'izers'),
        ):
            out[stem + british] = stem + american
    return out


def _yse(stems):
    """`-yse` verbs. The plural-ambiguous forms are removed below."""
    out = {}
    for stem in stems:
        for british, american in (
            ('yse', 'yze'), ('ysed', 'yzed'), ('ysing', 'yzing'),
            ('yses', 'yzes'), ('yser', 'yzer'), ('ysers', 'yzers'),
        ):
            out[stem + british] = stem + american
    return out


def _our(words):
    """`-our` nouns and what hangs off them."""
    out = {}
    for word in words:
        stem = word[: -len('our')]
        for british, american in (
            ('our', 'or'), ('ours', 'ors'), ('oured', 'ored'), ('ouring', 'oring'),
            ('ourful', 'orful'), ('ourless', 'orless'), ('ourable', 'orable'),
            ('ourite', 'orite'), ('ourites', 'orites'),
        ):
            out[stem + british] = stem + american
    return out


def _double_l(stems):
    """British doubles the `l` before a suffix where American does not."""
    out = {}
    for stem in stems:
        for british, american in (
            ('lled', 'led'), ('lling', 'ling'), ('ller', 'ler'), ('llers', 'lers'),
        ):
            out[stem + british] = stem + american
    return out


ISE_STEMS = (
    'apolog', 'author', 'canonical', 'capital', 'categor', 'central', 'character', 'civil',
    'colon', 'commercial', 'computer', 'contextual', 'critic', 'custom', 'decentral',
    'demoral', 'deserial', 'digit', 'emphas', 'empath', 'equal', 'familiar', 'final',
    'formal', 'general', 'global', 'granular', 'harmon', 'hospital', 'human', 'idol',
    'immun', 'individual', 'industrial', 'initial', 'internal', 'international', 'ital',
    'legal', 'legitim', 'liberal', 'local', 'marginal', 'material', 'maxim', 'memor',
    'minim', 'mobil', 'modern', 'modular', 'monet', 'moral', 'nation', 'national',
    'natural', 'neutral', 'normal', 'optim', 'organ', 'parameter', 'patron', 'penal',
    'personal', 'polar', 'popular', 'priorit', 'privat', 'random', 'raster', 'rational',
    'real', 'recogn', 'regular', 'revolution', 'ritual', 'sanit', 'scrutin', 'secular',
    'serial', 'social', 'special', 'stabil', 'standard', 'steril', 'subsid', 'summar',
    'symbol', 'sympath', 'synchron', 'synthes', 'systemat', 'tantal', 'theor', 'token',
    'total', 'trivial', 'urban', 'util', 'vandal', 'vector', 'verbal', 'victim', 'visual',
    'vocal',
)

YSE_STEMS = ('anal', 'paral', 'catal', 'dial', 'breathal')

OUR_WORDS = (
    'colour', 'behaviour', 'favour', 'honour', 'labour', 'neighbour', 'rigour', 'vigour',
    'valour', 'rumour', 'vapour', 'flavour', 'odour', 'ardour', 'armour', 'clamour',
    'endeavour', 'fervour', 'harbour', 'humour', 'parlour', 'rancour', 'saviour',
    'splendour', 'succour', 'tumour', 'candour', 'demeanour', 'enamour',
)

LL_STEMS = (
    'cance', 'labe', 'leve', 'mode', 'signa', 'trave', 'fue', 'jewe', 'marve', 'counse',
    'quarre', 'tota', 'unrave', 'grove', 'shrive',
)

# The explicit map wins over anything a generator produced.
_derived = {}
_derived.update(_ise(ISE_STEMS))
_derived.update(_yse(YSE_STEMS))
_derived.update(_our(OUR_WORDS))
_derived.update(_double_l(LL_STEMS))
_derived.update(BRITISH)
BRITISH = _derived

# **Removed whatever a generator produced.** `analyses` is the American plural of *analysis* as
# well as the British verb and nothing can tell those apart; `vaporise` is already the American
# spelling; `cancellation` doubles its `l` in both. Flagging any of them would make the gate
# wrong in a way a reader cannot argue with, which is how a gate gets switched off.
for _same_in_both in (
    'analyses', 'vaporise', 'vaporises', 'vaporised', 'vaporising', 'vaporisation',
    'vaporisations', 'vaporiser', 'vaporisers', 'cancellation', 'cancellations',
):
    BRITISH.pop(_same_in_both, None)

# **One word at a time, looked up - not one alternation of every word.**
#
# The first version was `(?<![A-Za-z])(word|word|...)(?-i:(?![a-z]))`, which is O(text x words):
# at every position the engine tries each branch. That was tolerable at 240 words and is not at
# 1271 - deriving the inflections made the alternation five times wider, and the gate went from
# a hundred seconds to not finishing. Scanning for *a word* is one character class, and asking
# whether that word is British is a dict lookup, so the cost stops depending on the list at all.
#
# The pattern splits `camelCase` as well as `snake_case`, which is what the alternation's
# lookarounds were for: `normaliseText` gives `normalise` and `Text`, `normalise_text` gives
# `normalise` and `text`. **`aria-labelledby` gives `labelledby`**, one token that is not in the
# map - so the attribute is spared structurally rather than by a lookahead somebody has to get
# right. `ALLCAPS` stays whole, so `CATALOGUE` is found.
WORD = re.compile('[A-Z]?[a-z]+|[A-Z]+(?![a-z])')

# Other people's frozen HTML. See the module docstring.
SKIP_DIRS = ('crates/landscape-golden/pages/',)

# **Some passages have to write the British forms to say what they are about** - the register
# entry about this sweep, the benchmark run that describes it, the comment in `verify.py` above
# the gate. They mark the passage, not the file:
#
#     american-spelling: off
#     ... a paragraph that quotes `analyse` and `catalogue` ...
#     american-spelling: on
#
# Both markers are comments in every format this repository uses - `<!-- -->` in Markdown, `#`
# in Python, `//` in Rust - so nothing shows in the rendered page. **An `off` with no `on` is a
# failure**, not a licence to skip the rest of the file: that is the only way a marker can turn
# into a silent hole, so it is the one thing checked before anything else.
MUTE = 'american-spelling: off'
UNMUTE = 'american-spelling: on'

# One file is exempt whole, because the exemption *is* the file: the word list below is 240
# British spellings, and bracketing it would mean bracketing everything.
EXEMPT = {
    'scripts/american_spelling.py': 'the word list itself - every key is a British spelling',
}

# Base64 alphabet. A run of these longer than DATA_RUN is data, not English.
#
# **Masked once per line rather than re-walked per match.** The first version asked, for every
# candidate word, whether that position sat inside a long run - and answering walked to both
# ends of the run. On `prototype/demo-idea.html` that is a **seven-million-character line**, so
# three candidate matches cost fifteen seconds of walking the same blob three times, and the
# whole gate took about a hundred seconds on every verify and every CI run. Substituting spaces
# of equal length costs one pass, keeps every offset and line number exactly where it was, and
# takes the gate to about a second.
DATA_RUN = 48
DATA = re.compile('[A-Za-z0-9+/=]{' + str(DATA_RUN + 1) + ',}')

# **The first version measured the whitespace-delimited run and skipped anything over 60**, which
# is fine for prose and wrong for code: `normalise("HTTPS://WWW.Example.com/Pricing")...` is 69
# characters with no space in it, so a real call site went unswept and the compiler found it
# instead of this gate. A base64 run has no punctuation in it - that is what separates data from
# a line of code, and it is what this measures now. The longest identifier here is 30 characters.


def without_data(line: str) -> str:
    """The line with every long base64 run replaced by spaces of the same length.

    Same length on purpose: every offset and every line number stays where it was, so nothing
    downstream has to know this happened. The prototype pages embed video and captions as data
    URIs, and a long enough blob contains every short word there is - `greYTK`, `kErb` and
    `oMOuldq` are all real matches from one.
    """
    return DATA.sub(lambda run: ' ' * (run.end() - run.start()), line)


def names_both(line: str) -> bool:
    """A line that mentions both markers is writing *about* them, not using them.

    `docs/CODING_QUALITY.md` says a passage "brackets itself with `american-spelling: off` and
    `american-spelling: on`", and the first version of this read that sentence as an unclosed
    mute and failed - which it reported, correctly, on the first run after the sentence was
    written. A line carrying both changes nothing and is not itself scanned.
    """
    return MUTE in line and UNMUTE in line


def unclosed_mute(text: str) -> int | None:
    """The line of an `off` marker that is never turned back on, if there is one."""
    opened = None
    for n, line in enumerate(text.split(chr(10)), 1):
        if names_both(line):
            continue
        if MUTE in line:
            opened = n
        elif UNMUTE in line:
            opened = None
    return opened


def offenders(text: str) -> list[tuple[int, str, str]]:
    """Every British spelling in `text`, as `(line, found, american)`.

    Line by line rather than over the whole document, because the mute markers are line-scoped
    and because a base64 blob is one line however long it is.
    """
    found = []
    muted = False
    for n, line in enumerate(text.split(chr(10)), 1):
        if names_both(line):
            continue
        if MUTE in line:
            muted = True
            continue
        if UNMUTE in line:
            muted = False
            continue
        if muted:
            continue
        # Cheap first: a line with no letters at all is most of a base64 blob's neighbourhood.
        for m in WORD.finditer(without_data(line)):
            word = m.group(0)
            american = BRITISH.get(word.lower())
            if american is not None:
                found.append((n, word, american))
    return found


FIXTURES = [
    ('the button that started this', 'press Analyse', ['Analyse']),
    ('the American form passes', 'press Analyze', []),
    ('an identifier with an underscore is reached',
     'fn normalise_text() {}', ['normalise']),
    ('an identifier in camelCase is reached', 'const analyseNow = 1;', ['analyse']),
    ('aria-labelledby is HTML, not English',
     '<section aria-labelledby="examples-heading">', []),
    ('labelled on its own is still caught', 'a labelled claim', ['labelled']),
    ('analyses is the plural noun and is not flagged', 'two analyses a day', []),
    ('analysis is the same word in both', 'one analysis', []),
    ('cancellation has two l in both', 'a cancellation', []),
    ('optimistic is not an -ise word', 'an optimistic read', []),
    ('a base64 blob is not English',
     'data:image/png;base64,' + 'A' * 40 + 'greYTK' + 'B' * 40, []),
    ('a short line with grey in it is English', 'the button is grey', ['grey']),
    # The call site the first version of this gate skipped, because it measured whitespace and
    # this line is 69 characters without one. The compiler found it; this should have.
    ('a long line of code is still code',
     '            normalise("HTTPS://WWW.Example.com/Pricing").ends_with("/Pricing"),',
     ['normalise']),
    ('a long identifier is not a blob',
     'fn deserialise_the_whole_configuration_document() {}', ['deserialise']),
    ('capitals are matched', 'CATALOGUE', ['CATALOGUE']),
    ('a longer word is not a partial match', 'greyhounds run', []),
    # The four the hand-written list missed, and the reason the families are derived.
    ('a third person the base form does not imply', 'it generalises', ['generalises']),
    ('a gerund the base form does not imply', 'generalising over it', ['generalising']),
    ('a noun the verb does not imply', 'canonicalisation of a URL', ['canonicalisation']),
    ('a past participle two stems along', 'synthesised from the status', ['synthesised']),
    # Removed from whatever the generators produced. Flagging any of these is the way a gate
    # earns being switched off.
    ('the American plural is not a British verb', 'two analyses a day', []),
    ('cancellation doubles its l in both', 'a cancellation policy', []),
    ('vaporise is already American', 'it will vaporise', []),
    ('two on one line are both reported',
     'the colour and the behaviour', ['colour', 'behaviour']),
    ('a muted passage is not checked',
     '<!-- american-spelling: off -->' + chr(10) + 'it said Analyse' + chr(10)
     + '<!-- american-spelling: on -->', []),
    ('the mute ends where it says it does',
     '<!-- american-spelling: off -->' + chr(10) + 'it said Analyse' + chr(10)
     + '<!-- american-spelling: on -->' + chr(10) + 'and here it says colour', ['colour']),
    ('nothing is muted before the marker',
     'colour first' + chr(10) + '# american-spelling: off' + chr(10) + 'analyse'
     + chr(10) + '# american-spelling: on', ['colour']),
    ('a line naming both markers mutes nothing after it',
     'brackets itself with `american-spelling: off` and `american-spelling: on`' + chr(10)
     + 'and then a colour', ['colour']),
]

# `(name, text, the line an unclosed marker sits on, or None)`. **An `off` with no `on` is the
# one way a marker becomes a silent hole**, so it has fixtures of its own.
MARKER_FIXTURES = [
    ('balanced',
     '# american-spelling: off' + chr(10) + 'x' + chr(10) + '# american-spelling: on', None),
    ('an off with no on', 'x' + chr(10) + '# american-spelling: off' + chr(10) + 'y', 2),
    ('no markers at all', 'x' + chr(10) + 'y', None),
    ('a second pair left open',
     '# american-spelling: off' + chr(10) + '# american-spelling: on' + chr(10)
     + '# american-spelling: off', 3),
    ('a line naming both markers is documentation, not a mute',
     'brackets itself with `american-spelling: off` and `american-spelling: on`', None),
]


def self_test() -> list[str]:
    """The gate checked against its own examples, before it checks anything else."""
    broken = []
    for name, text, want_line in MARKER_FIXTURES:
        got = unclosed_mute(text)
        if got != want_line:
            broken.append(f'  {name}: wanted {want_line}, got {got}')
    for name, text, want in FIXTURES:
        got = [w for _, w, _ in offenders(text)]
        if got != want:
            broken.append(f'  {name}: wanted {want}, got {got}')
    return broken


def tracked() -> list[str]:
    """Every file in the repository, **including ones not committed yet**.

    `git ls-files` alone lists what is *tracked*, and a brand-new file is not - so the gate was
    blind to exactly the file most likely to carry the mistake, right up until the moment it was
    committed and CI saw it. That is what happened: a new module went in with `behaviour` in a
    comment, five local runs said `none found`, and the build went red on the push.

    `--others --exclude-standard` adds the untracked files that are not ignored, which is the
    set a person means by "the files I am working on". Entry 16 of the register is the same
    lesson with the halves swapped: a check whose idea of "the code" differs from CI's is a
    check that reports on something nobody is shipping.
    """
    out = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard'],
        text=True,
    )
    # `--cached --others` can name the same path twice; `dict.fromkeys` keeps the order.
    return list(dict.fromkeys(p for p in out.split(chr(10)) if p))


def main() -> int:
    broken = self_test()
    if broken:
        print('This gate no longer agrees with its own examples:' + chr(10))
        print(chr(10).join(broken))
        return 1

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(root)

    hits = []
    open_markers = []
    muted = 0
    frozen = 0
    unreadable = []
    checked = 0
    for path in tracked():
        if path in EXEMPT:
            continue
        if any(path.startswith(d) for d in SKIP_DIRS):
            frozen += 1
            continue
        try:
            text = io.open(path, encoding='utf-8').read()
        except UnicodeDecodeError:
            # Not text. The films and the recorded video are the whole of this in practice, and
            # a count is the honest report: naming forty binaries every run is noise.
            unreadable.append((path, 'not text'))
            continue
        except OSError as why:
            # **Named, unlike the binaries.** A file this cannot open is a fact about the
            # working tree rather than about the file's type, and silently skipping it is how a
            # gate reports coverage it does not have.
            unreadable.append((path, str(why)))
            continue
        checked += 1
        dangling = unclosed_mute(text)
        if dangling is not None:
            open_markers.append((path, dangling))
            continue
        # Counting `text.count(MUTE)` would include the sentences that *describe* the markers,
        # and this number exists to be trusted rather than approximately right.
        muted += sum(
            1 for line in text.split(chr(10)) if MUTE in line and not names_both(line)
        )
        for line, found, american in offenders(text):
            hits.append((path, line, found, american))

    # Checked before the words, because an unclosed `off` mutes everything below it and would
    # otherwise make this gate quieter the more broken it is.
    if open_markers:
        print('An `american-spelling: off` is never turned back on. Everything below it in'
              + chr(10) + 'the file is unchecked, which is the one thing a marker must not do:'
              + chr(10))
        for path, line in open_markers:
            print(f'  {path}:{line}')
        return 1

    if not hits:
        # **Everything skipped is printed, and that sentence has to be true.** An earlier version
        # said it while silently passing over the frozen pages and every binary in the tree, so a
        # green run presented known blind spots as coverage. Review found it.
        print(
            f'American spelling: {len(BRITISH)} words checked '
            f'across {checked} text file(s), none found.'
        )
        for path, why in EXEMPT.items():
            print(f'  not checked: {path} - {why}')
        if frozen:
            print(f'  not checked: {frozen} file(s) under ' + ', '.join(SKIP_DIRS))
        if unreadable:
            not_text = [p for p, why in unreadable if why == 'not text']
            if not_text:
                print(f'  not checked: {len(not_text)} file(s) that are not text')
            for path, why in unreadable:
                if why != 'not text':
                    print(f'  not checked: {path} - {why}')
        if muted:
            print(f'  not checked: {muted} passage(s) marked `{MUTE}`')
        return 0

    print(f'{len(hits)} British spelling(s). This repository writes American:' + chr(10))
    for path, line, found, american in hits:
        print(f'  {path}:{line}  {found} -> {american}')
    print(
        chr(10) + 'If one of these is a proper noun, a quotation, or somebody else\'s '
        'identifier,' + chr(10) + 'it needs an exclusion here rather than a different spelling.'
    )
    return 1


if __name__ == '__main__':
    sys.exit(main())
