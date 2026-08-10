"""Every totals row in `docs/Full_Feature_List.md`, added up against its own columns.

**A percentage nobody adds up is a percentage that drifts.** S2's totals row read `18 / 16 / 2`
while its ten feature rows summed to `19 / 16 / 3`, so the page published 89% for a state that
was 84% — and every pull request that quoted it repeated the number. Nothing here was wrong on
purpose; a row was added once and the total below it was not.

This is the same shape as the benchmark-count gate: a document that must hold a number, checked
against the thing the number is derived from rather than trusted. The difference is that the
source here is the document's own rows, so no test run is needed and it costs nothing.

**And then the summary of the summaries drifted.** The first version of this checked each state
against its own feature rows and stopped there, so the Summary table at the top of the page — the
one a reader sees first, and the one every status report quotes — went on saying S2 was 78% for
four pull requests after its own table said 89%. A gate that checks the derived numbers but not
the number derived from *those* leaves the most-read row of the page unchecked, which is where
drift is worth the least. The Summary is now checked against the per-state tables, and the total
row against the Summary.

    python3 scripts/feature_totals.py
"""

import collections
import io
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PAGE = os.path.join(HERE, '..', 'docs', 'Full_Feature_List.md')

# `| Feature | State | Est. PRs | Done | Left | Complete |`, with the numbers optionally bold.
FEATURE = re.compile(
    r'^\|\s*(?P<feature>[^|]+?)\s*\|\s*(?P<state>S\d)\s*\|\s*'
    r'\*{0,2}(?P<est>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<done>\d+)\*{0,2}\s*\|\s*'
    r'\*{0,2}(?P<left>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<pct>\d+)%\*{0,2}\s*\|\s*$'
)
# `| | | **19** | **17** | **2** | **89%** |` — the row with no feature and no state.
TOTAL = re.compile(
    r'^\|\s*\|\s*\|\s*\*{0,2}(?P<est>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<done>\d+)\*{0,2}\s*\|\s*'
    r'\*{0,2}(?P<left>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<pct>\d+)%\*{0,2}\s*\|\s*$'
)
# The Summary at the top: `| [**S2**](#s2--…) | Any business idea handled correctly | 19 | … |`.
# Anchored on the link, so the italic uncounted row below it ("not a state") is not a state.
SUMMARY = re.compile(
    r'^\|\s*\[\*\*(?P<state>S\d)\*\*\]\(#[^)]*\)\s*\|[^|]*\|\s*'
    r'\*{0,2}(?P<est>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<done>\d+)\*{0,2}\s*\|\s*'
    r'\*{0,2}(?P<left>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<pct>\d+)%\*{0,2}\s*\|\s*$'
)
# `| | **Total** | **131** | **58** | **73** | **44%** |` — the Summary's own bottom line.
GRAND = re.compile(
    r'^\|\s*\|\s*\*{0,2}Total\*{0,2}\s*\|\s*\*{0,2}(?P<est>\d+)\*{0,2}\s*\|\s*'
    r'\*{0,2}(?P<done>\d+)\*{0,2}\s*\|\s*\*{0,2}(?P<left>\d+)\*{0,2}\s*\|\s*'
    r'\*{0,2}(?P<pct>\d+)%\*{0,2}\s*\|\s*$'
)


def tables(lines):
    """Each totals row with the feature rows above it, in the order they appear."""
    out = []
    rows = []
    for n, line in enumerate(lines, 1):
        feature = FEATURE.match(line)
        if feature:
            rows.append(feature)
            continue
        total = TOTAL.match(line)
        if total:
            out.append((n, rows, total))
            rows = []
    return out


def summarised(lines):
    """The Summary's per-state rows and its Total row, by line number."""
    states = []
    grand = None
    for n, line in enumerate(lines, 1):
        row = SUMMARY.match(line)
        if row:
            states.append((n, row))
            continue
        if grand is None:
            bottom = GRAND.match(line)
            if bottom:
                grand = (n, bottom)
    return states, grand


def counted(row):
    return int(row.group('est')), int(row.group('done')), int(row.group('left'))


def percent(problems, at, what, done, est, claimed):
    if est and round(done * 100 / est) != claimed:
        problems.append(
            f'line {at}: {what} says {claimed}%, '
            f'{done} of {est} is {round(done * 100 / est)}%'
        )


