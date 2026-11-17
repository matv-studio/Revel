// Amorce commune : ouvre l'éditeur sur le site d'essai (simulateur Tauri + pont du canevas).
import { createRequire } from 'node:module';
import fs from 'node:fs';
const require = createRequire('/home/claude/revel/tools/');
export const { chromium } = require('playwright');
export const UI = 'http://127.0.0.1:8170', BRIDGE = 'http://127.0.0.1:8174';
const mock = fs.readFileSync('/home/claude/revel/apps/revel-ui/dev/mock-tauri.js', 'utf8');
export const invoke = async (cmd, args = {}) => (await fetch(BRIDGE + '/__revel/invoke/' + cmd, { method: 'POST', body: JSON.stringify(args) })).json();
export async function open(browser, theme = 'dark', size = { width: 1440, height: 900 }) {
  const page = await browser.newPage({ viewport: size, deviceScaleFactor: 1 });
  page.on('pageerror', (e) => console.log('pageerror', e.message.slice(0, 200)));
  await page.addInitScript(mock);
  await page.goto(`${UI}/#lang=fr&theme=${theme}&accent=8b5cf6&scope=full&bridge=${encodeURIComponent(BRIDGE)}`);
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame');
  const frameOf = () => page.frames().find((f) => f.url().includes('rv='));
  for (let i = 0; i < 200; i++) {
    const f = frameOf();
    try { if (f && await f.evaluate(() => document.readyState === 'complete' && !!document.querySelector('[data-rv]'))) break; } catch {}
    await page.waitForTimeout(100);
  }
  return { page, frameOf };
}
