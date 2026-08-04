# Compiles docs/Demo_Walkthrough.md into the demo.
#
#   python prototype/build.py            regenerate the prototype and every film page
#   python prototype/build.py --check    validate only, write nothing
#   python prototype/build.py --preview  print each film's computed timings
#
# The walkthrough is the single source: beats, actions and narration all live there.
# Step timings are COMPUTED from the pacing rules (Demo_Walkthrough.md §3), never
# authored, which is what makes those rules enforceable rather than aspirational.
#
# Re-record only when the picture changes. Wording, timing and ordering all flow
# from the walkthrough without new footage.
import base64, io, json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
HTML = os.path.join(HERE, 'ui-prototype.html')
WALK = os.path.join(os.path.dirname(HERE), 'docs', 'Demo_Walkthrough.md')
VIDEO = os.path.join(HERE, 'video')

OFFSET = 1.5           # page load + settle before the recorder starts the film

# --- pacing rules, Demo_Walkthrough.md §3 -------------------------------------
# Cut four times: 20%, 20%, 30%, 30%. These floors are ~31% of the first cut.
#
# **This cut changed almost nothing, and that was predicted.** At the previous values the
# reading floor already decided most beats; at these it decides nearly all of them. The
# slack is spent. Every further second has to come out of the captions.
#
# Only the SLACK has ever been cut. **CPS is untouched and must stay that way** — it is
# the one number that decides whether a caption can be read at all, and the others are
# only decisions about how long to wait afterwards. Cutting CPS would not make the film
# tighter, it would make it unreadable, which reads as *slower* because the viewer
# rewinds.
#
# **The slack is now nearly gone, and that changes what the next cut has to be.** At these
# values the reading floor decides almost every beat: `characters ÷ CPS + PAD` already
# exceeds BASE for any caption past about 28 characters. Another cut here would move
# almost nothing. Shortening the captions is the lever from this point on — which is what
# Video_Text_Best_Practices.md asks for anyway, so the two agree.
#
# `build-code-tour.py` imports these rather than keeping its own copy. Two sets of
# pacing constants drift, and the drift is invisible until someone watches both films
# back to back.
BASE = 1.3             # every beat
PAYOFF = 1.9           # result and means beats
NUMBER = 1.9           # any line containing a digit
SETTLE = 0.65          # after a scroll or a highlight
CPS = 14.0             # characters a second, reading aloud - NEVER reduced
PAD = 0.32             # breathing room after a caption is readable
# The floor tracks the pacing; the ceiling does not, and the asymmetry is the point.
#
# FILM_MIN asks "does this film have enough in it to be worth starting?", and it was
# calibrated against the original pacing. Cutting the slack shortens every film by about
# a fifth without removing a single beat, so a floor that stayed at 35s would flag three
# films for saying exactly as much as they said yesterday. It moves with the pacing.
#
# FILM_MAX is not about pacing at all. It is how long someone will actually watch, which
# no editing decision of ours changes. It stays where it is.
FILM_MIN, FILM_MAX = 16.0, 70.0
MAX_RESULTS = 4

KINDS = {'recognition', 'frustration', 'turn', 'action', 'wait', 'result', 'means', 'point', 'close', 'next'}
VERBS = {'hold', 'clean', 'type', 'go', 'report', 'to', 'spot', 'cite', 'ask', 'tier', 'view', 'adv', 'diff'}
MOVES = ('to', 'spot', 'cite')          # actions that move the page, so they need settling

# Words that do not belong in narration. docs/Video_Guidelines.md §2.2.
BANNED = ['parse', 'parsed', 'extract', 'render', 'spinner', 'diff', 'content hash',
          'deterministic', 'runtime', 'quota', 'rate limit', 'anonymous user',
          'degraded', 'truncated', 'watermark', 'source class', 'endpoint', 'API',
          'schema', 'pipeline', 'backend']


# ----------------------------------------------------------------- parse
def read_walkthrough():
    """Every '### Film n · `id` — Title' heading, its blurb, and its beat table."""
    body = io.open(WALK, encoding='utf-8').read()
    films, n = [], 0
    pat = re.compile(r'^###\s+Film\s+\d+\s*[·.]\s*`([a-z0-9-]+)`\s*[—-]\s*(.+?)\s*$', re.M)
    marks = list(pat.finditer(body))
    for i, m in enumerate(marks):
        chunk = body[m.end():marks[i + 1].start() if i + 1 < len(marks) else len(body)]
        n += 1
        blurb = ''
        bm = re.search(r'^\*\*Blurb\.\*\*\s*(.+?)\s*$', chunk, re.M)
        if bm:
            blurb = bm.group(1)
        beats = []
        for row in re.finditer(r'^\|\s*([a-z]+)\s*\|\s*`([^`]+)`\s*\|\s*(.+?)\s*\|\s*$', chunk, re.M):
            kind, action, text = row.group(1), row.group(2), row.group(3).strip()
            beats.append({'kind': kind, 'a': action, 'c': '' if text == '-' else text})
        if beats:
            films.append({'id': m.group(1), 'n': n, 'title': m.group(2), 'blurb': blurb, 'beats': beats})
    if not films:
        raise SystemExit('Demo_Walkthrough.md: no film tables found')
    return films