def wrong(lines):
    found = tables(lines)
    if not found:
        return ['no feature table with a totals row was found at all']

    problems = []
    per_state = {}
    for at, rows, total in found:
        if not rows:
            problems.append(f'line {at}: a totals row with no feature rows above it')
            continue
        state = rows[0].group('state')
        est = sum(int(r.group('est')) for r in rows)
        done = sum(int(r.group('done')) for r in rows)
        left = sum(int(r.group('left')) for r in rows)
        # A second table for one state would silently replace the first here, and then agree
        # with whichever Summary row was written last.
        if state in per_state:
            problems.append(f'line {at}: a second {state} table; one state, one table')
            continue
        per_state[state] = (est, done, left)
        claimed = counted(total)
        if claimed != (est, done, left):
            problems.append(
                f'line {at}: {state} totals say {claimed[0]} / {claimed[1]} / {claimed[2]}, '
                f'its {len(rows)} rows sum to {est} / {done} / {left}'
            )
            continue
        # Every row also has to add up on its own: done + left is what was estimated.
        for r in rows:
            if int(r.group('done')) + int(r.group('left')) != int(r.group('est')):
                problems.append(
                    f'{state} "{r.group("feature").strip()}": '
                    f'{r.group("done")} done + {r.group("left")} left is not {r.group("est")}'
                )
        percent(problems, at, state, done, est, int(total.group('pct')))

    # **And the Summary, which is the row anybody actually quotes.** It is derived from the
    # tables above and is not allowed to have its own opinion of them.
    states, grand = summarised(lines)
    if not states:
        return problems + ['the Summary table has no per-state rows at all']
    # **Duplicates first, because everything below is a comparison and a comparison cannot see
    # them.** Each row would be checked against its own table and pass; the total would then be
    # checked against the sum of the rows, and a total inflated to match two copies of S2 passes
    # as well. Nothing is wrong with any single number, and the page says something false.
    listed = collections.Counter(row.group('state') for _, row in states)
    twice = [state for state, times in sorted(listed.items()) if times > 1]
    if twice:
        return problems + [
            f'the Summary lists {state} {listed[state]} times; one state, one row - '
            f'every other check here compares one row at a time and cannot see this'
            for state in twice
        ]

    for at, row in states:
        state = row.group('state')
        if state not in per_state:
            problems.append(f'line {at}: the Summary has a {state} row and the page has no {state} table')
            continue
        if counted(row) != per_state[state]:
            says = counted(row)
            adds = per_state[state]
            problems.append(
                f'line {at}: the Summary says {state} is {says[0]} / {says[1]} / {says[2]}, '
                f'its own table says {adds[0]} / {adds[1]} / {adds[2]}'
            )
            continue
        percent(problems, at, f"the Summary's {state}", counted(row)[1], counted(row)[0],
                int(row.group('pct')))
    for state in sorted(set(per_state) - {row.group('state') for _, row in states}):
        problems.append(f'the page has a {state} table and the Summary does not list it')

    if grand is None:
        problems.append('the Summary has no Total row')
    else:
        at, row = grand
        adds = tuple(sum(c) for c in zip(*(counted(r) for _, r in states)))
        if counted(row) != adds:
            says = counted(row)
            problems.append(
                f'line {at}: the Summary total says {says[0]} / {says[1]} / {says[2]}, '
                f'its {len(states)} state rows sum to {adds[0]} / {adds[1]} / {adds[2]}'
            )
        else:
            percent(problems, at, 'the Summary total', adds[1], adds[0], int(row.group('pct')))
    return problems


# A page with two states, adding up, so each case below can break exactly one thing.
SOUND = [
    '| State | What it means | Est. PRs | Done | Left | Complete |',
    '|---|---|---|---|---|---|',
    '| [**S1**](#s1--a) | first | 2 | 2 | **0** | **100%** |',
    '| [**S2**](#s2--b) | second | 4 | 2 | **2** | **50%** |',
    '| | **Total** | **6** | **4** | **2** | **67%** |',
    '',
    '| one | S1 | 2 | 2 | 0 | 100% |',
    '| | | **2** | **2** | **0** | **100%** |',
    '',
    '| two | S2 | 3 | 2 | 1 | 67% |',
    '| three | S2 | 1 | 0 | 1 | 0% |',
    '| | | **4** | **2** | **2** | **50%** |',
]

