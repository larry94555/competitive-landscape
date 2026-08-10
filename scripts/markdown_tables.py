"""Every Markdown table in this repository, checked for the row that makes it a table.

**A table without its `|---|---|` line renders as a wall of pipes**, and only in a browser —
the text looks fine in an editor, in a diff, and in every review that reads the source. GitHub
is where these documents are read, so that is the rendering that decides whether they are
readable at all.

Two were found the day this was written, and neither was noticed by a person:

  * `docs/USING_THE_SITE.md` — a two-row table typed without the separator, which is the
    ordinary slip.
  * `docs/ROADMAP.md` — a table **split in half** by a paragraph inserted into the middle of
    it, so its last two rows had been rendering as text for as long as the paragraph had been
    there. Nobody edits a table by deleting its separator; they edit it by putting something
    where the separator's protection does not reach.

# What it checks, exactly

**This repository writes every table with outer pipes**, and this gate holds it to that:

    | Header | Header |
    |---|---|
    | cell   | cell   |

A run of two or more consecutive lines that begin and end with `|` must have a **GFM delimiter
row** second — each cell `-`, `--`, `:---`, `---:` or `:---:`. The first version accepted any
run of `-`, `:`, `|` and spaces, so `| : |` passed and was not a table; GFM requires at least
one hyphen per cell and that is what is required here.

**Tables without outer pipes are valid GFM and this repository does not use them.** `A | B`
over `--- | ---` renders perfectly well, and treating any line containing a pipe as a table row
would flag every sentence with a pipe in it. So the convention is the narrower one, and a
delimiter row that arrives *without* outer pipes is reported — not because it is wrong, but
because it is a table this gate cannot check, and a check with a silent blind spot is worse
than one that names it.

**It does not validate column counts or alignment.** A row with the wrong number of cells still
renders as a table, and the distinction this exists for is *table or not a table*.

    python3 scripts/markdown_tables.py
"""

import io
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# One delimiter cell, per GFM: optional colons around one or more hyphens.
CELL = r':?-+:?'
# `|---|`, `| :--- | ---: |`, `|-|-|`. Outer pipes required, which is this repository's shape.
SEPARATOR = re.compile(rf'^\|(?:\s*{CELL}\s*\|)+$')
# The same thing without outer pipes: valid GFM, not our convention, and unchecked.
BARE_SEPARATOR = re.compile(rf'^\s*{CELL}\s*(?:\|\s*{CELL}\s*)+$')

SKIP = {'.git', 'node_modules', 'target', 'dist', '.venv'}

# **Frozen copies of real websites, not prose we wrote.** `landscape-golden/pages` holds pages
# captured from the live web so extraction can be scored against them; one of them contains a
# table without outer pipes, which is how that site wrote it. Editing it to please a linter
# would be editing the evidence.
NOT_OURS = ('crates/landscape-golden/pages',)

# ``` and ~~~, and the indented form. All three can hold a line full of pipes that is not a
# table — a shell heredoc, a rendered example, a diff. The first version knew only about
# three backticks at the start of a line.
FENCE = re.compile(r'^\s{0,3}(`{3,}|~{3,})')


def without_code(lines):
    """The lines, with fenced and indented code blocks blanked out."""
    out = []
    closing = None
    for line in lines:
        if closing is not None:
            out.append('')
            if line.strip().startswith(closing):
                closing = None
            continue
        opened = FENCE.match(line)
        if opened:
            closing = opened.group(1)[0] * 3
            out.append('')
            continue
        # An indented code block: four spaces or a tab, and this repository only ever indents
        # that far inside one.
        out.append('' if line.startswith('    ') or line.startswith(chr(9)) else line)
    return out


def runs(lines):
    """Consecutive lines that begin and end with `|`, with the line each run starts on."""
    found = []
    run = []
    at = 0
    for n, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith('|') and stripped.endswith('|') and len(stripped) > 1:
            if not run:
                at = n
            run.append(stripped)
            continue
        if run:
            found.append((at, run))
            run = []
    if run:
        found.append((at, run))
    return found


