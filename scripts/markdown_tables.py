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

The check is deliberately narrow. It does not validate column counts or alignment: a row with
the wrong number of cells still renders as a table, and the point here is the difference between
*a table* and *not a table*.

    python3 scripts/markdown_tables.py
"""

import io
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# `|---|`, `| :--- | ---: |`, and the rest of what GitHub accepts as a delimiter row.
SEPARATOR = re.compile(r'^\|[\s:|-]+\|$')

SKIP = {'.git', 'node_modules', 'target', 'dist', '.venv'}


def without_code(lines):
    """The lines, with fenced blocks blanked out.

    A shell heredoc or a Markdown example can contain pipes, and neither is a table.
    """
    out = []
    fenced = False
    for line in lines:
        if line.startswith('```'):
            fenced = not fenced
            out.append('')
            continue
        out.append('' if fenced else line)
    return out


def runs(lines):
    """Consecutive table-looking lines, with the line number each run starts on."""
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


def pages():
    for base, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in SKIP]
        for name in sorted(files):
            if name.endswith('.md'):
                yield os.path.join(base, name)


def wrong():
    problems = []
    tables = 0
    for path in pages():
        lines = io.open(path, encoding='utf-8').read().split(chr(10))
        for at, run in runs(without_code(lines)):
            # One `|` line on its own is not a table anybody meant to write.
            if len(run) < 2:
                continue
            tables += 1
            if not SEPARATOR.match(run[1]):
                problems.append(
                    (os.path.relpath(path, ROOT).replace(os.sep, '/'), at, run[0][:60])
                )
    return tables, problems


def main():
    tables, problems = wrong()
    if problems:
        print('A table will render as a row of pipes rather than a table:' + chr(10))
        for path, at, first in problems:
            print(f'  {path}:{at}')
            print(f'    {first}')
        print(chr(10) + 'The second line of a table has to be its `|---|---|` separator. If a')
        print('table looks complete here, check whether something was inserted into the')
        print('middle of it - that splits one table into two, and the second half has no')
        print('separator of its own.')
        return 1
    print(f'{tables} markdown tables: every one has its separator row.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
