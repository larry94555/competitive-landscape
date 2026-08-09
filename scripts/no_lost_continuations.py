# A run of spaces in the middle of a sentence is a `\` continuation that did not survive.
#
# Rust lets a string literal span lines with a trailing backslash, and strips the newline *and*
# the following indentation. When the backslash is lost — by an editor, a patch tool, or a
# generator whose own escaping ate it — what is left is the indentation, baked into the string:
#
#     "we could not complete 2 of the 3 searches. That is usually                  temporary"
#
# Nothing fails. It compiles, the tests pass, and the sentence reaches a reader with a hole in
# it. This has happened repeatedly here, and every instance was found by eye afterwards.
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
#
# **Literals are read whole, across lines.** The first version of this matched `"…"` per line,
# which cannot see a literal that spans several — and a `\` continuation is *by definition* a
# literal that spans several. Review found the consequence: the three model prompts in
# `landscape-analyze::stages`, the longest multi-line literals in the repository and the place
# damage matters most, were invisible to the check written to find exactly this.
import hashlib
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

MIN_GAP = 9
HOLE = re.compile(r'[A-Za-z,.;:)] {%d,}[A-Za-z]' % MIN_GAP)

# Known damage, deferred by the **exact content** of the literal rather than by its file.
#
# Keyed on a digest, so this cannot rot into a blanket exemption: a new damaged string in the
# same file fails, and editing one of these fails too — which is correct, because editing one of
# these is the thing that needs a decision.
#
# These three are model prompts. Changing a prompt changes what a model is asked, so it needs
# `PROMPT_VERSION` bumped and the golden set re-run against a real model
# (`cargo test -p landscape-golden --test against_a_model -- --ignored`). That is a benchmark,
# not a lint fix, and doing it silently inside an unrelated change is how a report's numbers
# move for reasons nobody can trace.
DEFERRED = {
    '0e86797c34de1ee56e6ab03e644d8872': 'stages.rs: the pricing prompt - needs PROMPT_VERSION',
    '924ef4632ce06894b1c770be8cb4cbc0': 'stages.rs: the features prompt - needs PROMPT_VERSION',
    'aed8e01a14e9c5fd6ae6a1050f1acd03': 'stages.rs: the identity prompt - needs PROMPT_VERSION',
}


def digest(text):
    return hashlib.md5(text.encode('utf-8')).hexdigest()


def literals(src):
    """Every string literal in one file as (line, body), including ones spanning lines.

    A small state machine rather than a regex, because a regex cannot skip the things that
    would otherwise be mistaken for a quote: comments, char literals, and lifetimes.
    """
    out = []
    i, n, line = 0, len(src), 1
    while i < n:
        c = src[i]
        if c == '\n':
            line += 1
            i += 1
        elif c == '/' and src[i + 1:i + 2] == '/':
            j = src.find('\n', i)
            i = n if j < 0 else j
        elif c == '/' and src[i + 1:i + 2] == '*':
            j = src.find('*/', i + 2)
            end = n if j < 0 else j + 2
            line += src.count('\n', i, end)
            i = end
        elif c == "'":
            # `'a'` and `'\n'` are char literals; `'a` on its own is a lifetime.
            if src[i + 1:i + 2] == '\\':
                j = src.find("'", i + 2)
                i = n if j < 0 else j + 1
            elif src[i + 2:i + 3] == "'":
                i += 3
            else:
                i += 1
        elif c == 'r' and re.match(r'r#*"', src[i:i + 8] or ''):
            hashes = len(src[i + 1:]) - len(src[i + 1:].lstrip('#'))
            start = i + 1 + hashes + 1
            close = '"' + '#' * hashes
            j = src.find(close, start)
            body = src[start:n if j < 0 else j]
            out.append((line, body))
            line += body.count('\n')
            i = n if j < 0 else j + len(close)
        elif c == '"':
            start_line, j, buf = line, i + 1, []
            while j < n and src[j] != '"':
                if src[j] == '\\':
                    buf.append(src[j:j + 2])
                    j += 2
                else:
                    buf.append(src[j])
                    j += 1
            body = ''.join(buf)
            out.append((start_line, body))
            line += body.count('\n')
            i = j + 1
        else:
            i += 1
    return out


def main():
    bad, deferred, stale = [], 0, set(DEFERRED)
    for base, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = [d for d in dirs if d not in ('target', 'fixtures')]
        for name in files:
            if not name.endswith('.rs'):
                continue
            path = os.path.join(base, name)
            rel = os.path.relpath(path, ROOT).replace('\\', '/')
            src = io.open(path, encoding='utf-8').read()
            for line, body in literals(src):
                if not HOLE.search(body):
                    continue
                key = digest(body)
                if key in DEFERRED:
                    deferred += 1
                    stale.discard(key)
                    continue
                bad.append((rel, line, ' '.join(body.split())[:110], key))

    if bad:
        print('A sentence has a run of spaces in it, where a `\\` continuation used to be:\n')
        for rel, line, text, key in bad:
            print(f'  {rel}:{line}   [{key}]')
            print(f'    {text}')
        print()
        print('Rust strips the newline and the indentation after a trailing backslash. Without')
        print('the backslash the indentation stays, and the sentence reaches a reader with a')
        print('hole in it. Split the literal into several `println!`s, or use `concat!` - both')
        print('survive a reformat, which a continuation in this repository has repeatedly not.')
        print()
        print('If this one genuinely cannot be fixed here, add its digest to DEFERRED with the')
        print('reason. Do not add the file.')
        return 1

    if stale:
        print('A deferred string is no longer in the tree, so its entry is stale:\n')
        for key in sorted(stale):
            print(f'  {key}   {DEFERRED[key]}')
        print('\nRemove it from DEFERRED. An exemption nobody can point at is one nobody rechecks.')
        return 1

    extra = f' ({deferred} deferred by digest, see DEFERRED)' if deferred else ''
    print(f'no sentence has a lost continuation in it{extra}.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
