#!/usr/bin/env python3
"""Put a defect back into the code and see whether a test notices.

    python3 scripts/mutate.py mutations.json

**This is the only mechanical thing that has ever found a defect in this repository.** Reading
real output found some, using the product found some, review found most — and the test suite,
left to itself, found none. Breaking the code on purpose is the one check that asks the question
the suite is supposed to answer: *if this were wrong, would anything say so?*

It has been written by hand for most of the last dozen pull requests and thrown away each time,
which is how two of its own traps got hit twice. Both are guarded here.

## The file

A JSON list. Each entry names a defect in the words a person would use, the file, the exact text
to replace, and what to replace it with:

    [
      {
        "name": "the features loop ignores the answer",
        "file": "crates/landscape-analyze/src/stages.rs",
        "old": "if so_far(...) == crate::Wanted::No {\\n    break;\\n}",
        "new": "so_far(...);",
        "run": ["cargo", "nextest", "run", "-p", "landscape-analyze"]
      }
    ]

`run` is optional and defaults to the whole Rust suite. For frontend mutations use
`["npx", "vitest", "run"]` and set `"cwd": "web"`.

Keep the file in your scratchpad; it belongs to one change, not to the repository.

## The two traps this guards against

**An anchor that matches more than once.** `str.replace(old, new, 1)` silently edits the *first*
occurrence. When three stages had been given identical `break` blocks, a mutation aimed at one of
them broke another, reported `MISSED`, and very nearly had a covered case written up as a gap —
entry 17. **A non-unique anchor is refused here rather than applied.**

**A restored file that cargo thinks is unchanged.** `shutil.move` preserves mtime, so putting the
original back can leave the build cache holding the mutated artefact and the *next* run measuring
the wrong code. Every file is touched after restoring.

## MISSED is an exit code, not a line of output

A missed mutation means a test you believe in cannot fail. That is a finding, so it leaves a
non-zero status rather than a word in a table somebody skims.
"""

from __future__ import annotations

import argparse
import io
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_RUN = ["cargo", "nextest", "run", "--all-features"]


def failing_tests(output: str) -> list[str]:
    """Test names the runner reported as failures — nextest and vitest both."""
    names: list[str] = []
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith("FAIL "):  # nextest
            names.append(stripped.split("] ")[-1].strip())
        elif " FAIL " in line and "src/" in line:  # vitest
            names.append(line.split("> ")[-1].strip())
    seen: list[str] = []
    for name in names:
        if name not in seen:
            seen.append(name)
    return seen


def ran_any_tests(output: str) -> bool:
    """Whether the runner got far enough to execute anything.

    Both runners announce what they ran when they finish, pass or fail. Nothing else in the
    output tells "every test passed" apart from "no test ever started".
    """
    return any(
        mark in output
        for mark in ("tests run:", "test result:", "Tests ", "Test Files")
    )


def last_meaningful_line(output: str) -> str:
    """One line worth reading from a run that produced no summary."""
    for line in reversed(output.splitlines()):
        stripped = line.strip()
        if stripped and not stripped.startswith(("=", "|", "note:")):
            return stripped[:160]
    return "no output at all"


