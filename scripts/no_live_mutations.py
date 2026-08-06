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
    for catalogue in sorted(glob.glob(str(ROOT / "docs" / "mutations" / "*.json"))):
        try:
            entries = json.loads(io.open(catalogue, encoding="utf-8").read())
        except json.JSONDecodeError as broken:
            print(f"{os.path.basename(catalogue)}: not readable as JSON - {broken}")
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
                found.append((entry["file"], entry["name"], os.path.basename(catalogue)))
    return found


def main() -> int:
    found = live_mutations()
    if not found:
        print("no recorded mutation is live in the tree.")
        return 0

    print("A deliberate defect is in the source:\n")
    for path, name, catalogue in found:
        print(f"  {path}\n    {name}   [{catalogue}]")
    print(
        "\nThis is what an interrupted `scripts/mutate.py` leaves behind. Put the original\n"
        "back before committing - `git diff` will show it."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
