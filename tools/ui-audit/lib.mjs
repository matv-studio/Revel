import { createRequire } from 'module';
import fs from 'fs';
const require = createRequire(new URL('../', import.meta.url));
export const { chromium } = require('playwright');
const mock = fs.readFileSync(new URL('../../apps/revel-ui/dev/mock-tauri.js', import.meta.url), 'utf8');
const auditSrc = fs.readFileSync(new URL('./audit-page.js', import.meta.url), 'utf8');
export const UI = process.env.REVEL_UI || 'http://127.0.0.1:8171', BRIDGE = process.env.REVEL_BRIDGE || 'http://127.0.0.1:8162';
export const OUT = process.env.REVEL_AUDIT_OUT || 'ui-audit-out';
export async function start({ theme = 'dark', lang = 'fr', w = 1440, h = 900, extra = '' } = {}) {
  const browser = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push('pageerror: ' + e.message.slice(0, 300)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push('console: ' + m.text().slice(0, 300)); });
  await page.addInitScript(mock);
  await page.goto(`${UI}/#lang=${lang}&theme=${theme}${extra}&bridge=${encodeURIComponent(BRIDGE)}`);
  return { browser, ctx, page, errors };
}
export async function openSite(page) {
  await page.waitForSelector('.site-open', { timeout: 30000 });
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame', { timeout: 30000 });
  await page.waitForTimeout(2500);
}
export const frameOf = (page) => page.frames().find((f) => f.url().includes('rv='));
export const invoke = async (cmd, args = {}) => { const r = await fetch(`${BRIDGE}/__revel/invoke/${cmd}`, { method: 'POST', body: JSON.stringify(args) }); const t = await r.text(); try { return JSON.parse(t); } catch { return t; } };
const results = {};
export async function audit(page, name, { shot = true, dir = OUT } = {}) {
  fs.mkdirSync(dir, { recursive: true });
  await page.waitForTimeout(250);
  if (shot) await page.screenshot({ path: `${dir}/${name}.png` });
  const r = await page.evaluate(auditSrc);
  results[name] = r;
  return r;
}
export function save(file) {
  let prev = {};
  try { prev = JSON.parse(fs.readFileSync(file, 'utf8')); } catch {}
  fs.writeFileSync(file, JSON.stringify({ ...prev, ...results }, null, 0));
}
// Inventaire des éléments actionnables visibles (libellé, classe).
export const inventory = (page) => page.evaluate(() => [...document.querySelectorAll('button, a[href], [role=tab], [role=button], select, input, summary')].filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && r.top < innerHeight && r.left < innerWidth; }).map((e) => (e.tagName.toLowerCase() + '.' + [...e.classList].slice(0, 2).join('.') + ' «' + ((e.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 30) || e.getAttribute('aria-label') || e.getAttribute('title') || e.getAttribute('placeholder') || '') + '»')));
// Parcourt un conteneur défilant par pas et contrôle chaque portion.
export async function scrollAudit(page, innerSel, name, { shot = true, max = 12 } = {}) {
  const ok = await page.evaluate((s) => { let e = document.querySelector(s); if (!e) return false; while (e && e !== document.body) { const cs = getComputedStyle(e); if (/(auto|scroll)/.test(cs.overflowY) && e.scrollHeight > e.clientHeight + 4) break; e = e.parentElement; } if (!e || e === document.body) { window.__sc = null; return true; } window.__sc = e; e.scrollTop = 0; return true; }, innerSel);
  if (!ok) { console.log('absent : ' + innerSel); return; }
  for (let i = 0; i < max; i++) {
    await audit(page, `${name}-${i}`, { shot });
    const more = await page.evaluate(() => { const e = window.__sc; if (!e) return false; const before = e.scrollTop; e.scrollTop += Math.round(e.clientHeight * 0.85); return e.scrollTop > before + 2; });
    if (!more) break;
    await page.waitForTimeout(200);
  }
  await page.evaluate(() => { if (window.__sc) window.__sc.scrollTop = 0; });
}
// Ouvre un élément flottant (menu, fenêtre), le contrôle, le ferme par Échap.
export async function pop(page, name, target, { right = false, close = 'Escape', wait = 400, frame = null, pos = undefined } = {}) {
  try {
    const loc = typeof target === 'string' ? (frame || page).locator(target).first() : target;
    await loc.click({ button: right ? 'right' : 'left', timeout: 4000, position: pos });
    await page.waitForTimeout(wait);
    await audit(page, name);
  } catch (e) { console.log('ÉCHEC ' + name + ' : ' + e.message.split('\n')[0].slice(0, 120)); }
  if (close) {
    await page.keyboard.press(close); await page.waitForTimeout(250);
    const left = await openFloats(page);
    if (left.length) {
      console.log(`ÉCHAP NE FERME PAS (${name}) : ${left.join(', ')}`);
      escapes.push({ name, left });
      await page.mouse.click(700, 500); await page.waitForTimeout(250);
      const still = await openFloats(page);
      if (still.length) { console.log(`  clic ailleurs ne ferme pas non plus : ${still.join(', ')}`); await page.keyboard.press('Escape'); await page.evaluate(() => document.querySelectorAll('details[open]').forEach((d) => { if (!d.closest('.styles, .advanced-settings, .interactions')) d.open = false; })); }
    }
  }
}
export const escapes = [];
export const openFloats = (page) => page.evaluate(() => [...document.querySelectorAll('.menu, .pop, .ctx-menu, .overlay, .dialog, [role=menu], [role=dialog]')].filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && (!e.checkVisibility || e.checkVisibility({ contentVisibilityAuto: true, checkOpacity: true, checkVisibilityCSS: true })); }).map((e) => e.tagName.toLowerCase() + '.' + [...e.classList].slice(0, 2).join('.')));
