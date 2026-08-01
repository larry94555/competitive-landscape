// Records the prototype demo to WebM. Throwaway tooling, like the prototype it films.
// Captions are burned into the page itself, so no ffmpeg compositing step is needed.
//
//   node prototype/record-demo.mjs
//
import { chromium } from 'playwright';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { mkdirSync, readdirSync, renameSync, rmSync, existsSync } from 'node:fs';

const here = dirname(fileURLToPath(import.meta.url));
const page_url = pathToFileURL(join(here, 'ui-prototype.html')).href;
const outDir = join(here, 'video');

if (existsSync(outDir)) rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

const RUNTIME_MS = 113_000; // demo script is ~107s; a little tail for the end card

const launchOpts = { args: ['--force-prefers-reduced-motion=0'] };
let browser;
for (const channel of ['msedge', 'chrome', undefined]) {
  try {
    browser = await chromium.launch(channel ? { ...launchOpts, channel } : launchOpts);
    console.log(`launched: ${channel ?? 'bundled chromium'}`);
    break;
  } catch (e) {
    console.log(`  ${channel ?? 'bundled'} unavailable`);
  }
}
if (!browser) {
  console.error('No Chromium available. Run: npx playwright install chromium');
  process.exit(1);
}

const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 1,
  colorScheme: 'light',
  recordVideo: { dir: outDir, size: { width: 1440, height: 900 } },
});

const page = await context.newPage();
await page.goto(page_url, { waitUntil: 'load' });
await page.waitForTimeout(1200);

console.log('recording…');
await page.evaluate(() => window.__playDemo());
await page.waitForTimeout(RUNTIME_MS);

await context.close();
await browser.close();

// Playwright names videos with a random hash; give it a stable name.
const file = readdirSync(outDir).find((f) => f.endsWith('.webm'));
if (file) {
  renameSync(join(outDir, file), join(outDir, 'landscape-demo.webm'));
  console.log('wrote prototype/video/landscape-demo.webm');
} else {
  console.error('no video produced');
  process.exit(1);
}
