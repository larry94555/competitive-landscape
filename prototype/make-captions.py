# Generates a WebVTT track from the demo script in ui-prototype.html.
# The narration lives in the prototype source, so a wrong caption is caught in
# review like any other wrong code (CODING_QUALITY.md 9.5).
import io, os, re

HERE = os.path.dirname(os.path.abspath(__file__))
OFFSET = 1.5  # page load + settle before window.__playDemo() fires in the recorder

src = io.open(os.path.join(HERE, 'ui-prototype.html'), encoding='utf-8').read()
block = src[src.index('var DEMO = ['):src.index('function playDemo()')]

# { t:17.5, c:"…", a:function(){…} }
pattern = r'\{\s*t:\s*([0-9.]+)\s*,\s*c:\s*"((?:[^"\\]|\\.)*)"'
raw = [(float(m.group(1)), m.group(2)) for m in re.finditer(pattern, block, re.S)]
if not raw:
    raise SystemExit('no demo steps parsed - did the DEMO array shape change?')


def unescape(s):
    return s.replace("\\'", "'").replace('\\"', '"').replace('\\n', ' ')


# A step with c:"" keeps the previous caption on screen; extend the cue instead.
cues = []
for i, (t, c) in enumerate(raw):
    text = unescape(c).strip()
    if not text:
        continue
    j = i + 1
    while j < len(raw) and not unescape(raw[j][1]).strip():
        j += 1
    end = raw[j][0] if j < len(raw) else t + 6.0
    cues.append((t, end, text))


def stamp(sec):
    sec = max(0.0, sec + OFFSET)
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = sec % 60
    return '%02d:%02d:%06.3f' % (h, m, s)


lines = [
    'WEBVTT',
    '',
    'NOTE',
    'Landscape prototype walkthrough. These captions are also burned into the video',
    'frame; this track exists for accessibility tooling, text extraction and reuse.',
    '',
]
for n, (a, b, text) in enumerate(cues, 1):
    lines += [str(n), '%s --> %s' % (stamp(a), stamp(b)), text, '']

os.makedirs(os.path.join(HERE, 'video'), exist_ok=True)
out = os.path.join(HERE, 'video', 'landscape-demo.vtt')
io.open(out, 'w', encoding='utf-8').write('\n'.join(lines))
print('wrote %d cues -> %s' % (len(cues), out))
print('first:', cues[0][2][:64])
print('last :', cues[-1][2][:64])
