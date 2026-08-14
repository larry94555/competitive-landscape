#!/usr/bin/env python3
"""Nothing in this repository may contain a fragment of the script that wrote it.

**Three times, and only the third one had a check.** Text in this repository is often written by
a program - a Python heredoc assembling a paragraph out of string pieces so that an em dash or a
section sign survives a shell round trip. When one of those pieces is not concatenated the way
its author thought, the *source* of the generator lands in the file:

    /// and one feature page \"\"\" + D + \"\"\" `freshbooks.com/` and ...

It reads as garbage, it survives every gate this repository has - it is inside a comment, so
`fmt`, `clippy` and the markdown checks are all happy with it - and it reached a reviewer three
separate times before this file existed. That is the definition of a check worth writing: a
failure mode that recurs, that no existing gate can see, and that costs a person's attention
every time.

**What it looks for is a Python string operator surrounded by quotes**, which is a shape that
occurs in generator source and essentially never in prose. Code that legitimately discusses
Python is the exception, and it is handled the way `american_spelling.py` handles quoting a
British spelling: mark the passage.

    # generator-artifacts: off
    ... a passage that really does contain one ...
    # generator-artifacts: on
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Where text lives. `target/` and `node_modules/` are other people's files.
LOOK_IN = ("crates", "docs", "web/src", "scripts", ".claude", "*.md")
SKIP = {"target", "node_modules", ".git", "dist", "coverage"}

TEXT = {".rs", ".md", ".ts", ".tsx", ".py", ".json", ".toml", ".yml", ".yaml", ".html", ".css"}

# `""" + NAME + """` and `" + NAME + "`, the two shapes a broken concatenation leaves behind.
ARTIFACT = re.compile(r'"{1,3}\s*\+\s*[A-Za-z_][A-Za-z_0-9]*\s*\+\s*"{1,3}')

OFF = "generator-artifacts: off"
ON = "generator-artifacts: on"


def files() -> list[Path]:
    seen: list[Path] = []
    for pattern in LOOK_IN:
        for path in ROOT.glob(pattern if "*" in pattern else f"{pattern}/**/*"):
            if not path.is_file() or path.suffix not in TEXT:
                continue
            if any(part in SKIP for part in path.relative_to(ROOT).parts):
                continue
            seen.append(path)
    return sorted(set(seen))


def main() -> int:
    # This file necessarily contains the shape it looks for, in the pattern and in the docstring.
    here = Path(__file__).resolve()
    found: list[str] = []
    checked = 0
    for path in files():
        if path == here:
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        checked += 1
        marked = False
        for number, line in enumerate(lines, start=1):
            if OFF in line:
                marked = True
                continue
            if ON in line:
                marked = False
                continue
            if marked:
                continue
            if ARTIFACT.search(line):
                where = path.relative_to(ROOT).as_posix()
                found.append(f"  {where}:{number}\n      {line.strip()[:100]}")

    if found:
        print("A fragment of the script that wrote this text is still in it:\n")
        print("\n".join(found))
        print(
            "\nThat is a string concatenation that did not happen. Fix the text, and if a "
            "passage\nreally does need to show one, wrap it in `generator-artifacts: off` and "
            "`... : on`."
        )
        return 1

    print(f"{checked} text file(s): no generator artifacts.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