# Each is the sound page with one defect put back, and the words the complaint must contain.
# **A gate nobody has seen fail is a gate nobody has seen.**
BROKEN = [
    ('a state totals row that disagrees with its rows',
     ('| | | **4** | **2** | **2** | **50%** |', '| | | **4** | **3** | **1** | **75%** |'),
     'its 2 rows sum to'),
    ('a Summary row that disagrees with its own table',
     ('| [**S2**](#s2--b) | second | 4 | 2 | **2** | **50%** |',
      '| [**S2**](#s2--b) | second | 4 | 3 | **1** | **75%** |'),
     'its own table says'),
    ('a Summary total that disagrees with the Summary',
     ('| | **Total** | **6** | **4** | **2** | **67%** |',
      '| | **Total** | **7** | **4** | **3** | **57%** |'),
     'state rows sum to'),
    ('a percentage that is not the division it claims to be',
     ('| [**S1**](#s1--a) | first | 2 | 2 | **0** | **100%** |',
      '| [**S1**](#s1--a) | first | 2 | 2 | **0** | **80%** |'),
     'is 100%'),
    ('a Summary that leaves a state out',
     ('| [**S2**](#s2--b) | second | 4 | 2 | **2** | **50%** |', ''),
     'the Summary does not list it'),
    ('one state listed twice, with the total inflated to match',
     ('| | **Total** | **6** | **4** | **2** | **67%** |',
      '| [**S2**](#s2--b) | again | 4 | 2 | **2** | **50%** |' + chr(10)
      + '| | **Total** | **10** | **6** | **4** | **60%** |'),
     'one state, one row'),
    ('one state given two tables',
     ('| two | S2 | 3 | 2 | 1 | 67% |' + chr(10) + '| three | S2 | 1 | 0 | 1 | 0% |' + chr(10)
      + '| | | **4** | **2** | **2** | **50%** |',
      '| two | S2 | 4 | 2 | 2 | 50% |' + chr(10) + '| | | **4** | **2** | **2** | **50%** |'
      + chr(10) + chr(10) + '| three | S2 | 4 | 2 | 2 | 50% |' + chr(10)
      + '| | | **4** | **2** | **2** | **50%** |'),
     'one state, one table'),
]


def self_test():
    """Break the sound page seven ways and check that each complaint arrives.

    This runs before the real page, every time, because **a gate that has quietly stopped
    detecting anything still exits 0**. The duplicate-row case is here because it got past the
    first version of this file: each Summary row was checked against its own table and passed,
    and the total against the sum of the rows, so a Summary listing S2 twice with a total
    inflated to match was clean by every comparison the file made. Every number was right and
    the page was wrong, which is the shape a per-row check cannot see.
    """
    failures = []
    unsound = wrong(SOUND)
    if unsound:
        failures.append(f'the sound page is reported as broken: {unsound}')
    for name, (old, new), expected in BROKEN:
        page = chr(10).join(SOUND)
        if page.count(old) != 1:
            failures.append(f'{name}: its anchor is not in the sound page exactly once')
            continue
        said = wrong(page.replace(old, new, 1).split(chr(10)))
        if not any(expected in problem for problem in said):
            failures.append(f'{name}: nothing said {expected!r} - got {said}')
    return failures


def main():
    failures = self_test()
    if failures:
        print('This gate no longer detects what it is for:' + chr(10))
        for failure in failures:
            print('  ' + failure)
        return 1

    problems = wrong(io.open(PAGE, encoding='utf-8').read().split(chr(10)))
    if problems:
        print('A feature table does not add up:' + chr(10))
        for problem in problems:
            print('  ' + problem)
        print(chr(10) + 'A percentage nobody adds up is a percentage that drifts, and every')
        print('pull request that quotes it repeats the drift.')
        return 1
    lines = io.open(PAGE, encoding='utf-8').read().split(chr(10))
    tables_found = len(tables(lines))
    states, _ = summarised(lines)
    print(f'{len(BROKEN)} defects put back and caught. '
          f'{tables_found} feature tables: every total is the sum of its rows, and the '
          f'Summary has {len(states)} states, and its total is the sum of those.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
