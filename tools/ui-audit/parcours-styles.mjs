import { start, openSite, audit, save, frameOf, scrollAudit, pop, OUT } from './lib.mjs';
const { browser, page, errors } = await start();
await openSite(page);
let f = frameOf(page);
const select = async (s, pos = { x: 6, y: 6 }) => { await f.click(s, { position: pos }); await page.waitForTimeout(700); };
await select('section.hero', { x: 6, y: 300 });
await page.evaluate(() => document.querySelectorAll('.styles details').forEach((d) => { if (!d.open) d.querySelector('summary').click(); }));
await page.waitForTimeout(500);
const kinds = await page.evaluate(() => { const out = {}; document.querySelectorAll('.styles .pop-trigger, .styles [aria-haspopup], .styles button, .styles select, .styles summary').forEach((e) => { const k = e.tagName.toLowerCase() + '.' + [...e.classList].filter((c) => !/^is-|can-/.test(c)).join('.'); out[k] = (out[k] || 0) + 1; }); return out; });
console.log('contrôles du panneau Styles :', JSON.stringify(kinds));
// Interactions.
await page.click('.dock-tab[data-panel="interactions"]'); await page.waitForTimeout(400);
const fams = await page.$$eval('.anim-family', (x) => x.map((e) => e.getAttribute('data-family')));
console.log('familles :', fams.join(','));
for (const fam of fams) { await page.click(`.anim-family[data-family="${fam}"] .anim-add`).catch(() => {}); await page.waitForTimeout(250); }
await scrollAudit(page, '.interactions', 'interactions-familles');
// Une animation de chaque famille posée : réglages visibles.
for (const fam of fams) { await page.locator(`.anim-family[data-family="${fam}"] .anim-tile`).first().click().catch(() => {}); await page.waitForTimeout(500); }
await scrollAudit(page, '.interactions', 'interactions-reglages');
await page.click('.act-family .anim-add').catch((e) => console.log('actions : ' + e.message.slice(0, 60))); await page.waitForTimeout(400);
await page.evaluate(() => { const e = window.__sc; if (e) e.scrollTop = e.scrollHeight; }); await page.waitForTimeout(200);
await audit(page, 'interactions-action');
console.log('interactions :', await page.evaluate(() => [...document.querySelectorAll('.interactions [data-setting]')].map((e) => e.getAttribute('data-setting')).join(',')));
save(OUT + '/audit.json');
console.log('erreurs :', errors.length, errors.slice(0, 5).join(' || '));
await browser.close();
