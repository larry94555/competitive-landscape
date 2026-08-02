// Records the prototype walkthrough, one file per chapter.
//
//   node prototype/record-demo.mjs            all chapters
//   node prototype/record-demo.mjs answers    just one
//
// Three shorter films rather than one long one. Each stays well inside the 16MB
// artifact budget, which is what lets the frame be larger than a single long film
// could afford — and a viewer can stop after the first without missing the point.
//
// Captions are NOT burned in: they ship as a WebVTT track per chapter, written by
// build.py. Editing words therefore never needs a re-record.
import { chromium } from 'playwright';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { mkdirSync, readdirSync, renameSync, rmSync, existsSync, writeFileSync } from 'node:fs';

const here = dirname(fileURLToPath(import.meta.url));
const pageUrl = pathToFileURL(join(here, 'ui-prototype.html')).href;
const outDir = join(here, 'video');

const W = 1600, H = 1000, ZOOM = 1.35;
const only = process.argv[2] || null;

mkdirSync(outDir, { recursive: true });

let browser;
for (const channel of ['msedge', 'chrome', undefined]) {
  try {
    browser = await chromium.launch(channel ? { channel } : {});
    console.log(`launched: ${channel ?? 'bundled chromium'}`);
    break;
  } catch { /* try the next one */ }
}
if (!browser) {
  console.error('No Chromium available. Run: npx playwright install chromium');
  process.exit(1);
}

// Read the chapter list out of the prototype, so it is defined in exactly one place.
const probe = await browser.newContext({ viewport: { width: W, height: H } });
const probePage = await probe.newPage();
await probePage.goto(pageUrl, { waitUntil: 'load' });
const chapters = await probePage.evaluate(() => window.__chapters);
await probe.close();
writeFileSync(join(outDir, 'chapters.json'), JSON.stringify(chapters, null, 2));
console.log(`${chapters.length} chapters: ${chapters.map((c) => c.id).join(', ')}`);

for (const ch of chapters) {
  if (only && ch.id !== only) continue;

  const dir = join(outDir, '_' + ch.id);
  if (existsSync(dir)) rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });

  const ctx = await browser.newContext({
    viewport: { width: W, height: H },
    deviceScaleFactor: 1,
    colorScheme: 'light',
    recordVideo: { dir, size: { width: W, height: H } },
  });
  const page = await ctx.newPage();
  await page.goto(pageUrl, { waitUntil: 'load' });
  await page.addStyleTag({
    content: `html { zoom: ${ZOOM}; } .cap-bar { display: none !important; }`,
  });
  await page.waitForTimeout(1200);

  console.log(`  recording ${ch.id} (~${ch.seconds}s) …`);
  await page.evaluate((id) => window.__playDemo(id), ch.id);
  await page.waitForTimeout((ch.seconds + 4) * 1000);

  await ctx.close();

  const file = readdirSync(dir).find((f) => f.endsWith('.webm'));
  if (!file) { console.error(`  no video for ${ch.id}`); continue; }
  const dest = join(outDir, `landscape-${ch.id}.webm`);
  if (existsSync(dest)) rmSync(dest);
  renameSync(join(dir, file), dest);
  rmSync(dir, { recursive: true, force: true });
  console.log(`  wrote video/landscape-${ch.id}.webm`);
}

await browser.close();
console.log('done. Now: python prototype/build.py');
