#!/usr/bin/env python3
"""Refuse to let a mutation be committed as if it were the code.

    python3 scripts/no_live_mutations.py

**This exists because it happened.** `scripts/mutate.py` edits a source file, runs the suite,
and puts the original back — and if it is interrupted between those steps, or if something else
takes the file while it is mutated, the deliberate defect is simply sitting in the working tree.
A `git add -A` then commits it.

That is exactly what occurred: a hung test process held the compiled binary, every link failed,
the harness reported nothing useful, and a run was cut short leaving
`Ok(analysis) if analysis.status != AnalysisStatus::Failed => still.push(id)` in the handler —
the *inverse* of the rule that a failed analysis costs nothing, in the pull request that added
that rule. The tests were not run against it, because they could not be run at all.

So the mutation files are read back the other way round: every `new` payload is a defect this
repository has written down and knows the shape of, and finding one in the source is a finding.
Nothing here is heuristic — it compares against the exact strings the harness would have
written.

Exits non-zero with the file and the name, so a hook or CI can stop the commit.
"""

from __future__ import annotations

import glob
import io
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def live_mutations() -> list[tuple[str, str, str]]:
    """Every recorded mutation whose replacement is in the tree and whose original is not."""
    found: list[tuple[str, str, str]] = []
    for catalog in sorted(glob.glob(str(ROOT / "docs" / "mutations" / "*.json"))):
        try:
            entries = json.loads(io.open(catalog, encoding="utf-8").read())
        except json.JSONDecodeError as broken:
            print(f"{os.path.basename(catalog)}: not readable as JSON - {broken}")
            continue
        for entry in entries:
            target = ROOT / entry["file"]
            if not target.exists():
                continue
            source = io.open(target, encoding="utf-8").read()
            replacement, original = entry["new"], entry["old"]
            # Both halves matter. A replacement that happens to look like ordinary code is not
            # a finding while the original is still there - only the swap is.
            if replacement and replacement in source and original not in source:
                found.append((entry["file"], entry["name"], os.path.basename(catalog)))
    return found


def abandoned_backups() -> list[str]:
    """Every `*.mutate-backup` still on disk.

    **An exact signal, where the comparison above has a hole.** `live_mutations` matches the
    recorded `new` character for character, and a formatter run over an applied mutation defeats
    that: a `cargo fmt --all` reflowed a mutated `const TEMPLATES` onto one line — dropping the
    trailing comma as well as the newlines — so the swap was live and unrecognizable, and this
    gate reported a clean tree.

    A backup file is left behind by exactly two things, and both mean the tree is not to be
    trusted: a run that died holding a file, and `mutate.py` refusing to restore over somebody
    else's edit. Neither depends on what the mutated text looks like afterwards, which is why
    this check is sound where a text comparison is only usually right.
    """
    return [
        os.path.relpath(path, ROOT)
        for path in glob.glob(str(ROOT / "**" / "*.mutate-backup"), recursive=True)
    ]


def main() -> int:
    if left := abandoned_backups():
        print("A mutation run left its backup behind, so the tree is not what it looks like:\n")
        for path in left:
            print(f"  {path}")
        print(
            "\nThat file is the original. Compare it against the source beside it and put the\n"
            "right one back — `mutate.py` kept it rather than clobber an edit, or died holding\n"
            "it. Until it is gone nothing here can say whether a defect is live: a formatter\n"
            "run over an applied mutation makes the swap unrecognizable to the check below."
        )
        return 1

    found = live_mutations()
    if not found:
        print("no recorded mutation is live in the tree.")
        return 0

    print("A deliberate defect is in the source:\n")
    for path, name, catalog in found:
        print(f"  {path}\n    {name}   [{catalog}]")
    print(
        "\nThis is what an interrupted `scripts/mutate.py` leaves behind. Put the original\n"
        "back before committing - `git diff` will show it."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