def wrong_in(lines):
    """`(tables, problems)` for one file's lines. Split out so the fixtures can call it."""
    tables = 0
    problems = []
    clean = without_code(lines)
    for at, run in runs(clean):
        # One `|` line on its own is not a table anybody meant to write.
        if len(run) < 2:
            continue
        tables += 1
        if not SEPARATOR.match(run[1]):
            problems.append((at, run[0][:60], 'no separator row'))
    for n, line in enumerate(clean, 1):
        if BARE_SEPARATOR.match(line.strip()):
            problems.append((n, line.strip()[:60], 'a table without outer pipes'))
    return tables, problems


def pages():
    for base, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in SKIP]
        here = os.path.relpath(base, ROOT).replace(os.sep, '/')
        if here.startswith(NOT_OURS):
            continue
        for name in sorted(files):
            if name.endswith('.md'):
                yield os.path.join(base, name)


# Each fixture is the text, and what the check must say about it. **The gate is only as good as
# the shapes it has been shown**, and the first version of it was wrong about three of these.
FIXTURES = [
    ('a table with its separator', '| a | b |' + chr(10) + '|---|---|' + chr(10) + '| 1 | 2 |', []),
    ('a table with no separator', '| a | b |' + chr(10) + '| 1 | 2 |', ['no separator row']),
    ('one hyphen per cell is enough for GFM', '| a | b |' + chr(10) + '|-|-|', []),
    ('alignment colons', '| a | b |' + chr(10) + '| :--- | ---: |', []),
    ('colons with no hyphen are not a separator', '| a | b |' + chr(10) + '| : | : |',
     ['no separator row']),
    ('a table split by a paragraph',
     '| a | b |' + chr(10) + '|---|---|' + chr(10) + '| 1 | 2 |' + chr(10) + chr(10)
     + 'A paragraph.' + chr(10) + chr(10) + '| 3 | 4 |' + chr(10) + '| 5 | 6 |',
     ['no separator row']),
    ('pipes inside a backtick fence',
     '```' + chr(10) + '| a | b |' + chr(10) + '| 1 | 2 |' + chr(10) + '```', []),
    ('pipes inside a tilde fence',
     '~~~' + chr(10) + '| a | b |' + chr(10) + '| 1 | 2 |' + chr(10) + '~~~', []),
    ('pipes inside an indented block',
     'Example:' + chr(10) + chr(10) + '    | a | b |' + chr(10) + '    | 1 | 2 |', []),
    ('a single pipe line is not a table', '| a | b |', []),
    ('a table without outer pipes is named rather than ignored',
     'a | b' + chr(10) + '--- | ---' + chr(10) + '1 | 2', ['a table without outer pipes']),
]


def self_test():
    """Run every fixture. A gate nobody has seen fail is a gate nobody has seen."""
    failures = []
    for name, text, expected in FIXTURES:
        _, problems = wrong_in(text.split(chr(10)))
        got = sorted(why for _, _, why in problems)
        if got != sorted(expected):
            failures.append(f'{name}: expected {sorted(expected)}, got {got}')
    return failures


def main():
    failures = self_test()
    if failures:
        print('This gate no longer recognises what it is for:' + chr(10))
        for failure in failures:
            print('  ' + failure)
        return 1

    tables = 0
    problems = []
    for path in pages():
        lines = io.open(path, encoding='utf-8').read().split(chr(10))
        found, wrong = wrong_in(lines)
        tables += found
        rel = os.path.relpath(path, ROOT).replace(os.sep, '/')
        problems.extend((rel, at, first, why) for at, first, why in wrong)

    if problems:
        print('A table will not render as one:' + chr(10))
        for path, at, first, why in problems:
            print(f'  {path}:{at}  ({why})')
            print(f'    {first}')
        print(chr(10) + 'The second line of a table has to be its `|---|---|` separator. If a')
        print('table looks complete here, check whether something was inserted into the')
        print('middle of it - that splits one table into two, and the second half has no')
        print('separator of its own.')
        return 1

    print(f'{len(FIXTURES)} shapes checked, {tables} markdown tables: every one renders.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
