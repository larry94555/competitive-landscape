#!/usr/bin/env python3
"""Check that every recorded mutation can still be applied.

    python3 scripts/mutation_anchors.py

**A mutation whose anchor has rotted proves nothing and says so quietly.** `scripts/mutate.py`
prints `NOT APPLIED` for it and carries on; the run ends `26 of 27`, which reads like a coverage
gap rather than what it is — a pin that came loose when somebody reformatted the line under it.
Review found one: `cargo fmt` collapsed a `subject::resolve(...)` call onto one line two commits
after the anchor was written, and the catalogue it lived in was not the one being worked on, so
nothing re-ran it.

**This is text matching and nothing else, which is the point.** Running the catalogues takes
tens of minutes — too slow for a gate, so `scripts/verify.py` never ran them and a stale anchor
could ship. Reading each `old` out of the JSON and looking for it in the file takes a moment, and
it catches the whole class before the harness is ever started.

Two failures, and they are different:

* **missing** — the code moved and the pin did not. Retarget it, or delete it if a newer
  mutation supersedes it.
* **ambiguous** — the anchor appears more than once, so the harness would mutate the first
  occurrence, which may not be the one the name is about. Make it unique.

It deliberately does **not** check that `new` is absent; that is
`scripts/no_live_mutations.py`'s job, and the two are separate because they fail for opposite
reasons.

Exits non-zero with the catalogue, the name and the file, so a gate or CI can stop the commit.
"""

from __future__ import annotations

import glob
import io
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def rotted() -> list[tuple[str, str, str, str]]:
    """Every recorded mutation whose `old` is not in its file exactly once.

    Returns `(catalogue, name, file, what)` where `what` is `missing` or `ambiguous`.
    """
    found: list[tuple[str, str, str, str]] = []
    for path in sorted(glob.glob(str(ROOT / "docs" / "mutations" / "*.json"))):
        catalogue = os.path.basename(path)
        try:
            entries = json.loads(io.open(path, encoding="utf-8").read())
        except json.JSONDecodeError as broken:
            found.append((catalogue, "(whole file)", "-", f"not readable as JSON - {broken}"))
            continue
        for entry in entries:
            target = ROOT / entry["file"]
            if not target.exists():
                found.append((catalogue, entry["name"], entry["file"], "no such file"))
                continue
            source = io.open(target, encoding="utf-8").read()
            occurrences = source.count(entry["old"])
            if occurrences == 0:
                found.append((catalogue, entry["name"], entry["file"], "missing"))
            elif occurrences > 1:
                found.append(
                    (catalogue, entry["name"], entry["file"], f"ambiguous ({occurrences})")
                )
    return found


def main() -> int:
    # **A live mutation makes this lie, and it lies in the direction of alarm.** Run while the
    # harness has a file swapped out and the anchors it replaced read as `missing` - three did,
    # the first time this script was run, and they were the mutation in flight rather than rot.
    # Same hazard as the harness's own baseline; same guard.
    sys.path.insert(0, str(ROOT / "scripts"))
    import no_live_mutations  # noqa: PLC0415 — local, and only needed on this path

    if live := no_live_mutations.live_mutations():
        print("A mutation is live in the tree, so anchors cannot be checked against it:\n")
        for path, name, catalogue in live:
            print(f"  {path}\n    {name}   [{catalogue}]")
        print("\nEither a run is in flight - wait for it - or one was interrupted; restore it.")
        return 2

    loose = rotted()
    if not loose:
        total = sum(
            len(json.loads(io.open(p, encoding="utf-8").read()))
            for p in glob.glob(str(ROOT / "docs" / "mutations" / "*.json"))
        )
        print(f"{total} mutations: every anchor is in its file exactly once.")
        return 0

    print("A recorded mutation can no longer be applied:\n")
    for catalogue, name, path, what in loose:
        print(f"  {path}\n    {name}   [{catalogue}]   {what}")
    print(
        "\nThe harness would report NOT APPLIED for these, which looks like a coverage gap and\n"
        "is a loose pin. Retarget each one to the code as it is now, or delete it if a newer\n"
        "mutation supersedes it - then re-run that catalogue."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
