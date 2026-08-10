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


def wrong(page):
    lines = io.open(page, encoding='utf-8').read().split(chr(10))
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
                f'its six state rows sum to {adds[0]} / {adds[1]} / {adds[2]}'
            )
        else:
            percent(problems, at, 'the Summary total', adds[1], adds[0], int(row.group('pct')))
    return problems


def main():
    problems = wrong(PAGE)
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
    print(f'{tables_found} feature tables: every total is the sum of its rows, and the '
          f'Summary has {len(states)} states, and its total is the sum of those.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
