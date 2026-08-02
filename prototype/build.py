# Rebuilds the demo from narration.md.
#
#   python prototype/build.py            rebuild every chapter page
#   python prototype/build.py --check    validate the script, write nothing
#
# Reads the script, injects it into the prototype, and for each chapter writes a
# WebVTT track and a self-contained page. Does NOT re-record: the video carries no
# text of its own, so editing words never needs new footage.
#
# Chapter pages link to one another. Published URLs live in video/links.json —
# publish once, paste the URLs in, rebuild, republish.
import base64, io, json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
HTML = os.path.join(HERE, 'ui-prototype.html')
SCRIPT = os.path.join(HERE, 'narration.md')
VIDEO = os.path.join(HERE, 'video')

OFFSET = 1.5           # page load + settle before the demo starts in the recorder
BLANK = '_(intentionally blank)_'

# Words that do not belong in narration. docs/Video_Guidelines.md §2.2.
BANNED = ['parse', 'parsed', 'extract', 'render', 'spinner', 'diff', 'content hash',
          'deterministic', 'runtime', 'quota', 'rate limit', 'anonymous user',
          'degraded', 'truncated', 'watermark', 'source class', 'endpoint', 'API',
          'schema', 'pipeline', 'backend']


# ----------------------------------------------------------------- inputs
def read_script():
    body = io.open(SCRIPT, encoding='utf-8').read()
    out = []
    for m in re.finditer(r'^##\s*\[([a-z0-9-]+)\][^\n]*\n(.*?)(?=^##\s*\[|\Z)', body, re.S | re.M):
        text = m.group(2).strip()
        if text == BLANK:
            text = ''
        out.append((m.group(1), ' '.join(text.split())))
    if not out:
        raise SystemExit('narration.md: no "## [id]" headings found')
    return out


def demo_order():
    s = io.open(HTML, encoding='utf-8').read()
    block = s[s.index('  var DEMO = ['):s.index('  function playDemo')]
    return [(m.group(2), float(m.group(1)))
            for m in re.finditer(r'\{ t:([0-9.]+), id:"([a-z0-9-]+)"', block)]


def chapters():
    p = os.path.join(VIDEO, 'chapters.json')
    if not os.path.exists(p):
        raise SystemExit('video/chapters.json missing — run: node prototype/record-demo.mjs')
    chs = json.load(io.open(p, encoding='utf-8'))
    # from/to are not in chapters.json; recover them from the prototype
    s = io.open(HTML, encoding='utf-8').read()
    blk = s[s.index('  var CHAPTERS = ['):s.index('  function playDemo')]
    bounds = {m.group(1): (float(m.group(2)), float(m.group(3)))
              for m in re.finditer(r'id:"([a-z0-9-]+)"[\s\S]*?from:([0-9.]+),\s*to:([0-9.]+)', blk)}
    for c in chs:
        c['from'], c['to'] = bounds[c['id']]
    return chs


def links():
    p = os.path.join(VIDEO, 'links.json')
    return json.load(io.open(p, encoding='utf-8')) if os.path.exists(p) else {}


# ----------------------------------------------------------------- checks
def check(script, order):
    problems = []
    sids, oids = [i for i, _ in script], [i for i, _ in order]
    for extra in set(sids) - set(oids):
        problems.append('narration.md has [%s], which is not a step in the demo' % extra)
    for missing in set(oids) - set(sids):
        problems.append('demo step [%s] has no entry in narration.md' % missing)
    for sid, text in script:
        if not text:
            continue
        low = text.lower()
        for w in BANNED:
            if re.search(r'\b' + re.escape(w.lower()) + r'\b', low):
                problems.append('[%s] uses "%s" — see Video_Guidelines.md §2.2' % (sid, w))
        if not re.search(r'[.!?]$', text):
            problems.append('[%s] does not end in a full stop' % sid)
    return problems


# ----------------------------------------------------------------- outputs
def inject(script):
    s = io.open(HTML, encoding='utf-8').read()
    lines = ['  // <<NARRATION>> generated from narration.md by build.py — do not edit by hand',
             '  var N = {']
    for sid, text in script:
        lines.append('    "%s": "%s",' % (sid, text.replace('\\', '\\\\').replace('"', '\\"')))
    lines += ['  };', '  // <</NARRATION>>']
    new, n = re.subn(r'  // <<NARRATION>>.*?  // <</NARRATION>>',
                     lambda _m: '\n'.join(lines), s, flags=re.S)
    if n != 1:
        raise SystemExit('could not find the NARRATION block in ui-prototype.html')
    io.open(HTML, 'w', encoding='utf-8').write(new)


