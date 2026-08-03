#!/usr/bin/env python3
"""Lint shell commands in prose without executing them.

A pull request description is untrusted input — it can be written by anyone who opens a
PR — so this reads and checks, and never runs, what it finds. The README gets the stronger
treatment (actually executed) in `crates/landscape/tests/docs.rs`, because the README is
in the repository and goes through review.

    python3 scripts/lint_instructions.py README.md docs/*.md
    python3 scripts/lint_instructions.py --stdin-name "the PR description" < body.md

Exit code 1 if anything is wrong, with the line quoted.
"""

from __future__ import annotations

import argparse
import io
import re
import sys

# The port the binary listens on, read from the source rather than repeated here. Two
# copies of a constant is what caused the bug this script exists to prevent.
MAIN_RS = 'crates/landscape/src/main.rs'


def api_port(default: str = '8787') -> str:
    try:
        src = io.open(MAIN_RS, encoding='utf-8').read()
    except OSError:
        return default
    m = re.search(r'const DEFAULT_ADDR:\s*&str\s*=\s*"([^"]+)"', src)
    return m.group(1).rsplit(':', 1)[-1] if m else default


# A walkthrough teaches partly by showing what a failure looks like, so some of its commands
# are meant to be wrong. Marking them individually beats exempting a whole file: the marker
# says "this one is deliberate", and every unmarked command in the file is still checked.
MARKER = '# expect-failure'


def commands(text: str) -> list[tuple[int, str]]:
    """Shell commands inside ```bash fences, with `\\` continuations joined.

    A command preceded by `# expect-failure` is skipped. Without that, the only way to
    document an error would be to stop checking the file that documents it."""
    out: list[tuple[int, str]] = []
    inside = False
    pending = ''
    start = 0
    skip_next = False
    for n, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if line.startswith('```'):
            inside = line.startswith('```bash') or line.startswith('```sh')
            continue
        if inside and line.startswith(MARKER):
            skip_next = True
            continue
        if not inside or not line or line.startswith('#'):
            continue
        if skip_next:
            # Consume the whole command, continuations included.
            if not line.endswith('\\'):
                skip_next = False
            continue
        if not pending:
            start = n
        if line.endswith('\\'):
            pending += line[:-1].rstrip() + ' '
            continue
        out.append((start, pending + line))
        pending = ''
    if pending:
        out.append((start, pending.strip()))
    return out


def workspace_binaries() -> int:
    """How many binaries `cargo run` would have to choose between."""
    import glob
    import os
    n = 0
    for manifest in glob.glob('crates/*/Cargo.toml'):
        body = io.open(manifest, encoding='utf-8').read()
        if '[[bin]]' in body or os.path.exists(os.path.join(os.path.dirname(manifest), 'src', 'main.rs')):
            n += 1
    return n


def check(text: str, where: str) -> list[str]:
    port = api_port()
    binaries = workspace_binaries()
    problems: list[str] = []

    for line_no, cmd in commands(text):
        at = f'{where}:{line_no}'

        # A POST with a body and no content-type is rejected. Documenting one documents
        # a failure — this is exactly what shipped.
        if 'curl' in cmd and ' -d ' in cmd and 'content-type' not in cmd.lower():
            problems.append(
                f'{at}: sends a body with no content-type header, which the API rejects\n'
                f'    {cmd}\n'
                f"    fix: add -H 'content-type: application/json'"
            )

        # A command addressing our API on someone else's port reaches someone else's
        # program. Port 8080 in particular is llama.cpp's default.
        if '/api/' in cmd:
            found = re.findall(r'(?:localhost|127\.0\.0\.1|\[::1\]):(\d+)', cmd)
            for got in found:
                if got != port:
                    hint = ' (8080 is llama.cpp\'s default)' if got == '8080' else ''
                    problems.append(
                        f'{at}: talks to /api/ on port {got}, but the API listens on '
                        f'{port}{hint}\n    {cmd}'
                    )

        # A bare `cargo run` cannot choose between several binaries. Adding a second one
        # to the workspace silently broke every documented command; this is the check.
        if (
            cmd.startswith('cargo run')
            and binaries > 1
            and ' -p ' not in cmd
            and ' --bin ' not in cmd
        ):
            problems.append(
                f'{at}: this workspace builds {binaries} binaries, so a bare '
                f'`cargo run` cannot pick one\n'
                f'    {cmd}\n'
                f'    fix: `cargo run -p landscape -- ...`'
            )

        # A subcommand that no longer exists. Only the application has subcommands —
        # other binaries in the workspace take flags, and checking those against the
        # application's roles reports `--runs` as an unknown command.
        m = re.match(r'cargo run\s+(?:-p\s+(?P<pkg>\S+)\s+)?--\s+(?P<role>[a-z][a-z-]*)', cmd)
        if m and m.group('pkg') in (None, 'landscape'):
            role = m.group('role')
            try:
                src = io.open(MAIN_RS, encoding='utf-8').read()
            except OSError:
                src = ''
            if src and f'Some("{role}")' not in src:
                problems.append(f'{at}: documents `cargo run -- {role}`, which is not a command\n    {cmd}')

    return problems


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('files', nargs='*', help='files to check; omit to read stdin')
    ap.add_argument('--stdin-name', default='stdin', help='what to call stdin in messages')
    args = ap.parse_args()

    problems: list[str] = []
    if args.files:
        for path in args.files:
            problems += check(io.open(path, encoding='utf-8').read(), path)
    else:
        problems += check(sys.stdin.read(), args.stdin_name)

    if not problems:
        print('instructions look runnable.')
        return 0

    print(f'{len(problems)} problem(s) in the instructions:\n')
    for p in problems:
        print(f'  {p}\n')
    print('These are commands a reader will paste. A wrong one wastes their time and')
    print('looks like the software is broken.')
    return 1


if __name__ == '__main__':
    sys.exit(main())
