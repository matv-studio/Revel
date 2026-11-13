import { start, openSite, audit, save, inventory, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
const inv = {};
const step = async (name) => { await audit(page, name); inv[name] = await inventory(page); };
await page.waitForSelector('.site-card');
await step('accueil');
await openSite(page);
await step('editeur');
// Onglets des panneaux ancrés.
const tabs = await page.$$eval('.dock-tab', (t) => t.map((x) => x.getAttribute('data-panel')));
console.log('panneaux :', tabs.join(', '));
for (const t of tabs) { await page.click(`.dock-tab[data-panel="${t}"]`); await page.waitForTimeout(400); await step('panneau-' + t); }
// Vues.
const views = await page.$$eval('.view-tab', (t) => t.map((x) => x.textContent.trim()));
console.log('vues :', views.join(', '));
for (let i = 1; i < views.length; i++) {
  await (await page.$$('.view-tab'))[i].click(); await page.waitForTimeout(1200);
  await step('vue-' + views[i].toLowerCase().replace(/[^a-z]+/g, '-'));
}
// Paramètres du site : chaque rubrique.
const navs = await page.$$eval('.settings-nav-item', (t) => t.map((x) => [x.getAttribute('data-section'), x.textContent.trim(), x.disabled || x.classList.contains('is-planned')]));
console.log('rubriques :', navs.map((n) => n[0] + (n[2] ? '(prévu)' : '')).join(', '));
for (const [id, , off] of navs) { if (off || !id) continue; await page.click(`.settings-nav-item[data-section="${id}"]`); await page.waitForTimeout(600); await step('parametres-' + id); }
save(OUT + '/audit.json');
fs.writeFileSync(OUT + '/inventaire.json', JSON.stringify(inv, null, 1));
console.log('erreurs :', errors.length, errors.slice(0, 5).join(' || '));
await browser.close();