def apply(mutation: dict) -> int:
    path = ROOT / mutation["file"]
    name = mutation["name"]
    old, new = mutation["old"], mutation["new"]

    source = io.open(path, encoding="utf-8").read()
    occurrences = source.count(old)
    if occurrences == 0:
        print(f"  NOT APPLIED  {name}\n      the anchor is not in {mutation['file']}")
        return 1
    if occurrences > 1:
        # Entry 17. Refusing is the whole point: a mutation applied somewhere other than where
        # it was aimed reports MISSED for a case that is covered, and that gets written down.
        print(
            f"  NOT APPLIED  {name}\n"
            f"      the anchor appears {occurrences} times in {mutation['file']};"
            " make it unique or the result is about the wrong code"
        )
        return 1

    backup = path.with_suffix(path.suffix + ".mutate-backup")
    shutil.copy(path, backup)
    io.open(path, "w", encoding="utf-8", newline="\n").write(source.replace(old, new, 1))
    try:
        argv = mutation.get("run", DEFAULT_RUN)
        cwd = ROOT / mutation["cwd"] if "cwd" in mutation else ROOT
        done = subprocess.run(
            argv,
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            shell=(sys.platform == "win32" and argv[0] in {"npm", "npx"}),
        )
        output = (done.stdout or "") + (done.stderr or "")
    finally:
        shutil.move(str(backup), path)
        # Restoring preserves mtime, which leaves cargo holding the mutated artefact. Touch it,
        # or the next run measures code nobody is looking at.
        os.utime(path, None)

    if "error[" in output or "error TS" in output:
        print(f"  BROKEN       {name}\n      the mutation does not compile, so it proves nothing")
        return 1

    if not ran_any_tests(output):
        # **A run that never got as far as running is not a MISSED.** This printed "nothing
        # failed" for fourteen mutations in a row once, because a hung test process was holding
        # the binary and every link failed - and the check above only catches rustc's own
        # diagnostics, not a linker error, a lock on an executable, or a runner that died.
        #
        # The tool built to find checks that cannot fail had one in it. The rule is positive
        # evidence: a verdict is only about the code if the suite actually ran, and the
        # summary line is the proof that it did.
        print(
            f"  BROKEN       {name}\n"
            "      the suite did not run, so this says nothing about the code.\n"
            f"      {last_meaningful_line(output)}"
        )
        return 1

    caught = failing_tests(output)
    if caught:
        print(f"  caught       {name}\n      by {', '.join(caught[:3])}")
        return 0
    print(
        f"  MISSED       {name}\n"
        "      nothing failed. Either a test is needed, or this mutation did not do what you\n"
        "      think - check the second before believing the first."
    )
    return 1


def already_mutated() -> int:
    """Refuse to measure anything while a recorded defect is already in the tree.

    **Every mutation below backs up the file it is about to edit and restores that backup
    afterwards — so a tree that starts out mutated makes the defect the baseline.** Thirty-eight
    entries then report `caught` against code nobody wrote, the counts add up, and the run looks
    exactly like a clean one.

    That is not hypothetical twice over. `scripts/no_live_mutations.py` opens by saying *"this
    exists because it happened"* — a cut-short run once left an inverted rule in the source. It
    happened again, from a run killed by a timeout shorter than the catalogue takes, and the only
    signal was one entry reporting `NOT APPLIED` because the kill had eaten its anchor.

    That guard is `scripts/verify.py`'s first gate, which runs long after every result here has
    been read and believed. The check is the same one; the point is *when*.
    """
    sys.path.insert(0, str(ROOT / "scripts"))
    import no_live_mutations  # noqa: PLC0415 — local, and only needed on this path

    live = no_live_mutations.live_mutations()
    if not live:
        return 0
    print("Refusing to run: a recorded mutation is already live in the working tree.\n")
    for path, name, catalogue in live:
        print(f"  {path}\n    {name}   [{catalogue}]")
    print(
        "\nA previous run was interrupted before it could put the original back. Restore it -\n"
        "`git diff` shows the hunk - because every mutation below would otherwise be measured\n"
        "against that defect, and would report `caught` while proving nothing."
    )
    return 2


def main() -> int:
    parser = argparse.ArgumentParser(description="Reintroduce defects and report what notices.")
    parser.add_argument("file", help="JSON list of mutations")
    args = parser.parse_args()

    if refused := already_mutated():
        return refused

    mutations = json.loads(io.open(args.file, encoding="utf-8").read())
    print(f"{len(mutations)} mutation(s)\n")
    missed = sum(apply(m) for m in mutations)

    print()
    if missed:
        print(f"{missed} of {len(mutations)} not caught - each one is a test that cannot fail")
        return 1
    print(f"all {len(mutations)} caught")
    return 0


if __name__ == "__main__":
    sys.exit(main())
