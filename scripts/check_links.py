#!/usr/bin/env python3
"""Check that every internal link in the documentation goes somewhere.

    python3 scripts/check_links.py                 # every tracked .md
    python3 scripts/check_links.py docs/ROADMAP.md # just these

Exit code 1 if any link is broken, with the file, line and target.

Scope, and why it stops where it does:

- **Relative links to files** are checked — the target must exist on disk.
- **Anchors** are checked against the headings of the file they point into. This is the
  half that matters. A link to a file that was deleted breaks loudly the first time
  somebody clicks it; a link to `#4.7` in a document that renumbered its sections lands
  the reader at the top of a long page with no sign anything went wrong, and they conclude
  the reference was vague rather than broken.
- **`http(s)` links are not fetched.** A checker that reaches the network fails when a
  third party has an outage, and a check that goes red for reasons outside the repository
  is a check people learn to re-run rather than read. Their *shape* is validated, which
  catches the real local mistake: a bare `www.example.com` that renders as text.

`ROADMAP.md` Phase 0 asks for this alongside `docs/TUTORIAL.md`, for a reason this project
keeps rediscovering: every documentation bug that has reached a reader here was in prose
nobody executed.
"""

from __future__ import annotations

import argparse
import io
import os
import re
import subprocess
import sys

# [text](target) — but not ![image](target), and not a reference-style definition.
LINK = re.compile(r'(?<!\!)\[(?P<text>[^\]]*)\]\((?P<target>[^)\s]+)(?:\s+"[^"]*")?\)')

# Fenced code blocks. A link inside one is an example, not a link.
FENCE = re.compile(r'^\s*```')


def tracked_markdown() -> list[str]:
    """Every .md file in the working tree that git is not ignoring.

    `--others --exclude-standard` includes files that are new and not yet added. Without
    it a document is unchecked for exactly as long as it is newest — which is when its
    links are most likely to be wrong and when the author is still in a position to care.
    `.gitignore` is still honored, so scratch files and `node_modules` stay out.
    """
    try:
        out = subprocess.run(
            ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '*.md'],
            capture_output=True, text=True, check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return []
    # `--cached --others` can list the same path twice once a new file is staged.
    return sorted({p for p in out.splitlines() if p})


def headings(path: str) -> set[str]:
    """Every anchor GitHub would generate for this file.

    GitHub lowercases, drops anything that is not a letter, digit, space, hyphen or
    underscore, then turns spaces into hyphens. Reimplemented rather than imported so the
    checker has no dependencies — it has to run in CI on a cold machine.
    """
    anchors: set[str] = set()
    try:
        body = io.open(path, encoding='utf-8').read()
    except OSError:
        return anchors

    in_fence = False
    for line in body.splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence or not line.startswith('#'):
            continue
        title = line.lstrip('#').strip()
        slug = title.lower()
        slug = re.sub(r'[^\w\- ]', '', slug, flags=re.UNICODE)
        slug = slug.replace(' ', '-')
        if slug:
            anchors.add(slug)
            # GitHub disambiguates repeats with -1, -2. Accept a few so a document with
            # two "Consequences" headings does not report a false break.
            for n in range(1, 6):
                anchors.add(f'{slug}-{n}')
    # An explicit <a name="..."> or id="..." is an anchor too.
    for m in re.finditer(r'<a\s+(?:name|id)="([^"]+)"', body):
        anchors.add(m.group(1).lower())
    return anchors


def links(text: str) -> list[tuple[int, str, str]]:
    """(line number, link text, target) for every link outside a code fence."""
    out: list[tuple[int, str, str]] = []
    in_fence = False
    for n, line in enumerate(text.splitlines(), start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for m in LINK.finditer(line):
            out.append((n, m.group('text'), m.group('target')))
    return out


def check(path: str, anchor_cache: dict[str, set[str]]) -> list[str]:
    problems: list[str] = []
    try:
        text = io.open(path, encoding='utf-8').read()
    except OSError as e:
        return [f'{path}: cannot read: {e}']

    here = os.path.dirname(path) or '.'

    for line_no, label, target in links(text):
        at = f'{path}:{line_no}'

        if target.startswith(('http://', 'https://')):
            continue
        if target.startswith(('mailto:', 'tel:')):
            continue
        # A link that looks like a bare domain renders as text and fools the author into
        # thinking it is a link.
        if re.match(r'^www\.', target):
            problems.append(f'{at}: [{label}]({target}) has no scheme, so it is not a link')
            continue

        file_part, _, anchor = target.partition('#')

        if not file_part:
            # An anchor within this same file.
            resolved = path
        else:
            resolved = os.path.normpath(os.path.join(here, file_part))
            if not os.path.exists(resolved):
                problems.append(f'{at}: [{label}]({target}) -> no such file: {resolved}')
                continue

        if anchor and resolved.endswith('.md'):
            if resolved not in anchor_cache:
                anchor_cache[resolved] = headings(resolved)
            known = anchor_cache[resolved]
            if anchor.lower() not in known:
                problems.append(
                    f'{at}: [{label}]({target}) -> `{resolved}` has no heading `#{anchor}`'
                )

    return problems


def main() -> int:
    # Windows consoles default to cp1252, which cannot encode a path or heading containing
    # anything outside Latin-1 — and this repository's documents are full of em dashes.
    # A checker that crashes while reporting a broken link is worse than no checker: the
    # traceback buries the finding it was about to print.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding='utf-8', errors='replace')
        except (AttributeError, ValueError):
            pass

    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('files', nargs='*', help='files to check; omit for every tracked .md')
    args = ap.parse_args()

    paths = args.files or tracked_markdown()
    if not paths:
        print('no markdown files found.')
        return 0

    anchor_cache: dict[str, set[str]] = {}
    problems: list[str] = []
    for path in paths:
        problems += check(path, anchor_cache)

    if not problems:
        print(f'{len(paths)} files: every internal link resolves.')
        return 0

    print(f'{len(problems)} broken link(s):\n')
    for p in problems:
        print(f'  {p}')
    print('\nA link that goes nowhere teaches a reader that the cross-references in these')
    print('documents cannot be trusted, which costs more than the one broken link.')
    return 1


if __name__ == '__main__':
    sys.exit(main())
