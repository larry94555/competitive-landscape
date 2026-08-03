# Builds the code-tour film page.
#
#   RECORD_PAGE=code-tour.html RECORD_OUT=video-code node prototype/record-demo.mjs
#   python prototype/build-code-tour.py
#
# The beats live in code-tour.html rather than in a Markdown walkthrough: this is one
# film about code that will change under it, and keeping the excerpts next to the beats
# that point at them means a rename shows up in one file rather than two.
import base64
import io
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build  # noqa: E402  — for the pacing constants, so there is one copy of them

PAGE = os.path.join(HERE, 'code-tour.html')
VIDEO = os.path.join(HERE, 'video-code')
OFFSET = 1.5  # page load and settle before the recorder starts the film


def beats():
    """Read the beat list straight out of the page, so it is defined in one place.

    Delimited by the array's own closing bracket rather than by the comment that used to
    follow it. Rewording a comment broke this build once, which is a silly way for a film
    to stop being buildable.
    """
    src = io.open(PAGE, encoding='utf-8').read()
    start = src.index('var BEATS = [')
    end = src.index('\n  ];', start)
    block = src[start:end]
    out = []
    for m in re.finditer(r'\{\s*kind:\s*"([a-z]+)",\s*a:\s*"([^"]+)",\s*c:\s*"([^"]*)"\s*\}', block):
        out.append({'kind': m.group(1), 'a': m.group(2), 'c': m.group(3)})
    if not out:
        raise SystemExit('code-tour.html: no beats found')
    return out


def lay_out(bs):
    """Same pacing rules as docs/Demo_Walkthrough.md §3, applied here too.

    The constants are imported rather than repeated. They were repeated once, and a
    change to one film's pacing silently left the other at the old timing — the same
    two-copies-of-a-constant problem that has cost this project a day more than once.
    """
    t = 0.0
    for b in bs:
        d = build.PAYOFF if b['kind'] in ('result', 'means') else build.BASE
        if re.search(r'\d', b['c']):
            d = max(d, build.NUMBER)
        d = max(d, len(b['c']) / build.CPS + build.PAD)
        if b['a'].startswith(('spot:', 'file:')):
            d += build.SETTLE
        b['t'] = round(t, 1)
        b['d'] = round(d, 1)
        t += b['d']
    return bs, round(t, 1)


def stamp(sec):
    sec = max(0.0, sec)
    return '%02d:%02d:%06.3f' % (int(sec // 3600), int((sec % 3600) // 60), sec % 60)


def main():
    bs, total = lay_out(beats())

    cues = ['WEBVTT', '']
    for i, b in enumerate(bs, start=1):
        cues += [
            str(i),
            '%s --> %s' % (stamp(b['t'] + OFFSET), stamp(b['t'] + b['d'] + OFFSET)),
            b['c'],
            '',
        ]
    vtt = os.path.join(VIDEO, 'landscape-code.vtt')
    io.open(vtt, 'w', encoding='utf-8', newline='\n').write('\n'.join(cues))

    webm = os.path.join(VIDEO, 'landscape-code.webm')
    if not os.path.exists(webm):
        raise SystemExit('no footage. Run the recorder first (see the header of this file).')

    chapter = json.load(io.open(os.path.join(VIDEO, 'chapters.json'), encoding='utf-8'))[0]
    tpl = io.open(os.path.join(HERE, 'page-template.html'), encoding='utf-8').read()

    def b64(p):
        return base64.b64encode(io.open(p, 'rb').read()).decode('ascii')

    html = (
        tpl.replace('__N__', '1')
        .replace('__OF__', '1')
        .replace('__TITLE__', chapter['title'])
        .replace('__BLURB__', chapter['blurb'])
        .replace('__DUR__', '%ds' % round(total))
        .replace('__CHAPTERS__', '<li class="cur"><span>1. %s</span><em>now playing</em></li>' % chapter['title'])
        .replace(
            '__MORE__',
            '<div class="more off"><b>The full source is in the pull request.</b> '
            '<span>github.com/larry94555/competitive-landscape</span></div>',
        )
        .replace('__V64__', b64(webm))
        .replace('__T64__', b64(vtt))
    )

    out = os.path.join(HERE, 'demo-code.html')
    io.open(out, 'w', encoding='utf-8').write(html)
    mb = os.path.getsize(out) / 1048576
    flag = '' if mb < 15.5 else '   ** over the 16MB artifact limit **'
    print('  code  %2d captions  %5.1fs  %5.2f MB   %s%s' % (len(bs), total, mb, os.path.basename(out), flag))


if __name__ == '__main__':
    main()
