import io, os, re

HERE = os.path.dirname(os.path.abspath(__file__))

# ---- 1. add the new demo step for the reach section ----
html = os.path.join(HERE, 'ui-prototype.html')
s = io.open(html, encoding='utf-8').read()

a = '''    { t:60,  c:N["where-it-came-from"], a:function(){ var e=$("sec-swot");'''
b = '''    { t:57,  id:"size-and-standing", c:N["size-and-standing"], a:function(){ var e=$("sec-reach"); if(e) e.scrollIntoView({behavior:"smooth",block:"center"}); } },
    { t:62,  id:"rankings-not-visits", c:N["rankings-not-visits"], a:function(){} },
    { t:67,  c:N["where-it-came-from"], a:function(){ var e=$("sec-swot");'''
assert a in s, 'anchor for reach steps not found'
s = s.replace(a, b, 1)

# shift the remaining steps later to make room
shifts = [(66, 73), (70, 77), (75, 82), (81, 88), (86, 93), (90, 97), (95, 102),
          (100, 107), (105, 112), (111, 118), (116, 123), (120, 127), (125, 132),
          (129, 136), (133, 140), (138, 145)]
for old, new in shifts:
    pat = '{ t:%d,  id:' % old
    if pat in s:
        s = s.replace(pat, '{ t:%d,  id:' % new, 1)
    else:
        pat2 = '{ t:%d, id:' % old
        if pat2 in s:
            s = s.replace(pat2, '{ t:%d, id:' % new, 1)
        else:
            print('  shift miss: t:%d' % old)

io.open(html, 'w', encoding='utf-8').write(s)
print('demo steps shifted; reach steps added')

# ---- 2. rewrite the three affected narration lines, add two ----
np = os.path.join(HERE, 'narration.md')
t = io.open(np, encoding='utf-8').read()


def replace_body(sid, new_body, doc):
    pat = re.compile(r'(^##\s*\[' + re.escape(sid) + r'\][^\n]*\n\n)(.*?)(?=\n##\s*\[|\Z)',
                     re.S | re.M)
    m = pat.search(doc)
    if not m:
        print('  narration miss:', sid)
        return doc
    return doc[:m.start(2)] + new_body + '\n' + doc[m.end(2):]


t = replace_body('reads-sites',
    'It reads each company own website. One site would not let a program read it, so that page '
    'is listed with a link, for you to open yourself.', t)

t = replace_body('price-elsewhere',
    'Somebody blog gives a price for the third company. It is not the company own figure, so it '
    'stays out of the table, but it is reported underneath with a link and a note saying why we '
    'could not confirm it.', t)

# insert the two new entries before [where-it-came-from]
anchor = re.search(r'^##\s*\[where-it-came-from\]', t, re.M)
block = (
    '## [size-and-standing] 0:57.0  ·  5s on screen\n\n'
    'It also looks up how busy each website is, and when each company started, from services '
    'the companies do not control.\n\n'
    '## [rankings-not-visits] 1:02.0  ·  5s on screen\n\n'
    'Each line says which service it came from and when. These are rankings against every other '
    'website, not visitor counts, because visitor numbers are guesses.\n\n'
)
t = t[:anchor.start()] + block + t[anchor.start():]

io.open(np, 'w', encoding='utf-8').write(t)
print('narration.md updated')
