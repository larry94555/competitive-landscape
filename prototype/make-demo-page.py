# Builds the standalone demo page.
#
# Video + caption track are inlined as data URIs so the page is self-contained
# (the Artifact CSP blocks external hosts). Music and narration are *generated in
# the browser* rather than baked into a file: Web Audio for the ambient bed,
# Web Speech for the voice. That keeps the page from growing by tens of megabytes
# and means the voice tracks the caption cues exactly.
import base64, io, os

HERE = os.path.dirname(os.path.abspath(__file__))
webm = open(os.path.join(HERE, 'video', 'landscape-demo.webm'), 'rb').read()
vtt = open(os.path.join(HERE, 'video', 'landscape-demo.vtt'), 'rb').read()

V64 = base64.b64encode(webm).decode('ascii')
T64 = base64.b64encode(vtt).decode('ascii')

PAGE = r'''<title>Landscape — Product Demo</title>
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

  .stage{background:#0A0E0D;padding:0;position:relative}
  .stage video{display:block;width:100%;height:auto;max-height:88vh;margin:0 auto;background:#000}
  video::cue{background:rgba(8,12,11,.9);color:#F2F6F4;font-family:var(--sans);font-size:.95em;line-height:1.4}

  .bar{position:sticky;top:0;z-index:5;background:var(--surface);border-bottom:1px solid var(--rule);
    display:flex;gap:.5rem;align-items:center;flex-wrap:wrap;padding:.6rem 1rem}
  .bar .grow{margin-right:auto;font-family:var(--mono);font-size:.75rem;color:var(--muted)}
  .tog{font:inherit;font-size:.82rem;font-weight:600;border:1px solid var(--rule-strong,#C3CDC7);
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
  <button class="tog" id="tMusic" aria-pressed="false"><span class="led"></span>Music</button>
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
  <span>narration: your browser's speech engine</span>
  <span>music: generated live, no file</span>
  <span>1 min 53 s</span>
</div>

<div class="wrap">
  <div class="eyebrow">Prototype walkthrough</div>
  <h1>Landscape, end to end.</h1>
  <p class="sub">All seven flows, recorded from the clickable prototype. Simulated data and
  compressed timings.</p>

  <div class="callout">
    <strong>The recording itself has no audio.</strong> Captions are burned into the picture, so it
    reads fine muted. <em>Narration</em> and <em>Music</em> above are generated in your browser
    rather than baked into the file &mdash; the voice reads each caption as it appears, and the
    music is synthesised live, so neither adds a byte to the download. Both are off until you turn
    them on.
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

  /* ---------------- narration: Web Speech, driven by the caption cues -------- */
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
    u.onstart = function () { duck(true); };
    u.onend = function () { duck(false); };
    u.onerror = function () { duck(false); };
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

  /* ---------------- music: generated live with Web Audio --------------------- */
  // A slow modal pad in D with occasional bell tones. Deliberately quiet and
  // uneventful: it should be noticeable only if you go looking for it.
  var ac = null, musicGain = null, musicOn = false, bellTimer = null, voices = [];
  var ROOTS = [73.42, 98.00, 87.31, 65.41];       // D2  G2  F2  C2
  var BELLS = [587.33, 659.25, 783.99, 880.00, 1174.66]; // D5 E5 G5 A5 D6
  var chordIdx = 0;

  function ensureAudio() {
    if (ac) return;
    ac = new (window.AudioContext || window.webkitAudioContext)();
    musicGain = ac.createGain();
    musicGain.gain.value = 0.0;

    var wet = ac.createGain(); wet.gain.value = 0.28;
    var delay = ac.createDelay(2.0); delay.delayTime.value = 0.42;
    var fb = ac.createGain(); fb.gain.value = 0.34;
    var damp = ac.createBiquadFilter(); damp.type = 'lowpass'; damp.frequency.value = 1800;
    delay.connect(fb); fb.connect(damp); damp.connect(delay);
    delay.connect(wet); wet.connect(ac.destination);
    musicGain.connect(ac.destination); musicGain.connect(delay);
    window.__wet = wet;

    var lfo = ac.createOscillator(); lfo.frequency.value = 0.045;
    var lfoAmt = ac.createGain(); lfoAmt.gain.value = 240;
    lfo.connect(lfoAmt); lfo.start();
    window.__lfo = lfoAmt;
  }

  function padVoice(freq, at, dur) {
    var f = ac.createBiquadFilter();
    f.type = 'lowpass'; f.frequency.value = 520; f.Q.value = 0.6;
    window.__lfo.connect(f.frequency);
    var g = ac.createGain(); g.gain.value = 0;
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(0.16, at + 2.6);
    g.gain.setValueAtTime(0.16, at + dur - 3.0);
    g.gain.linearRampToValueAtTime(0, at + dur);
    [0, 1].forEach(function (k) {
      var o = ac.createOscillator();
      o.type = k ? 'triangle' : 'sawtooth';
      o.frequency.value = freq * (k ? 2 : 1);
      o.detune.value = k ? 5 : -5;
      o.connect(f); o.start(at); o.stop(at + dur + .1);
      voices.push(o);
    });
    f.connect(g); g.connect(musicGain);
  }

  function chord(at) {
    var r = ROOTS[chordIdx % ROOTS.length]; chordIdx++;
    padVoice(r, at, 13);
    padVoice(r * 1.5, at, 13);      // fifth
    padVoice(r * 3.0, at + 1.2, 11);
  }

  function bell() {
    if (!ac || !musicOn) return;
    var f = BELLS[Math.floor(Math.random() * BELLS.length)];
    var t = ac.currentTime, o = ac.createOscillator(), g = ac.createGain();
    o.type = 'sine'; o.frequency.value = f;
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(0.055, t + 0.02);
    g.gain.exponentialRampToValueAtTime(0.0008, t + 4.5);
    o.connect(g); g.connect(musicGain); g.connect(window.__wet);
    o.start(t); o.stop(t + 4.8);
    bellTimer = setTimeout(bell, 4200 + Math.random() * 6500);
  }

  function startMusic() {
    ensureAudio();
    if (ac.state === 'suspended') ac.resume();
    musicOn = true;
    musicGain.gain.cancelScheduledValues(ac.currentTime);
    musicGain.gain.linearRampToValueAtTime(0.5, ac.currentTime + 2.5);
    var t = ac.currentTime + 0.1;
    for (var i = 0; i < 12; i++) chord(t + i * 11.5);   // ~2.3 min of bed
    clearTimeout(bellTimer);
    bellTimer = setTimeout(bell, 6000);
  }

  function stopMusic() {
    musicOn = false;
    clearTimeout(bellTimer);
    if (!ac) return;
    musicGain.gain.cancelScheduledValues(ac.currentTime);
    musicGain.gain.linearRampToValueAtTime(0.0, ac.currentTime + 1.2);
    voices.forEach(function (o) { try { o.stop(ac.currentTime + 1.4); } catch (e) {} });
    voices = [];
  }

  // Duck the bed while the voice is speaking so words stay clear.
  function duck(on) {
    if (!ac || !musicOn) return;
    musicGain.gain.cancelScheduledValues(ac.currentTime);
    musicGain.gain.linearRampToValueAtTime(on ? 0.16 : 0.5, ac.currentTime + 0.35);
  }

  /* ---------------- controls ------------------------------------------------- */
  function setStatus() {
    var bits = [];
    bits.push(voiceOn ? 'narration on' : 'silent video');
    if (musicOn) bits.push('music on');
    bits.push('captions burned in');
    status.textContent = bits.join(' · ');
  }

  var tVoice = document.getElementById('tVoice');
  var tMusic = document.getElementById('tMusic');

  tVoice.onclick = function () {
    voiceOn = !voiceOn;
    this.setAttribute('aria-pressed', voiceOn ? 'true' : 'false');
    if (!voiceOn && window.speechSynthesis) { speechSynthesis.cancel(); duck(false); }
    else {
      var t = vid.textTracks && vid.textTracks[0];
      var c = t && t.activeCues && t.activeCues[0];
      if (c && !vid.paused) say(c.text);
    }
    setStatus();
  };

  tMusic.onclick = function () {
    var on = this.getAttribute('aria-pressed') !== 'true';
    this.setAttribute('aria-pressed', on ? 'true' : 'false');
    on ? startMusic() : stopMusic();
    setStatus();
  };

  document.getElementById('tFull').onclick = function () {
    var el = vid;
    if (document.fullscreenElement) { document.exitFullscreen(); return; }
    (el.requestFullscreen || el.webkitRequestFullscreen || el.webkitEnterFullscreen || function () {}).call(el);
  };

  document.getElementById('playAll').onclick = function () {
    if (!voiceOn) tVoice.click();
    if (tMusic.getAttribute('aria-pressed') !== 'true') tMusic.click();
    vid.currentTime = 0;
    vid.play();
  };

  vid.addEventListener('pause', function () { if (window.speechSynthesis) speechSynthesis.cancel(); duck(false); });
  vid.addEventListener('seeking', function () { if (window.speechSynthesis) speechSynthesis.cancel(); });
  vid.addEventListener('ended', function () { if (window.speechSynthesis) speechSynthesis.cancel(); stopMusic(); tMusic.setAttribute('aria-pressed','false'); setStatus(); });

  setStatus();
})();
</script>
'''

html = PAGE.replace('__V64__', V64).replace('__T64__', T64)
out = os.path.join(HERE, 'demo-page.html')
io.open(out, 'w', encoding='utf-8').write(html)
print('wrote %s (%.2f MB)' % (out, os.path.getsize(out) / 1048576))