def stamp(sec):
    sec = max(0.0, sec)
    return '%02d:%02d:%06.3f' % (int(sec // 3600), int((sec % 3600) // 60), sec % 60)


def write_vtt(ch, script, order):
    """One track per chapter, timed from that chapter's start."""
    text_by_id = dict(script)
    inside = [(sid, t) for sid, t in order if ch['from'] <= t <= ch['to']]
    cues = []
    for i, (sid, t) in enumerate(inside):
        body = text_by_id.get(sid, '')
        if not body:
            continue
        j = i + 1
        while j < len(inside) and not text_by_id.get(inside[j][0], ''):
            j += 1
        end = inside[j][1] if j < len(inside) else t + 6.0
        cues.append((t - ch['from'] + OFFSET, end - ch['from'] + OFFSET, body))

    out = ['WEBVTT', '', 'NOTE',
           'Generated from prototype/narration.md. Edit that file, not this one.', '']
    for n, (a, b, body) in enumerate(cues, 1):
        out += [str(n), '%s --> %s' % (stamp(a), stamp(b)), body, '']
    path = os.path.join(VIDEO, 'landscape-%s.vtt' % ch['id'])
    io.open(path, 'w', encoding='utf-8').write('\n'.join(out))
    return len(cues)


PAGE = io.open(os.path.join(HERE, 'page-template.html'), encoding='utf-8').read()


def build_page(ch, chs, url_map):
    webm = os.path.join(VIDEO, 'landscape-%s.webm' % ch['id'])
    vtt = os.path.join(VIDEO, 'landscape-%s.vtt' % ch['id'])
    if not os.path.exists(webm):
        print('  no footage for %s — skipped' % ch['id'])
        return None

    v64 = base64.b64encode(open(webm, 'rb').read()).decode('ascii')
    t64 = base64.b64encode(open(vtt, 'rb').read()).decode('ascii')

    secs = int(ch['seconds'])
    dur = '%d:%02d' % (secs // 60, secs % 60)

    # chapter list, with links where we have them
    items = []
    for c in chs:
        cur = c['id'] == ch['id']
        href = url_map.get(c['id'])
        label = '%d. %s' % (c['n'], c['title'])
        if cur:
            items.append('<li class="cur"><span>%s</span><em>you are here</em></li>' % label)
        elif href:
            items.append('<li><a href="%s">%s</a></li>' % (href, label))
        else:
            items.append('<li><span class="soon">%s</span></li>' % label)

    nxt = next((c for c in chs if c['n'] == ch['n'] + 1), None)
    if nxt and url_map.get(nxt['id']):
        more = ('<a class="more" href="%s">More video demos available &rsaquo;<b>%d. %s</b></a>'
                % (url_map[nxt['id']], nxt['n'], nxt['title']))
    elif nxt:
        more = ('<div class="more off">More video demos available &rsaquo; <b>%d. %s</b>'
                '<span>link added when it is published</span></div>' % (nxt['n'], nxt['title']))
    else:
        first = chs[0]
        more = (('<a class="more" href="%s">Back to the beginning &rsaquo;<b>1. %s</b></a>'
                 % (url_map[first['id']], first['title'])) if url_map.get(first['id'])
                else '<div class="more off">That is the last of them.</div>')

    html = (PAGE
            .replace('__V64__', v64).replace('__T64__', t64)
            .replace('__N__', str(ch['n'])).replace('__OF__', str(len(chs)))
            .replace('__TITLE__', ch['title']).replace('__BLURB__', ch['blurb'])
            .replace('__DUR__', dur)
            .replace('__CHAPTERS__', '\n'.join(items))
            .replace('__MORE__', more))
    out = os.path.join(HERE, 'demo-%s.html' % ch['id'])
    io.open(out, 'w', encoding='utf-8').write(html)
    return out


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
    chs, url_map = chapters(), links()
    for ch in chs:
        n = write_vtt(ch, script, order)
        out = build_page(ch, chs, url_map)
        if out:
            mb = os.path.getsize(out) / 1048576
            flag = '' if mb < 15.5 else '   ** over the 16MB artifact limit **'
            print('  %d. %-9s %2d captions   %5.2f MB   %s%s'
                  % (ch['n'], ch['id'], n, mb, os.path.basename(out), flag))
    if not url_map:
        print('\nNo video/links.json yet — pages say "link added when it is published".')
        print('Publish them, save the URLs as {"report":"…","answers":"…","more":"…"}, rebuild.')


if __name__ == '__main__':
    main()
