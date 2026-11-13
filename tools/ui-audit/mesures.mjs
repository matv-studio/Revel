import { start, frameOf, UI, OUT } from './lib.mjs';
import fs from 'fs';
const out = {};
const { browser, page, errors } = await start();
await page.addInitScript(() => { window.__lt = []; try { new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__lt.push([Math.round(e.startTime), Math.round(e.duration)]); }).observe({ entryTypes: ['longtask'] }); } catch {} });
let t0 = Date.now();
await page.reload();
await page.waitForSelector('.site-open', { timeout: 30000 });
out['chargement interface (ms)'] = Date.now() - t0;
const res = await page.evaluate(() => performance.getEntriesByType('resource').filter((r) => /\.wasm|\.js|\.css/.test(r.name)).map((r) => [r.name.split('/').pop().replace(/-[0-9a-f]{8,}/, ''), r.encodedBodySize || r.transferSize, Math.round(r.duration)]));
out['ressources'] = res.map((r) => `${r[0]} ${(r[1] / 1048576).toFixed(2)} Mo ${r[2]} ms`).join(' ; ');
t0 = Date.now();
await page.click('.site-open');
await page.waitForSelector('iframe.canvas-frame');
out['ouverture du site → cadre (ms)'] = Date.now() - t0;
await page.waitForFunction(() => document.querySelectorAll('.layers .layer').length > 3, null, { timeout: 30000 });
out['ouverture du site → calques (ms)'] = Date.now() - t0;
await page.waitForTimeout(2500);
// Réaction : action → prochaine image peinte après que la condition est vraie.
const react = async (act, cond, arg) => { const t = Date.now(); await act(); await page.waitForFunction(cond, arg, { polling: 'raf', timeout: 15000 }); return Date.now() - t; };
const med = (a) => { a = [...a].sort((x, y) => x - y); return a[a.length >> 1]; };
// Vues
const views = ['CMS', 'Localisation', 'Plan du site', 'Paramètres', 'Éditeur'];
const vt = {};
for (let round = 0; round < 3; round++) for (const v of views) { const d = await react(() => page.locator('.view-tab', { hasText: v }).click(), (v) => document.querySelector('.view-tab.is-on')?.textContent.trim() === v && [...document.querySelectorAll('.view-pane')].some((p) => p.getBoundingClientRect().width > 0 && p.querySelector('*')), v); (vt[v] = vt[v] || []).push(d); await page.waitForTimeout(350); }
out['changement de vue (ms, 1er / médiane)'] = Object.entries(vt).map(([k, a]) => `${k} ${a[0]} / ${med(a)}`).join(' ; ');
// Onglets du dock
const tabs = await page.$$eval('.dock-tab', (t) => t.map((x) => x.getAttribute('data-panel')));
const dt = [];
for (let round = 0; round < 2; round++) for (const p of tabs) { dt.push(await react(() => page.click(`.dock-tab[data-panel="${p}"]`), (p) => document.querySelector(`.dock-tab[data-panel="${p}"]`)?.classList.contains('is-on') || document.querySelector(`.dock-tab[data-panel="${p}"]`)?.getAttribute('aria-selected') === 'true', p).catch(() => -1)); await page.waitForTimeout(120); }
out['onglets des panneaux (ms)'] = `médiane ${med(dt.filter((x) => x >= 0))} max ${Math.max(...dt)} (${dt.filter((x) => x < 0).length} sans réponse)`;
await page.click('.dock-tab[data-panel="styles"]').catch(() => {}); await page.click('.dock-tab[data-panel="layers"]').catch(() => {});
// Sélection → panneau de styles
const f = frameOf(page);
const targets = ['h1', 'nav a', 'section:nth-of-type(2) h2', 'section:nth-of-type(2) p', 'footer a', 'img'];
const st = [];
for (const t of targets) { const loc = f.locator(t).first(); if (!(await loc.count())) continue; await loc.scrollIntoViewIfNeeded().catch(() => {}); const before = await page.$eval('.layers', (l) => l.querySelector('.layer.is-selected')?.textContent || ''); const d = await react(() => loc.click({ position: { x: 4, y: 4 }, timeout: 4000 }), (b) => { const s = document.querySelector('.layers .layer.is-selected'); return s && s.textContent !== b && document.querySelector('.styles .style-row'); }, before).catch(() => -1); st.push(d); await page.waitForTimeout(300); }
out['sélection → calque et styles (ms)'] = st.join(', ');
// Annuler / rétablir après un réglage
await page.evaluate(() => document.querySelectorAll('.styles details').forEach((d) => { d.open = true; }));
const n = page.locator('.styles [data-prop="opacity"] .nbox-input').first();
const ut = [];
for (let i = 0; i < 4; i++) { await n.click(); await n.fill(String(30 + i * 10)); await n.press('Enter'); await page.waitForTimeout(350); await page.evaluate(() => document.activeElement.blur()); ut.push(await react(() => page.keyboard.press('Control+z'), (v) => { const i = document.querySelector('.styles [data-prop="opacity"] .nbox-input'); return i && i.value !== v; }, String(30 + i * 10)).catch(() => -1)); await page.waitForTimeout(250); }
out['annuler → panneau à jour (ms)'] = ut.join(', ');
// Palette de commandes
const pt = []; for (let i = 0; i < 3; i++) { pt.push(await react(() => page.keyboard.press('Control+k'), () => !!document.querySelector('.palette input'))); await page.keyboard.press('Escape'); await page.waitForTimeout(250); }
out['palette de commandes (ms)'] = pt.join(', ');
const lt = await page.evaluate(() => window.__lt);
out['tâches longues (> 50 ms)'] = `${lt.length} ; les plus longues : ${lt.map((x) => x[1]).sort((a, b) => b - a).slice(0, 8).join(', ')} ms`;
out['mémoire JS (Mo)'] = await page.evaluate(() => performance.memory ? Math.round(performance.memory.usedJSHeapSize / 1048576) : null);
out['erreurs'] = errors.length;
for (const [k, v] of Object.entries(out)) console.log(k + ' : ' + v);
fs.writeFileSync(OUT + '/perf.json', JSON.stringify(out, null, 1));
await browser.close();
