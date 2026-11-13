import { start, openSite, audit, save, scrollAudit, pop, inventory, escapes, openFloats, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
page.setDefaultTimeout(6000);
const T = async (name, fn) => { try { await fn(); } catch (e) { console.log(`ÉTAPE EN ÉCHEC (${name}) : ` + e.message.split('\n')[0].slice(0, 140)); } };
const esc = async () => { await page.keyboard.press('Escape'); await page.waitForTimeout(250); };
await page.waitForSelector('.site-open', { timeout: 30000 });
// Accueil
await audit(page, 'accueil-grille');
await T('liste', async () => { await page.locator('button[aria-label="Liste"], button[title="Liste"]').first().click(); await page.waitForTimeout(300); await audit(page, 'accueil-liste'); await page.locator('button[aria-label="Grille"], button[title="Grille"]').first().click(); });
await T('nouveau site', async () => { await page.locator('.btn--primary', { hasText: 'Nouveau site' }).click(); await page.waitForTimeout(700); await scrollAudit(page, '.dialog, .new-site', 'accueil-nouveau-site', { max: 3 }); await esc(); console.log('nouveau site après Échap :', (await openFloats(page)).join(',') || 'fermé'); if ((await openFloats(page)).length) await page.locator('.dialog button', { hasText: 'Annuler' }).first().click().catch(() => {}); });
await T('importer', async () => { await page.locator('.btn', { hasText: /^Importer$/ }).click(); await page.waitForTimeout(700); await audit(page, 'accueil-importer'); console.log('importer :', (await inventory(page)).filter((x) => /dialog|import|menu-item/.test(x)).slice(0, 14).join(' ; ').slice(0, 500)); await esc(); console.log('importer après Échap :', (await openFloats(page)).join(',') || 'fermé'); });
await T('profil', () => pop(page, 'accueil-profil', '.profile-btn'));
await T('carte ctx', () => pop(page, 'accueil-ctx-carte', '.site-card, .site-open', { right: true }));
await T('param site', async () => { await page.locator('[aria-label="Paramètres du site"]').first().click(); await page.waitForTimeout(700); await audit(page, 'accueil-parametres-du-site'); console.log('param site → vue :', await page.evaluate(() => document.querySelector('.view-tab.is-on')?.textContent)); await page.locator('.logo').click(); await page.waitForTimeout(700); });
await T('supprimer', async () => { await page.locator('[aria-label="Supprimer le site…"]').first().click(); await page.waitForTimeout(500); await audit(page, 'accueil-supprimer'); await esc(); console.log('supprimer après Échap :', (await openFloats(page)).join(',') || 'fermé'); });
// Réglages de l'application
await T('réglages', async () => {
  await page.locator('.view-tab', { hasText: 'Réglages' }).click(); await page.waitForTimeout(900);
  const navs = await page.$$eval('.settings-nav button', (b) => b.map((x) => x.textContent.trim()));
  console.log('réglages :', navs.join(' | '));
  for (let i = 0; i < navs.length; i++) {
    await page.locator('.settings-nav button').nth(i).click(); await page.waitForTimeout(600);
    await scrollAudit(page, '.settings-nav ~ *', 'reglages-' + i, { max: 5 });
  }
});
await page.locator('.view-tab', { hasText: 'Sites' }).click().catch(() => {}); await page.waitForTimeout(400);
await openSite(page);
// Localisation
await T('loc', async () => {
  await page.locator('.view-tab', { hasText: 'Localisation' }).click(); await page.waitForTimeout(1000);
  await audit(page, 'loc-base');
  await pop(page, 'loc-ajouter-langue', page.locator('summary', { hasText: 'Ajouter une langue' }));
  await page.locator('summary', { hasText: 'Ajouter une langue' }).click(); await page.waitForTimeout(200);
  await page.locator('.menu-item', { hasText: 'English' }).first().click(); await page.waitForTimeout(1500);
  await audit(page, 'loc-avec-anglais');
  console.log('loc après ajout :', (await inventory(page)).filter((x) => !/view-tab|logo|site-menu-btn|split-|profile-btn|menu-item/.test(x)).slice(0, 30).join(' ; ').slice(0, 900));
});
// Plan du site
await T('plan', async () => {
  await page.locator('.view-tab', { hasText: 'Plan du site' }).click(); await page.waitForTimeout(1500);
  await page.locator('.map-node').first().click(); await page.waitForTimeout(600);
  await audit(page, 'plan-noeud');
  await pop(page, 'plan-ctx-noeud', '.map-node', { right: true });
  for (const f of ['Parties communes', 'Autres sites', 'Médias et réseaux']) { await page.locator('.map-filters button', { hasText: f }).click(); await page.waitForTimeout(500); }
  await audit(page, 'plan-filtres');
  await page.locator('.map-expand').first().click(); await page.waitForTimeout(600); await audit(page, 'plan-deplie');
});
// Éditeur : palette de commandes, aide
await page.locator('.view-tab', { hasText: 'Éditeur' }).click(); await page.waitForTimeout(800);
await T('palette', async () => { await page.keyboard.press('Control+k'); await page.waitForTimeout(500); await audit(page, 'editeur-palette'); console.log('palette :', (await openFloats(page)).join(',') || 'absente'); await esc(); console.log('palette après Échap :', (await openFloats(page)).join(',') || 'fermé'); });
await T('publier', async () => { await page.locator('.topbar .btn--primary', { hasText: 'Publier' }).click(); await page.waitForTimeout(900); await audit(page, 'editeur-publier'); console.log('publier :', (await openFloats(page)).join(',') || 'absent'); await esc(); console.log('publier après Échap :', (await openFloats(page)).join(',') || 'fermé'); });
save(OUT + '/audit.json');
fs.writeFileSync(OUT + '/echap-s7.json', JSON.stringify(escapes));
console.log('erreurs :', errors.length, errors.slice(0, 4).join(' || ').slice(0, 600));
await browser.close();
