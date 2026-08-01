# Builds the standalone demo page: video + caption track, both inlined as data URIs
# so the page is fully self-contained (Artifact CSP blocks external hosts).
import base64, io, os

HERE = os.path.dirname(os.path.abspath(__file__))
webm = open(os.path.join(HERE, 'video', 'landscape-demo.webm'), 'rb').read()
vtt = open(os.path.join(HERE, 'video', 'landscape-demo.vtt'), 'rb').read()

V64 = base64.b64encode(webm).decode('ascii')
T64 = base64.b64encode(vtt).decode('ascii')

PAGE = '''<title>Landscape — Product Demo</title>
<style>
  :root{--ground:#F6F8F6;--surface:#FFF;--ink:#141C19;--ink-soft:#3D4A45;--muted:#6B7772;
    --rule:#DCE3DE;--accent:#0F6E5C;--accent-soft:#E2F0EB;--accent-ink:#0A4A3E;
    --mono:ui-monospace,"SF Mono",SFMono-Regular,"Cascadia Mono",Menlo,Consolas,monospace;
    --sans:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Arial,sans-serif;}
  @media (prefers-color-scheme:dark){:root{--ground:#0D1311;--surface:#141C19;--ink:#E8EFEB;
    --ink-soft:#B8C5BF;--muted:#85958E;--rule:#26332E;--accent:#4FBFA3;--accent-soft:#12302A;--accent-ink:#7FD6BF;}}
  :root[data-theme="dark"]{--ground:#0D1311;--surface:#141C19;--ink:#E8EFEB;--ink-soft:#B8C5BF;
    --muted:#85958E;--rule:#26332E;--accent:#4FBFA3;--accent-soft:#12302A;--accent-ink:#7FD6BF;}
  :root[data-theme="light"]{--ground:#F6F8F6;--surface:#FFF;--ink:#141C19;--ink-soft:#3D4A45;
    --muted:#6B7772;--rule:#DCE3DE;--accent:#0F6E5C;--accent-soft:#E2F0EB;--accent-ink:#0A4A3E;}
  *{box-sizing:border-box}
  body{margin:0;background:var(--ground);color:var(--ink);font-family:var(--sans);line-height:1.6}
  .wrap{max-width:60rem;margin:0 auto;padding:2.5rem 1.25rem 4rem}
  .eyebrow{font-family:var(--mono);font-size:.75rem;letter-spacing:.1em;text-transform:uppercase;
    color:var(--accent);margin-bottom:.6rem}
  h1{font-size:clamp(1.6rem,1.3rem + 1.4vw,2.3rem);letter-spacing:-.03em;font-weight:690;
    margin:0 0 .6rem;text-wrap:balance}
  .sub{color:var(--ink-soft);max-width:58ch;margin:0 0 1.75rem;font-size:1.08rem}
  video{width:100%;border:1px solid var(--rule);border-radius:4px;background:#000;display:block;
    box-shadow:0 1px 2px rgba(20,28,25,.06),0 12px 32px -16px rgba(20,28,25,.28)}
  video::cue{background:rgba(10,14,13,.9);color:#F2F6F4;font-family:var(--sans);font-size:.9em}
  .meta{display:flex;flex-wrap:wrap;gap:.4rem 1.1rem;font-family:var(--mono);font-size:.76rem;
    color:var(--muted);margin-top:.8rem}
  .meta .ok{color:var(--accent)}
  .callout{background:var(--accent-soft);border-left:3px solid var(--accent);color:var(--accent-ink);
    padding:.9rem 1.1rem;border-radius:2px;font-size:.95rem;margin:1.5rem 0}
  .callout strong{font-weight:660}
  h2{font-size:.78rem;font-family:var(--mono);font-weight:600;letter-spacing:.08em;
    text-transform:uppercase;color:var(--muted);margin:2.5rem 0 1rem;padding-bottom:.5rem;
    border-bottom:1px solid var(--rule)}
  ol{margin:0;padding-left:0;list-style:none;counter-reset:s}
  li{counter-increment:s;padding:.55rem 0 .55rem 3rem;position:relative;
    border-bottom:1px solid var(--rule);color:var(--ink-soft)}
  li:last-child{border-bottom:0}
  li::before{content:counter(s,decimal-leading-zero);position:absolute;left:0;top:.55rem;
    font-family:var(--mono);font-size:.75rem;color:var(--accent)}
  li b{color:var(--ink);font-weight:640}
  .note{font-size:.9rem;color:var(--muted);max-width:60ch;margin-top:1.25rem}
</style>
<div class="wrap">
  <div class="eyebrow">Prototype walkthrough &middot; 1 min 53 s &middot; silent</div>
  <h1>Landscape, end to end.</h1>
  <p class="sub">All seven flows, recorded from the clickable prototype. Simulated data and
  compressed timings.</p>

  <div class="callout">
    <strong>There is no audio track at all.</strong> Captions are burned into the picture, so
    watching muted is the only way it is meant to be watched. A selectable WebVTT track ships
    alongside &mdash; turn it on with the <em>CC</em> control if you want copyable text or a
    screen&#8209;reader&#8209;accessible version.
  </div>

  <video controls preload="metadata" playsinline>
    <source src="data:video/webm;base64,__V64__" type="video/webm">
    <track kind="captions" srclang="en" label="English (selectable)"
           src="data:text/vtt;base64,__T64__">
    Your browser cannot play WebM. The file is in the repo at
    <code>prototype/video/landscape-demo.webm</code>.
  </video>
  <div class="meta">
    <span class="ok">Captions burned in</span>
    <span class="ok">WebVTT track included (23 cues)</span>
    <span>No audio</span>
    <span>1440&times;900</span>
    <span>WebM / VP8</span>
  </div>

  <h2>What it covers</h2>
  <ol>
    <li><b>Unregistered flow</b> &mdash; one analysis a day, full report, no signup wall</li>
    <li><b>The wait, honestly</b> &mdash; sources landing, pricing parsed before prose, gaps that
        list what was checked</li>
    <li><b>Citations</b> &mdash; URL, timestamp, content hash and the quoted line, one click away</li>
    <li><b>Follow-up conversation</b> &mdash; including a question the sources cannot answer,
        declined rather than invented</li>
    <li><b>Registered flow</b> &mdash; one an hour, ten follow-ups</li>
    <li><b>Subscribed flow</b> &mdash; five an hour, unlimited follow-ups, identical report</li>
    <li><b>Notification flow</b> &mdash; the alert email and the diff behind it</li>
    <li><b>Community</b> &mdash; seven channels; sign-up and log-in issues open to anyone</li>
    <li><b>Admin</b> &mdash; usage, capacity, and every rate limit editable at runtime</li>
  </ol>

  <p class="note"><strong>This is a prototype, not the product.</strong> Faked data, no backend,
  throwaway code. It exists to answer one question before any production code is written: does a
  90&ndash;180 second wait read as work happening, or as a hang? The demo plays at 8&times; speed
  &mdash; to feel the real thing, open the prototype and switch the header toggle to
  <em>real</em>.</p>
</div>
'''

html = PAGE.replace('__V64__', V64).replace('__T64__', T64)
out = os.path.join(HERE, 'demo-page.html')
io.open(out, 'w', encoding='utf-8').write(html)
print('wrote %s (%.2f MB)' % (out, os.path.getsize(out) / 1048576))