def selectors_in(html):
    """Ids and class names the prototype actually contains, for validating actions.

    Class attributes are collected loosely: many are built by string concatenation
    inside the script, so the attribute value is not clean HTML. Pulling identifier
    tokens out of whatever is there beats trying to parse it properly."""
    names = set(re.findall(r'id="([A-Za-z0-9_-]+)"', html))
    for c in re.findall(r'class="([^"]*)"', html):
        names |= set(re.findall(r'[A-Za-z][A-Za-z0-9_-]*', c))
    return names


# ----------------------------------------------------------------- timing
def duration(beat):
    """Beat length, derived. Nothing here is authored by hand."""
    d = PAYOFF if beat['kind'] in ('result', 'means') else BASE
    text = beat['c']
    if re.search(r'\d', text):
        d = max(d, NUMBER)
    if text:
        d = max(d, len(text) / CPS + PAD)
    if beat['a'].split(':')[0] in MOVES:
        d += SETTLE
    return round(d, 1)


def lay_out(films):
    """Stamp each beat with its start time and length; each film with its total."""
    for f in films:
        t = 0.0
        for b in f['beats']:
            b['t'] = round(t, 1)
            b['d'] = duration(b)
            t += b['d']
        f['seconds'] = round(t, 1)
    return films


# ----------------------------------------------------------------- checks
def check(films, html):
    have, problems = selectors_in(html), []
    for f in films:
        where = 'film %s' % f['id']
        kinds = [b['kind'] for b in f['beats']]

        for i, b in enumerate(f['beats']):
            if b['kind'] not in KINDS:
                problems.append('%s: unknown beat kind "%s"' % (where, b['kind']))
            # The rule the old films broke: a result with nothing explaining what it is.
            if b['kind'] == 'result' and (i + 1 >= len(kinds) or kinds[i + 1] != 'means'):
                problems.append('%s: result "%s" is not followed by a means'
                                % (where, b['c'][:44]))
            verb = b['a'].split(':')[0]
            if verb not in VERBS:
                problems.append('%s: unknown action "%s"' % (where, b['a']))
            if verb in ('spot', 'to', 'cite'):
                arg = b['a'].split(':', 1)[1]
                for tok in re.findall(r'[#.]([A-Za-z0-9_-]+)', arg):
                    if tok not in have:
                        problems.append('%s: action "%s" targets "%s", which is not in the prototype'
                                        % (where, b['a'], tok))
            if b['c']:
                low = b['c'].lower()
                for w in BANNED:
                    if re.search(r'\b' + re.escape(w.lower()) + r'\b', low):
                        problems.append('%s: "%s" — see Video_Guidelines.md §2.2' % (where, w))
                if not re.search(r'[.!?]$', b['c']):
                    problems.append('%s: "%s" does not end in a full stop' % (where, b['c'][:44]))
                if len(b['c']) / CPS + PAD > b['d'] + 0.05:
                    problems.append('%s: "%s" cannot be read in %.1fs' % (where, b['c'][:36], b['d']))

        n_res = kinds.count('result')
        if n_res > MAX_RESULTS:
            problems.append('%s: %d results, limit is %d — split it' % (where, n_res, MAX_RESULTS))
        if n_res < 2:
            problems.append('%s: only %d result(s) — a film needs at least 2' % (where, n_res))
        if not (FILM_MIN <= f['seconds'] <= FILM_MAX):
            problems.append('%s: %.1fs, outside %g-%gs' % (where, f['seconds'], FILM_MIN, FILM_MAX))
    return problems


# ----------------------------------------------------------------- outputs
def inject(films):
    s = io.open(HTML, encoding='utf-8').read()
    out = ['  // <<FILMS>> generated from docs/Demo_Walkthrough.md by build.py - do not edit by hand',
           '  var FILMS = [']
    for f in films:
        out.append('    { id:%s, n:%d, title:%s, blurb:%s, seconds:%s, beats:['
                   % (js(f['id']), f['n'], js(f['title']), js(f['blurb']), f['seconds']))
        for b in f['beats']:
            out.append('      { t:%s, d:%s, kind:%s, a:%s, c:%s },'
                       % (b['t'], b['d'], js(b['kind']), js(b['a']), js(b['c'])))
        out.append('    ] },')
    out += ['  ];', '  // <</FILMS>>']
    new, n = re.subn(r'  // <<FILMS>>.*?  // <</FILMS>>', lambda _m: '\n'.join(out), s, flags=re.S)
    if n != 1:
        raise SystemExit('could not find the FILMS block in ui-prototype.html')
    io.open(HTML, 'w', encoding='utf-8').write(new)


