# Rebuilds the demo from narration.md.
#
#   python prototype/build.py
#
# Reads the script, injects it into the prototype, regenerates the subtitle track,
# and rebuilds the standalone demo page. Does NOT re-record: the video carries no
# text of its own, so editing words never needs new footage.
#
#   python prototype/build.py --check    validate the script without writing anything
import io, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
HTML = os.path.join(HERE, 'ui-prototype.html')
SCRIPT = os.path.join(HERE, 'narration.md')
VTT = os.path.join(HERE, 'video', 'landscape-demo.vtt')

OFFSET = 1.5   # page load + settle before the demo starts in the recorder
BLANK = '_(intentionally blank)_'

# Words that do not belong in narration. docs/Video_Guidelines.md §2.2.
BANNED = ['parse', 'parsed', 'extract', 'render', 'spinner', 'diff', 'content hash',
          'deterministic', 'runtime', 'quota', 'rate limit', 'anonymous user',
          'degraded', 'truncated', 'watermark', 'source class', 'endpoint', 'API',
          'schema', 'pipeline', 'backend']


def read_script():
    """narration.md -> [(id, text)] in file order."""
    body = io.open(SCRIPT, encoding='utf-8').read()
    out = []
    for m in re.finditer(r'^##\s*\[([a-z0-9-]+)\][^\n]*\n(.*?)(?=^##\s*\[|\Z)',
                         body, re.S | re.M):
        text = m.group(2).strip()
        if text == BLANK:
            text = ''
        text = ' '.join(text.split())
        out.append((m.group(1), text))
    if not out:
        raise SystemExit('narration.md: no "## [id]" headings found')
    return out


def demo_order():
    """ui-prototype.html -> [(id, t)] in demo order."""
    s = io.open(HTML, encoding='utf-8').read()
    block = s[s.index('  var DEMO = ['):s.index('  function playDemo()')]
    return [(m.group(2), float(m.group(1)))
            for m in re.finditer(r'\{\s*t:\s*([0-9.]+)\s*,\s*id:"([a-z0-9-]+)"', block)]


def check(script, order):
    problems = []
    sids, oids = [i for i, _ in script], [i for i, _ in order]
    for extra in set(sids) - set(oids):
        problems.append('narration.md has [%s], which is not a step in the demo' % extra)
    for missing in set(oids) - set(sids):
        problems.append('demo step [%s] has no entry in narration.md' % missing)

    times = dict(order)
    for i, (sid, text) in enumerate(script):
        if not text or sid not in times:
            continue
        low = text.lower()
        for w in BANNED:
            if re.search(r'\b' + re.escape(w.lower()) + r'\b', low):
                problems.append('[%s] uses "%s" — see Video_Guidelines.md §2.2' % (sid, w))
        if not re.search(r'[.!?]$', text):
            problems.append('[%s] does not end in a full stop' % sid)
    return problems


def inject(script):
    s = io.open(HTML, encoding='utf-8').read()
    lines = ['  // <<NARRATION>> generated from narration.md by build.py — do not edit by hand',
             '  var N = {']
    for sid, text in script:
        lines.append('    "%s": "%s",' % (sid, text.replace('\\', '\\\\').replace('"', '\\"')))
    lines += ['  };', '  // <</NARRATION>>']
    # Lambda replacement: the script contains backslashes that re.sub would
    # otherwise read as group references. Count substitutions rather than
    # compare strings - an unchanged script is a no-op, not a failure.
    new, n = re.subn(r'  // <<NARRATION>>.*?  // <</NARRATION>>',
                     lambda _m: '\n'.join(lines), s, flags=re.S)
    if n != 1:
        raise SystemExit('could not find the NARRATION block in ui-prototype.html')
    io.open(HTML, 'w', encoding='utf-8').write(new)


def write_vtt(script, order):
    text_by_id = dict(script)
    cues = []
    for i, (sid, t) in enumerate(order):
        body = text_by_id.get(sid, '')
        if not body:
            continue
        j = i + 1
        while j < len(order) and not text_by_id.get(order[j][0], ''):
            j += 1
        end = order[j][1] if j < len(order) else t + 6.0
        cues.append((t, end, body))

    def stamp(sec):
        sec = max(0.0, sec + OFFSET)
        return '%02d:%02d:%06.3f' % (int(sec // 3600), int((sec % 3600) // 60), sec % 60)

    out = ['WEBVTT', '', 'NOTE',
           'Generated from prototype/narration.md. Edit that file, not this one.', '']
    for n, (a, b, body) in enumerate(cues, 1):
        out += [str(n), '%s --> %s' % (stamp(a), stamp(b)), body, '']
    os.makedirs(os.path.dirname(VTT), exist_ok=True)
    io.open(VTT, 'w', encoding='utf-8').write('\n'.join(out))
    return len(cues)


def main():
    script, order = read_script(), demo_order()
    problems = check(script, order)
    if problems:
        print('narration problems:')
        for p in problems:
            print('  -', p)
        if '--check' in sys.argv:
            sys.exit(1)
        print()
    if '--check' in sys.argv:
        print('narration.md is clean: %d lines, all ids match the demo.' % len(script))
        return

    inject(script)
    n = write_vtt(script, order)
    subprocess.run([sys.executable, os.path.join(HERE, 'make-demo-page.py')], check=True)
    print('rebuilt from narration.md: %d captions.' % n)
    print('Re-record only if what happens on screen changed:  node prototype/record-demo.mjs')


if __name__ == '__main__':
    main()
