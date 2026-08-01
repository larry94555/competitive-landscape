# Builds the standalone demo page.
#
# Video + caption track are inlined as data URIs so the page is self-contained
# (the Artifact CSP blocks external hosts). Narration is generated in the browser
# with Web Speech, driven by the caption cues, so it cannot drift out of sync and
# adds nothing to the download.
import base64, io, os

HERE = os.path.dirname(os.path.abspath(__file__))
webm = open(os.path.join(HERE, 'video', 'landscape-demo.webm'), 'rb').read()
vtt = open(os.path.join(HERE, 'video', 'landscape-demo.vtt'), 'rb').read()

V64 = base64.b64encode(webm).decode('ascii')
T64 = base64.b64encode(vtt).decode('ascii')

PAGE = r'''<title>Landscape — Product Demo</title>
<style>
  :root{--ground:#F6F8F6;--surface:#FFF;--ink:#141C19;--ink-soft:#3D4A45;--muted:#6B7772;
    --rule:#DCE3DE;--rule-strong:#C3CDC7;--accent:#0F6E5C;--accent-soft:#E2F0EB;--accent-ink:#0A4A3E;
    --mono:ui-monospace,"SF Mono",SFMono-Regular,"Cascadia Mono",Menlo,Consolas,monospace;
    --sans:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Arial,sans-serif;}
  @media (prefers-color-scheme:dark){:root{--ground:#0D1311;--surface:#141C19;--ink:#E8EFEB;
    --ink-soft:#B8C5BF;--muted:#85958E;--rule:#26332E;--rule-strong:#35453E;
    --accent:#4FBFA3;--accent-soft:#12302A;--accent-ink:#7FD6BF;}}
  :root[data-theme="dark"]{--ground:#0D1311;--surface:#141C19;--ink:#E8EFEB;--ink-soft:#B8C5BF;
    --muted:#85958E;--rule:#26332E;--rule-strong:#35453E;--accent:#4FBFA3;--accent-soft:#12302A;--accent-ink:#7FD6BF;}
  :root[data-theme="light"]{--ground:#F6F8F6;--surface:#FFF;--ink:#141C19;--ink-soft:#3D4A45;
    --muted:#6B7772;--rule:#DCE3DE;--rule-strong:#C3CDC7;--accent:#0F6E5C;--accent-soft:#E2F0EB;--accent-ink:#0A4A3E;}
  *{box-sizing:border-box}
  body{margin:0;background:var(--ground);color:var(--ink);font-family:var(--sans);line-height:1.6}

  .stage{background:#0A0E0D;position:relative}
  .stage video{display:block;width:100%;height:auto;max-height:88vh;margin:0 auto;background:#000}
  video::cue{background:rgba(8,12,11,.9);color:#F2F6F4;font-family:var(--sans);font-size:.95em;line-height:1.4}

  .bar{position:sticky;top:0;z-index:5;background:var(--surface);border-bottom:1px solid var(--rule);
    display:flex;gap:.5rem;align-items:center;flex-wrap:wrap;padding:.6rem 1rem}
  .bar .grow{margin-right:auto;font-family:var(--mono);font-size:.75rem;color:var(--muted)}
  .tog{font:inherit;font-size:.82rem;font-weight:600;border:1px solid var(--rule-strong);
    background:var(--surface);color:var(--ink-soft);border-radius:2px;padding:.4rem .8rem;cursor:pointer;
    display:inline-flex;align-items:center;gap:.4rem;transition:background .12s,color .12s,border-color .12s}
  .tog:hover{border-color:var(--accent);color:var(--accent)}
  .tog[aria-pressed="true"]{background:var(--accent);border-color:var(--accent);color:#fff}
  .tog .led{width:.45rem;height:.45rem;border-radius:50%;background:currentColor;opacity:.35}
  .tog[aria-pressed="true"] .led{opacity:1}
  .tog.primary{background:var(--accent);border-color:var(--accent);color:#fff}
  .tog.primary:hover{opacity:.9;color:#fff}

  .wrap{max-width:60rem;margin:0 auto;padding:2rem 1.25rem 4rem}
  .eyebrow{font-family:var(--mono);font-size:.75rem;letter-spacing:.1em;text-transform:uppercase;
    color:var(--accent);margin-bottom:.5rem}
  h1{font-size:clamp(1.6rem,1.3rem + 1.4vw,2.3rem);letter-spacing:-.03em;font-weight:690;
    margin:0 0 .6rem;text-wrap:balance}
  .sub{color:var(--ink-soft);max-width:58ch;margin:0 0 1.5rem;font-size:1.08rem}
  .callout{background:var(--accent-soft);border-left:3px solid var(--accent);color:var(--accent-ink);
    padding:.9rem 1.1rem;border-radius:2px;font-size:.95rem;margin:1.5rem 0}
  h2{font-size:.78rem;font-family:var(--mono);font-weight:600;letter-spacing:.08em;text-transform:uppercase;
    color:var(--muted);margin:2.25rem 0 1rem;padding-bottom:.5rem;border-bottom:1px solid var(--rule)}
  ol{margin:0;padding-left:0;list-style:none;counter-reset:s}
  li{counter-increment:s;padding:.55rem 0 .55rem 3rem;position:relative;border-bottom:1px solid var(--rule);
    color:var(--ink-soft)}
  li:last-child{border-bottom:0}
  li::before{content:counter(s,decimal-leading-zero);position:absolute;left:0;top:.55rem;
    font-family:var(--mono);font-size:.75rem;color:var(--accent)}
  li b{color:var(--ink);font-weight:640}
  .note{font-size:.9rem;color:var(--muted);max-width:62ch;margin-top:1.25rem}
  .meta{display:flex;flex-wrap:wrap;gap:.35rem 1.1rem;font-family:var(--mono);font-size:.74rem;
    color:var(--muted);padding:.6rem 1rem;border-bottom:1px solid var(--rule)}
  .meta .ok{color:var(--accent)}
</style>

<div class="bar">
  <span class="grow" id="status">silent video · captions burned in</span>
  <button class="tog primary" id="playAll">▶ Play with narration</button>
  <button class="tog" id="tVoice" aria-pressed="false"><span class="led"></span>Narration</button>
  <button class="tog" id="tFull">⛶ Fullscreen</button>
</div>

<div class="stage">
  <video id="vid" controls preload="metadata" playsinline>
    <source src="data:video/webm;base64,__V64__" type="video/webm">
    <track id="trk" kind="captions" srclang="en" label="English" default
           src="data:text/vtt;base64,__T64__">
    Your browser cannot play WebM. The file is in the repo at
    <code>prototype/video/landscape-demo.webm</code>.
  </video>
</div>

<div class="meta">
  <span class="ok">1600×1000</span>
  <span class="ok">captions burned in + selectable track</span>
  <span>optional narration: your browser's speech engine</span>
  <span>1 min 53 s</span>
</div>

<div class="wrap">
  <div class="eyebrow">Prototype walkthrough</div>
  <h1>Landscape, end to end.</h1>
  <p class="sub">All seven flows, recorded from the clickable prototype. Simulated data and
  compressed timings.</p>

  <div class="callout">
    <strong>The recording has no audio.</strong> Captions are burned into the picture, so it reads
    fine muted. <em>Narration</em> is optional and generated in your browser &mdash; the voice reads
    each caption as it appears, so it cannot drift out of sync, and it adds nothing to the download.
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
  &mdash; to feel the real thing, open the prototype and switch the header toggle to <em>real</em>.</p>
</div>

<script>
(function () {
  "use strict";
  var vid = document.getElementById('vid');
  var status = document.getElementById('status');
  var voiceOn = false, chosenVoice = null;

  function pickVoice() {
    var vs = window.speechSynthesis ? speechSynthesis.getVoices() : [];
    if (!vs.length) return null;
    var en = vs.filter(function (v) { return /^en(-|_|$)/i.test(v.lang); });
    var pool = en.length ? en : vs;
    // Prefer the neural/"Natural" voices where a browser exposes them.
    var prefs = [/natural/i, /aria|jenny|libby|sonia|ava|samantha|serena|zoe/i, /google/i];
    for (var i = 0; i < prefs.length; i++) {
      var hit = pool.filter(function (v) { return prefs[i].test(v.name); })[0];
      if (hit) return hit;
    }
    return pool[0];
  }
  if (window.speechSynthesis) {
    chosenVoice = pickVoice();
    speechSynthesis.onvoiceschanged = function () { chosenVoice = pickVoice() || chosenVoice; };
  }

  function say(text) {
    if (!voiceOn || !window.speechSynthesis || !text) return;
    speechSynthesis.cancel();
    var u = new SpeechSynthesisUtterance(text.replace(/\s+/g, ' ').trim());
    if (chosenVoice) { u.voice = chosenVoice; u.lang = chosenVoice.lang; }
    u.rate = 1.08; u.pitch = 1.0; u.volume = 1.0;
    speechSynthesis.speak(u);
  }

  function hookCues() {
    var t = vid.textTracks && vid.textTracks[0];
    if (!t) return;
    t.mode = 'hidden';                 // burned-in captions are already visible
    t.addEventListener('cuechange', function () {
      var c = t.activeCues && t.activeCues[0];
      if (c) say(c.text);
    });
  }
  if (vid.readyState >= 1) hookCues(); else vid.addEventListener('loadedmetadata', hookCues);

  function setStatus() {
    status.textContent = (voiceOn ? 'narration on' : 'silent video') + ' · captions burned in';
  }

  var tVoice = document.getElementById('tVoice');

  tVoice.onclick = function () {
    voiceOn = !voiceOn;
    this.setAttribute('aria-pressed', voiceOn ? 'true' : 'false');
    if (!voiceOn && window.speechSynthesis) speechSynthesis.cancel();
    else {
      var t = vid.textTracks && vid.textTracks[0];
      var c = t && t.activeCues && t.activeCues[0];
      if (c && !vid.paused) say(c.text);
    }
    setStatus();
  };

  document.getElementById('tFull').onclick = function () {
    if (document.fullscreenElement) { document.exitFullscreen(); return; }
    (vid.requestFullscreen || vid.webkitRequestFullscreen || vid.webkitEnterFullscreen || function () {}).call(vid);
  };

  document.getElementById('playAll').onclick = function () {
    if (!voiceOn) tVoice.click();
    vid.currentTime = 0;
    vid.play();
  };

  vid.addEventListener('pause', function () { if (window.speechSynthesis) speechSynthesis.cancel(); });
  vid.addEventListener('seeking', function () { if (window.speechSynthesis) speechSynthesis.cancel(); });
  vid.addEventListener('ended', function () { if (window.speechSynthesis) speechSynthesis.cancel(); });

  setStatus();
})();
</script>
'''

html = PAGE.replace('__V64__', V64).replace('__T64__', T64)
out = os.path.join(HERE, 'demo-page.html')
io.open(out, 'w', encoding='utf-8').write(html)
print('wrote %s (%.2f MB)' % (out, os.path.getsize(out) / 1048576))