def js(v):
    return '"%s"' % v.replace('\\', '\\\\').replace('"', '\\"')


def stamp(sec):
    sec = max(0.0, sec)
    return '%02d:%02d:%06.3f' % (int(sec // 3600), int((sec % 3600) // 60), sec % 60)


def write_vtt(f):
    cues, n = ['WEBVTT', ''], 0
    for b in f['beats']:
        if not b['c']:
            continue
        n += 1
        cues += ['%d' % n,
                 '%s --> %s' % (stamp(b['t'] + OFFSET), stamp(b['t'] + b['d'] + OFFSET)),
                 b['c'], '']
    p = os.path.join(VIDEO, 'landscape-%s.vtt' % f['id'])
    io.open(p, 'w', encoding='utf-8', newline='\n').write('\n'.join(cues))
    return n


def b64(p):
    return base64.b64encode(io.open(p, 'rb').read()).decode('ascii')


def build_page(f, films, urls):
    webm = os.path.join(VIDEO, 'landscape-%s.webm' % f['id'])
    vtt = os.path.join(VIDEO, 'landscape-%s.vtt' % f['id'])
    if not os.path.exists(webm):
        return None
    tpl = io.open(os.path.join(HERE, 'page-template.html'), encoding='utf-8').read()

    rows = []
    for o in films:
        url = urls.get(o['id'])
        cur = o['id'] == f['id']
        label = '%d. %s' % (o['n'], o['title'])
        if cur:
            rows.append('<li class="cur"><span>%s</span><em>now playing</em></li>' % label)
        elif url:
            rows.append('<li><a href="%s">%s</a><em>%ds</em></li>' % (url, label, round(o['seconds'])))
        else:
            rows.append('<li class="soon"><span>%s</span><em>not published yet</em></li>' % label)

    nxt = next((o for o in films if o['n'] == f['n'] + 1), None)
    if nxt and urls.get(nxt['id']):
        more = ('<a class="more" href="%s"><b>Next:</b> %s <span>&rarr;</span></a>'
                % (urls[nxt['id']], nxt['title']))
    elif nxt:
        more = '<div class="more off"><b>Next:</b> %s <span>not published yet</span></div>' % nxt['title']
    else:
        more = '<div class="more off"><b>That is the walkthrough.</b> <span>Now try it yourself.</span></div>'

    html = (tpl.replace('__N__', str(f['n'])).replace('__OF__', str(len(films)))
               .replace('__TITLE__', f['title']).replace('__BLURB__', f['blurb'])
               .replace('__DUR__', '%ds' % round(f['seconds']))
               .replace('__CHAPTERS__', '\n    '.join(rows)).replace('__MORE__', more)
               .replace('__V64__', b64(webm)).replace('__T64__', b64(vtt)))
    out = os.path.join(HERE, 'demo-%s.html' % f['id'])
    io.open(out, 'w', encoding='utf-8').write(html)
    return out


# ----------------------------------------------------------------- main
def main():
    films = lay_out(read_walkthrough())
    html = io.open(HTML, encoding='utf-8').read()
    problems = check(films, html)

    if '--preview' in sys.argv:
        for f in films:
            print('\n%d. %-9s %-46s %5.1fs' % (f['n'], f['id'], f['title'], f['seconds']))
            for i, b in enumerate(f['beats']):
                print('   %2d %-11s %-34s %5.1fs  %s'
                      % (i + 1, b['kind'], b['a'], b['d'], b['c'][:58]))
        print()

    if problems:
        print('walkthrough problems (%d):' % len(problems))
        for p in problems:
            print('  -', p)
        if '--check' in sys.argv:
            sys.exit(1)
        print()
    elif '--check' in sys.argv or '--preview' in sys.argv:
        total = sum(f['seconds'] for f in films)
        print('walkthrough is clean: %d films, %d beats, %.0fs total.'
              % (len(films), sum(len(f['beats']) for f in films), total))

    if '--check' in sys.argv or '--preview' in sys.argv:
        return

    inject(films)
    urls = {}
    p = os.path.join(VIDEO, 'links.json')
    if os.path.exists(p):
        urls = json.load(io.open(p, encoding='utf-8'))
    built = 0
    for f in films:
        n = write_vtt(f)
        out = build_page(f, films, urls)
        if out:
            built += 1
            mb = os.path.getsize(out) / 1048576
            flag = '' if mb < 15.5 else '   ** over the 16MB artifact limit **'
            print('  %2d. %-9s %2d captions  %5.1fs  %5.2f MB   %s%s'
                  % (f['n'], f['id'], n, f['seconds'], mb, os.path.basename(out), flag))
    if built < len(films):
        print('\n%d film(s) have no footage yet. Record with:  node prototype/record-demo.mjs'
              % (len(films) - built))


if __name__ == '__main__':
    main()
