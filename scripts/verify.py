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
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


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
                for mark in ("tests run:", "Tests ", "every internal link", "look runnable")
            ):
                return stripped
        return ""


def gates(web: bool) -> list[Gate]:
    found = [
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

    print()
    width = max(len(name) for name, _, _ in results)
    for name, ok, detail in results:
        mark = "ok  " if ok else "FAIL"
        print(f"  {mark}  {name:<{width}}  {detail.splitlines()[0] if detail else ''}")
        if not ok and detail:
            print(f"      {detail}")

    failed = [name for name, ok, _ in results if not ok]
    if failed:
        print(f"\n{len(failed)} gate(s) failed: {', '.join(failed)}")
        return 1
    print(f"\nall {len(results)} gates pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
