#!/usr/bin/env python3
"""Every gate this repository has, each judged by its own exit code.

    python3 scripts/verify.py            # everything
    python3 scripts/verify.py --fast     # skip the slow ones (release build, doc tests)

**Why this exists rather than a list of commands in a README.** Entry 16 of
`.claude/skills/coding-mistakes/SKILL.md` is a link checker that passed locally and failed in
CI, because it read the working tree and CI reads the commit. The one after it is a verification
line that printed `CLIPPY OK` whether or not clippy had failed:

    cargo clippy ... | tail -4 && echo "CLIPPY OK"

A pipeline takes its status from the *last* command, so `tail` succeeding hid a broken build. The
register calls that class **a check that cannot fail**, and it is the most common single cause of
a defect reaching a pull request here.

So this script does three things a hand-typed command line kept failing to do:

* takes each gate's **own** exit status, never a pipe's;
* runs the file-reading checks against a **clean checkout of HEAD**, not the working tree;
* exits non-zero if anything failed, so it cannot be skimmed past.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCHMARKS = ROOT / "docs" / "BENCHMARKS.md"


class Gate:
    def __init__(self, name: str, argv: list[str], *, cwd: Path | None = None, slow: bool = False):
        self.name = name
        self.argv = argv
        self.cwd = cwd or ROOT
        self.slow = slow

    def run(self) -> tuple[bool, str]:
        try:
            done = subprocess.run(
                self.argv,
                cwd=self.cwd,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                shell=(sys.platform == "win32" and self.argv[0] in {"npm", "npx"}),
            )
        except FileNotFoundError as missing:
            return False, f"not installed: {missing.filename}"
        if done.returncode == 0:
            return True, self._summary(done.stdout + done.stderr)
        tail = (done.stdout + done.stderr).strip().splitlines()[-12:]
        return False, "\n      ".join(tail)

    @staticmethod
    def _summary(output: str) -> str:
        """One line worth reading when a gate passes — usually a count."""
        for line in reversed(output.splitlines()):
            stripped = line.strip()
            if any(
                mark in stripped
                for mark in (
                    "tests run:",
                    "Tests ",
                    "every internal link",
                    "look runnable",
                    "anchor is in its file",
                )
            ):
                return stripped
        return ""


def gates(web: bool) -> list[Gate]:
    found = [
        # **Against the working tree, deliberately.** Every other file-reading check runs on a
        # clean checkout of HEAD, because a link that resolves only because of an untracked
        # file is the defect there. This one is the opposite: it exists to stop a deliberate
        # defect *reaching* a commit, so asking HEAD would only ever find it too late.
        Gate("no live mutations", [sys.executable, "scripts/no_live_mutations.py"]),
        # **The other way a catalog stops meaning anything.** That gate finds a mutation left
        # *in* the tree; this one finds a mutation that can no longer be put there, because the
        # line it pins was reformatted or moved. `scripts/mutate.py` prints `NOT APPLIED` and
        # carries on, so the run ends `26 of 27` and reads like a coverage gap. Review found one
        # that way; the check then found four more, rotting since earlier phases, in catalogs
        # nobody had reason to re-run. Text matching only, so it costs nothing - running the
        # catalogs themselves takes tens of minutes and is why they were never a gate.
        Gate("mutation anchors", [sys.executable, "scripts/mutation_anchors.py"]),
        # **A defect no compiler and no test can see.** A `\` continuation in a Rust string
        # literal strips the newline *and* the indentation after it; lose the backslash and the
        # indentation stays, baked into a sentence a reader is shown. It has happened four times
        # here - a CLI message, two test assertions, three model prompts - and every one was
        # found by eye, afterwards. Text matching, so it costs nothing.
        Gate("lost continuations", [sys.executable, "scripts/no_lost_continuations.py"]),
        # **A percentage nobody adds up is a percentage that drifts.** `Full_Feature_List.md`'s
        # S2 totals row read 18/16/2 while its own ten rows summed to 19/16/3, so the page
        # published 89% for a state that was 84% — and every pull request quoting it repeated
        # the number. Nothing was wrong on purpose: a row was added and the total below it was
        # not. Same shape as the benchmark-count gate, and free, because the source of the
        # number is the document's own rows.
        Gate("feature totals", [sys.executable, "scripts/feature_totals.py"]),
        # A table with no `|---|` renders as a wall of pipes, and only in a browser - the
        # source looks fine in an editor and in every diff. These documents are read on
        # GitHub, so that is the rendering that decides whether they are readable.
        Gate("markdown tables", [sys.executable, "scripts/markdown_tables.py"]),
        # american-spelling: off
        # **One dialect, and it is American.** Nobody had chosen one, so the repository grew
        # both - `analyse` beside `analysis`, `catalogue` beside `catalog`, `normalise` beside
        # `normalize` - and the first place it showed was the word on the busiest button in the
        # product. Mixed spelling is not cosmetic: a reader cannot tell a house style from a
        # typo, and `grep normalize` finds half the callers. Sweeping it fixed today; this is
        # what stops tomorrow, because the next `analyse` is typed by somebody who never knew a
        # decision was made.
        # american-spelling: on
        Gate("american spelling", [sys.executable, "scripts/american_spelling.py"]),
        # **Third time, so now there is a check.** A paragraph assembled by a Python heredoc can
        # leave the heredoc's own source in the file when a concatenation is written wrong, and
        # nothing else here can see it: it lands inside a comment, where `fmt`, `clippy` and the
        # markdown gates are all content. It reached a reviewer three separate times.
        Gate("generator artifacts", [sys.executable, "scripts/no_generator_artifacts.py"]),
        Gate("fmt", ["cargo", "fmt", "--all", "--check"]),
        # `--all-features` and `--all-targets`, because CI uses both and a lint that only fires
        # on a test target is exactly the one a hurried local run misses.
        Gate("clippy", ["cargo", "clippy", "--all-targets", "--all-features", "-q", "--", "-D", "warnings"]),
        Gate("tests", ["cargo", "nextest", "run", "--all-features"]),
        Gate("doctests", ["cargo", "test", "--all-features", "--doc", "-q"], slow=True),
    ]
    if web:
        found += [
            Gate("web types", ["npm", "run", "typecheck"], cwd=ROOT / "web"),
            Gate("web lint", ["npm", "run", "lint"], cwd=ROOT / "web"),
            Gate("web tests", ["npx", "vitest", "run"], cwd=ROOT / "web"),
        ]
    return found


def documentation_gates(tree: Path) -> list[Gate]:
    """The file-reading checks, pointed at whatever tree is given.

    Run against a clean checkout rather than the working directory: an untracked file makes a
    link resolve locally that will not resolve for anybody else, which is entry 16.
    """
    return [
        Gate("links", [sys.executable, "scripts/check_links.py"], cwd=tree),
        Gate(
            "instructions",
            [
                sys.executable,
                "scripts/lint_instructions.py",
                "README.md",
                "docs/Feature_Walkthrough.md",
                "docs/TUTORIAL.md",
            ],
            cwd=tree,
        ),
    ]


#: The gate whose summary carries each count, the word the table uses for it, and how to read
#: the number out of what that tool prints.
COUNTS = {
    "tests": ("Rust", r"(\d+) tests run"),
    "web tests": ("frontend", r"Tests\s+(\d+) passed"),
}

#: Vitest colors its summary, so `Tests  51 passed` arrives with escape sequences sitting
#: between the words. Found by this gate refusing to read its own input on the first full run,
#: which is the behavior wanted: it could not find a number and said so.
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def claimed_counts() -> dict[str, int | None] | None:
    """The test counts the newest benchmark run claims, keyed by the word the table uses.

    `docs/BENCHMARKS.md` is written newest-first, so the **first** `| now |` row is the run
    being added and every one below it is frozen history. Entry 17 of the register is the
    non-unique anchor — a replacement that landed on the first of six identical lines by
    accident — so taking the first match here is stated as the deliberate choice it is.
    """
    try:
        text = BENCHMARKS.read_text(encoding="utf-8")
    except OSError:
        return None
    for line in text.splitlines():
        if line.startswith("| now |"):
            numbers = [int(found) for found in re.findall(r"\*\*(\d+)\*\*", line)]
            if not numbers:
                return None
            return {
                "Rust": numbers[0],
                "frontend": numbers[1] if len(numbers) > 1 else None,
            }
    return None


def gate_summary(results: list[tuple[str, bool, str]], name: str) -> str | None:
    """The passing summary line of a gate that has already run, if it passed."""
    for ran, ok, detail in results:
        if ran == name:
            return detail if ok else None
    return None


def compared(claimed: dict[str, int | None], summaries: dict[str, str]) -> str | None:
    """`None` when every count the table claims is the one that ran, else what disagrees.

    Pure, and separated from the reading above so that [`self_check`] can hand it fixed inputs.
    """
    for gate, (what, pattern) in COUNTS.items():
        if gate not in summaries:
            continue
        says = claimed.get(what)
        found = re.search(pattern, ANSI.sub("", summaries[gate]))
        if found is None:
            return f"no count in the {gate} summary: {summaries[gate]!r}"
        ran = int(found.group(1))
        if says is None:
            return f"the benchmark row claims no {what} count, but {ran} tests ran"
        if says != ran:
            return f"the table says {says} {what} tests, the suite ran {ran}"
    return None


def self_check() -> str | None:
    """That [`compared`] can still both agree and disagree.

    Entry 28 of the register is the mutation harness reporting a clean pass while nothing had
    run at all. A comparison broken open — `!=` become `==`, a regex that never matches — would
    do the same thing here forever, silently, and this is the last gate anybody reads. So it is
    given counts that agree and counts that do not, on every run, and must tell them apart.
    """
    # The frontend line is the colored one vitest actually prints, escape sequences and all,
    # because the plain version is not what this gate is ever handed.
    summaries = {
        "tests": "2 tests run: 2 passed",
        "web tests": "\x1b[2m      Tests \x1b[22m \x1b[1m\x1b[32m3 passed\x1b[39m\x1b[22m",
    }
    if compared({"Rust": 2, "frontend": 3}, summaries) is not None:
        return "the comparison rejects counts that agree"
    if compared({"Rust": 9, "frontend": 3}, summaries) is None:
        return "the comparison accepts a Rust count that disagrees"
    if compared({"Rust": 2, "frontend": 9}, summaries) is None:
        return "the comparison accepts a frontend count that disagrees"
    if compared({"Rust": 2, "frontend": None}, summaries) is None:
        return "the comparison accepts a row with the frontend count missing"
    return None


def benchmark_counts(results: list[tuple[str, bool, str]], web: bool) -> tuple[str, bool, str]:
    """What `docs/BENCHMARKS.md` claims for this run, against what the gates just counted.

    A count written into a document is a second source of truth for something the harness
    already knows, and review found this one wrong by three: the table said 583 where the
    suite ran 586. Every fix in this project that stuck removed the second source rather than
    adding a comparison — but a benchmark table is a record of history and cannot be removed,
    so it is read back instead, from the run that produced it.

    **The failure modes are failures.** An unreadable row, an unparseable summary, or a tests
    gate that did not pass all report FAIL rather than quietly agreeing, because a check that
    cannot fail is the most common defect route here.

    **And what it does not cover.** This runs here and not in CI, where the two counts belong to
    two different jobs and reaching them would mean wrapping a pipeline around a test command —
    which is entry 17, the `| tail && echo OK` that printed success over a broken build. So the
    number is checked by the command a change is supposed to pass before it becomes a pull
    request, and a change that skips that command can still carry a wrong one.
    """
    # Named for what it did **not** check when the frontend was skipped, rather than passing
    # quietly on half the row: entry 8 is an output that could not say what it had left out.
    name = "benchmark counts" if web else "benchmark counts (Rust only)"

    broken = self_check()
    if broken is not None:
        return name, False, broken

    claimed = claimed_counts()
    if claimed is None:
        return name, False, f"no `| now |` row with a count in docs/{BENCHMARKS.name}"

    summaries: dict[str, str] = {}
    for gate in COUNTS:
        if gate == "web tests" and not web:
            continue
        summary = gate_summary(results, gate)
        if summary is None:
            return name, False, f"the {gate} gate did not pass, so there is no count to compare"
        summaries[gate] = summary

    disagrees = compared(claimed, summaries)
    if disagrees is not None:
        return name, False, disagrees
    return name, True, "the `now` row matches this run"


def clean_checkout() -> tuple[Path, object] | None:
    """A worktree at HEAD, so the documentation checks see what is committed."""
    where = Path(tempfile.mkdtemp(prefix="landscape-verify-"))
    tree = where / "head"
    done = subprocess.run(
        ["git", "worktree", "add", "-q", "--detach", str(tree), "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if done.returncode != 0:
        shutil.rmtree(where, ignore_errors=True)
        return None
    return tree, where


def say(line: str) -> None:
    """Print a line that may hold anything a tool wrote.

    Windows consoles default to cp1252, and eslint's `✖` crashed this reporter *while it
    was reporting a failure* — the one moment it has a job to do. A gate's output is arbitrary
    bytes from somebody else's program, so it is written defensively rather than trusted.
    """
    try:
        print(line, flush=True)
    except UnicodeEncodeError:
        encoding = sys.stdout.encoding or "ascii"
        print(line.encode(encoding, "replace").decode(encoding), flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fast", action="store_true", help="skip the slow gates")
    parser.add_argument("--no-web", action="store_true", help="skip the frontend gates")
    args = parser.parse_args()

    results: list[tuple[str, bool, str]] = []
    for gate in gates(web=not args.no_web):
        if args.fast and gate.slow:
            continue
        print(f"... {gate.name}", flush=True)
        ok, detail = gate.run()
        results.append((gate.name, ok, detail))

    checkout = clean_checkout()
    if checkout is None:
        print("... documentation gates against the working tree (no clean checkout available)")
        for gate in documentation_gates(ROOT):
            ok, detail = gate.run()
            results.append((f"{gate.name} (working tree)", ok, detail))
    else:
        tree, holder = checkout
        try:
            for gate in documentation_gates(tree):
                print(f"... {gate.name} (clean checkout of HEAD)", flush=True)
                ok, detail = gate.run()
                results.append((gate.name, ok, detail))
        finally:
            subprocess.run(
                ["git", "worktree", "remove", "--force", str(tree)],
                cwd=ROOT,
                capture_output=True,
            )
            shutil.rmtree(holder, ignore_errors=True)

    # Last, because it reads what the gates above just counted. Against the working tree and
    # not the clean checkout: the number being checked is the one about to be committed.
    results.append(benchmark_counts(results, web=not args.no_web))

    print()
    width = max(len(name) for name, _, _ in results)
    for name, ok, detail in results:
        mark = "ok  " if ok else "FAIL"
        say(f"  {mark}  {name:<{width}}  {detail.splitlines()[0] if detail else ''}")
        if not ok and detail:
            say(f"      {detail}")

    failed = [name for name, ok, _ in results if not ok]
    if failed:
        print(f"\n{len(failed)} gate(s) failed: {', '.join(failed)}")
        return 1
    print(f"\nall {len(results)} gates pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
