import { start, openSite, audit, save, scrollAudit, pop, inventory, escapes, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
await openSite(page);
const T = async (name, fn) => { try { await fn(); } catch (e) { console.log(`ÉTAPE EN ÉCHEC (${name}) : ` + e.message.split('\n')[0].slice(0, 140)); } };
await page.locator('.view-tab', { hasText: 'CMS' }).click(); await page.waitForTimeout(1200);
const cols = await page.$$eval('.cms-col', (c) => c.map((x) => x.textContent.trim()));
console.log('collections :', cols.join(' | '));
console.log((await inventory(page)).filter((x) => !/th-sort|row-link|cms-col/.test(x)).join(' ; ').slice(0, 1500));
await T('champs', () => pop(page, 'cms-champs', page.locator('.cms-main button', { hasText: 'Champs' }), { close: null, wait: 800 }));
await T('champs défilement', () => scrollAudit(page, '.cms-schema, .schema, .dialog', 'cms-champs-defile', { max: 5 }));
console.log('champs :', (await inventory(page)).slice(0, 60).join(' ; ').slice(0, 1200));
await page.keyboard.press('Escape'); await page.waitForTimeout(400);
console.log('après Échap (champs) :', (await page.$('.cms-schema, .schema')) ? 'toujours ouvert' : 'fermé');
await T('flux', () => pop(page, 'cms-flux', page.locator('.cms-main button', { hasText: 'Flux' })));
await T('modèle', () => pop(page, 'cms-modele-des-pages', page.locator('.cms-main button', { hasText: 'Modèle des pages' }), { wait: 1500 }));
console.log('après modèle :', await page.evaluate(() => document.querySelector('.view-tab.is-on')?.textContent));
await page.locator('.view-tab', { hasText: 'CMS' }).click(); await page.waitForTimeout(800);
await T('ctx ligne', () => pop(page, 'cms-ctx-contenu', '.cms-table tbody tr', { right: true }));
await T('ctx collection', () => pop(page, 'cms-ctx-collection', '.cms-col', { right: true }));
// Fiche d'un contenu de chaque collection.
for (let i = 0; i < cols.length; i++) {
  await T('collection ' + i, async () => {
    await page.locator('.cms-col').nth(i).click(); await page.waitForTimeout(700);
    await audit(page, 'cms-collection-' + i);
    await page.locator('.cms-table tbody tr').first().click(); await page.waitForTimeout(1200);
    await scrollAudit(page, '.cms-item', 'cms-fiche-' + i, { max: 7 });
    if (i === 0) console.log('fiche :', (await inventory(page)).filter((x) => /cms-item|rt-|btn|icon-btn/.test(x)).slice(0, 50).join(' ; ').slice(0, 1400));
    await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    if (await page.$('.cms-item')) { const back = page.locator('.cms-item .icon-btn, .cms-item-head button').first(); await back.click().catch(() => {}); await page.waitForTimeout(500); }
  });
}
await T('nouveau contenu', () => pop(page, 'cms-nouveau-contenu', page.locator('.cms-main .btn--primary'), { wait: 900 }));
save(OUT + '/audit.json');
fs.writeFileSync(OUT + '/echap-s4.json', JSON.stringify(escapes));
console.log('erreurs :', errors.length, errors.slice(0, 5).join(' || '));
await browser.close();
