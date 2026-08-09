# A run of spaces in the middle of a sentence is a `\` continuation that did not survive.
#
# Rust lets a string literal span lines with a trailing backslash, and strips the newline *and*
# the following indentation. When the backslash is lost — by an editor, a patch tool, or a
# generator whose own escaping ate it — what is left is the indentation, baked into the string:
#
#     "we could not complete 2 of the 3 searches. That is usually                  temporary"
#
# Nothing fails. It compiles, the tests pass, and the sentence reaches a reader with a hole in
# it. This has happened four times in this repository, in a CLI message, a test assertion and
# three model prompts, and every one of them was found by eye afterwards.
#
# The check has to tell that apart from **column padding**, which this codebase uses constantly
# and deliberately: `"none    no price found"`, `"{:<58} answers      may set a table value"`.
# Two signals do it, and both are needed:
#
#   1. **Nine spaces or more.** A continuation keeps the whole indentation of the line that
#      followed it, and a string literal inside a function inside a block starts at column 12 or
#      further in. Every real one found here carried 10 or 14. Every deliberate pad was 4 to 8 —
#      a column that wide would be unreadable, which is why nobody writes one.
#   2. **A letter on both sides.** A pad is nearly always followed by a format specifier or a
#      bracket; prose is followed by another word.
#
# Both together, so widening a table by two spaces does not start failing builds and a genuine
# hole in a sentence does not slip through on a narrow indent.
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

MIN_GAP = 9
HOLE = re.compile(r'[A-Za-z,.;:)] {%d,}[A-Za-z]' % MIN_GAP)

# Strings whose spacing is deliberate, each with the reason it cannot simply be fixed here.
#
# The three prompts are `landscape-analyze::stages`. Editing a prompt changes what a model is
# asked, so it needs `PROMPT_VERSION` bumped and the golden set re-run against a real model —
# which is a benchmark, not a lint fix. Recorded rather than quietly skipped.
ALLOWED = {
    'crates/landscape-analyze/src/stages.rs',
}


def literals(line):
    """The double-quoted spans of one line, minus the quotes. Crude on purpose."""
    return re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', line)


def main():
    bad = []
    deferred = 0
    for base, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = [d for d in dirs if d not in ('target', 'fixtures')]
        for name in files:
            if not name.endswith('.rs'):
                continue
            path = os.path.join(base, name)
            rel = os.path.relpath(path, ROOT).replace('\\', '/')
            for n, line in enumerate(io.open(path, encoding='utf-8'), 1):
                if not any(HOLE.search(lit) for lit in literals(line)):
                    continue
                if rel in ALLOWED:
                    deferred += 1
                    continue
                bad.append((rel, n, line.strip()[:110]))

    if bad:
        print('A sentence has a run of spaces in it, where a `\\` continuation used to be:\n')
        for rel, n, text in bad:
            print(f'  {rel}:{n}')
            print(f'    {text}')
        print()
        print('Rust strips the newline and the indentation after a trailing backslash. Without')
        print('the backslash the indentation stays, and the sentence reaches a reader with a')
        print('hole in it. Split the literal into several `println!`s, or use `concat!` — both')
        print('survive a reformat, which a continuation in this repository has repeatedly not.')
        return 1

    extra = f' ({deferred} deferred, see ALLOWED)' if deferred else ''
    print(f'no sentence has a lost continuation in it{extra}.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
