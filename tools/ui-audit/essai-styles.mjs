import { start, openSite, frameOf, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
page.setDefaultTimeout(5000);
await openSite(page);
const target = process.argv[2] || 'h1';
let f = frameOf(page);
await f.locator(target).first().click({ position: { x: 8, y: 8 } }); await page.waitForTimeout(800);
const id = await f.evaluate((t) => document.querySelector(t).getAttribute('data-rv'), target);
console.log('calque :', await page.$eval('.layers .layer.is-selected', (e) => e.getAttribute('data-tech')));
const snap = () => frameOf(page).evaluate((id) => { const e = document.querySelector(`[data-rv="${id}"]`); if (!e) return 'ABSENT'; const cs = getComputedStyle(e); const r = e.getBoundingClientRect(); let s = Math.round(r.width) + 'x' + Math.round(r.height) + ';' + (e.getAttribute('class') || '') + ';' + (e.getAttribute('style') || '') + ';'; for (const p of cs) s += cs.getPropertyValue(p) + '|'; let h = 0; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0; return h; }, id);
const waitChange = async (base, max = 1500) => { const t0 = Date.now(); while (Date.now() - t0 < max) { const s = await snap().catch(() => 'ERR'); if (s !== base && s !== 'ERR') return Date.now() - t0; await page.waitForTimeout(15); } return -1; };
await page.evaluate(() => document.querySelectorAll('.styles details').forEach((d) => { d.open = true; }));
await page.waitForTimeout(400);
const rows = await page.$$eval('.styles .style-row, .styles .style-cell', (rs) => rs.map((r, i) => ({ i, label: (r.querySelector('.style-label-text, .style-label, label')?.textContent || r.getAttribute('aria-label') || '').trim().slice(0, 30), seg: r.querySelectorAll('.seg button').length, sel: r.querySelectorAll('select').length, nbox: r.querySelectorAll('.nbox-input, input.nbox').length, sw: r.querySelectorAll('.switch input').length, color: r.querySelectorAll('.color-input').length, section: r.closest('details')?.querySelector('summary')?.textContent.trim().slice(0, 20) || '' })));
console.log('lignes :', rows.length);
const res = []; const times = [];
const base0 = await snap();
for (const row of rows) {
  const loc = page.locator('.styles .style-row, .styles .style-cell').nth(row.i);
  let kind = row.seg ? 'seg' : row.sel ? 'select' : row.nbox ? 'nbox' : row.sw ? 'switch' : null;
  if (!kind) { res.push({ ...row, r: 'non testé (' + (row.color ? 'couleur' : 'autre') + ')' }); continue; }
  try {
    await page.evaluate(() => document.querySelectorAll('.styles details').forEach((d) => { d.open = true; }));
    const before = await snap();
    if (kind === 'seg') { const b = loc.locator('.seg button:not(.is-on):not(.is-implied)').last(); await b.click(); }
    else if (kind === 'select') { const s = loc.locator('select').first(); const v = await s.evaluate((e) => { const o = [...e.options].filter((o) => !o.selected && !o.disabled && o.value !== '').pop(); return o ? o.value : null; }); if (v === null) { res.push({ ...row, r: 'pas d\'autre option' }); continue; } await s.selectOption(v); }
    else if (kind === 'nbox') { const n = loc.locator('.nbox-input, input.nbox').first(); await n.click(); await n.fill(/ouleur/.test(row.label) ? '#ff0000' : '7'); await n.press('Enter'); }
    else { await loc.locator('.switch').first().click(); }
    const dt = await waitChange(before);
    if (dt < 0) res.push({ ...row, kind, r: 'SANS EFFET sur le canevas' }); else times.push(dt);
    await page.evaluate(() => document.activeElement && document.activeElement.blur());
    await page.keyboard.press('Control+z'); await page.waitForTimeout(350);
    let after = await snap();
    if (after !== before) { await page.waitForTimeout(600); after = await snap(); }
    if (after !== before && dt >= 0) res.push({ ...row, kind, r: 'NON RÉTABLI par Ctrl+Z' });
    if (after !== before) { for (let k = 0; k < 3 && (await snap()) !== base0; k++) { await page.keyboard.press('Control+z'); await page.waitForTimeout(400); } }
    const selOk = await page.$('.layers .layer.is-selected');
    if (!selOk) { res.push({ ...row, kind, r: 'SÉLECTION PERDUE' }); await frameOf(page).locator(target).first().click({ position: { x: 8, y: 8 } }); await page.waitForTimeout(600); }
  } catch (e) { res.push({ ...row, kind, r: 'ERREUR ' + e.message.split('\n')[0].slice(0, 80) }); await page.keyboard.press('Escape'); }
}
times.sort((a, b) => a - b);
console.log('réaction style → canevas (ms) : n=' + times.length + ' médiane=' + times[times.length >> 1] + ' p90=' + times[Math.floor(times.length * 0.9)] + ' max=' + times[times.length - 1]);
const bad = res.filter((r) => !/non testé|pas d'autre/.test(r.r));
console.log('non testés :', res.filter((r) => /non testé/.test(r.r)).map((r) => r.section + '/' + r.label).join(' ; ').slice(0, 500));
for (const b of bad) console.log(`${b.r} — ${b.section} / ${b.label} [${b.kind}]`);
console.log('état final identique au départ :', (await snap()) === base0);
console.log('erreurs :', errors.length, errors.slice(0, 3).join(' || ').slice(0, 400));
fs.writeFileSync(OUT + '/f1-' + target.replace(/\W/g, '') + '.json', JSON.stringify({ res, times }));
await browser.close();
