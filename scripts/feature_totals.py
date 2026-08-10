"""Every totals row in `docs/Full_Feature_List.md`, added up against its own columns.

**A percentage nobody adds up is a percentage that drifts.** S2's totals row read `18 / 16 / 2`
while its ten feature rows summed to `19 / 16 / 3`, so the page published 89% for a state that
was 84% — and every pull request that quoted it repeated the number. Nothing here was wrong on
purpose; a row was added once and the total below it was not.

This is the same shape as the benchmark-count gate: a document that must hold a number, checked
against the thing the number is derived from rather than trusted. The difference is that the
source here is the document's own rows, so no test run is needed and it costs nothing.

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


def wrong(page):
    lines = io.open(page, encoding='utf-8').read().split(chr(10))
    found = tables(lines)
    if not found:
        return ['no feature table with a totals row was found at all']

    problems = []
    for at, rows, total in found:
        if not rows:
            problems.append(f'line {at}: a totals row with no feature rows above it')
            continue
        state = rows[0].group('state')
        est = sum(int(r.group('est')) for r in rows)
        done = sum(int(r.group('done')) for r in rows)
        left = sum(int(r.group('left')) for r in rows)
        claimed = (int(total.group('est')), int(total.group('done')), int(total.group('left')))
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
        if est and round(done * 100 / est) != int(total.group('pct')):
            problems.append(
                f'line {at}: {state} says {total.group("pct")}%, '
                f'{done} of {est} is {round(done * 100 / est)}%'
            )
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
    tables_found = len(tables(io.open(PAGE, encoding='utf-8').read().split(chr(10))))
    print(f'{tables_found} feature tables: every total is the sum of its rows.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
