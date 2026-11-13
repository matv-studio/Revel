import { start, openSite, audit, save, frameOf, scrollAudit, pop, escapes, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
await openSite(page);
let f = frameOf(page);
const select = async (s, pos = { x: 6, y: 6 }) => { await f.click(s, { position: pos }); await page.waitForTimeout(700); };
await select('section.hero', { x: 6, y: 300 });
await page.evaluate(() => document.querySelectorAll('.styles details').forEach((d) => { if (!d.open) d.querySelector('summary').click(); }));
await page.waitForTimeout(500);
// Fenêtres flottantes des contrôles de style.
const vis = (s) => page.locator(s).first();
for (const [name, s] of [['styles-pop-plus-affichage', '.styles .seg-more'], ['styles-pop-unite', '.styles .nbox-unit'], ['styles-pop-variable', '.styles .nbox-chip'], ['styles-pop-couleur', '.styles .color-swatch'], ['styles-pop-icone', '.styles .icon-btn.pop-trigger']]) {
  await page.locator(s).first().scrollIntoViewIfNeeded().catch(() => {});
  await pop(page, name, s);
  if (!(await page.$('.styles .style-section'))) { await select('section.hero', { x: 6, y: 300 }); }
}
// Barre d'outils de l'éditeur.
const sums = await page.$$('.editor-toolbar summary');
for (let i = 0; i < sums.length; i++) {
  const label = await sums[i].evaluate((e) => (e.getAttribute('aria-label') || e.getAttribute('title') || e.textContent || '').trim().toLowerCase().replace(/[^a-zà-ÿ]+/g, '-').slice(0, 24));
  await pop(page, 'outils-' + label, page.locator('.editor-toolbar summary').nth(i), { close: null });
  await page.locator('.editor-toolbar summary').nth(i).click().catch(() => {}); await page.waitForTimeout(200);
}
// Barre supérieure.
try { await pop(page, 'barre-menu-du-site', '.site-menu-btn'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'barre-options-apercu', '.split-more'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'barre-profil', '.profile-btn'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'barre-publier', '.topbar .btn--primary'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
if (await page.$('.overlay')) { await page.keyboard.press('Escape'); await page.waitForTimeout(300); }
// Menus contextuels.
f = frameOf(page);
try { await pop(page, 'ctx-canevas', 'section.hero h1', { right: true, frame: f, pos: { x: 5, y: 5 } }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'ctx-calque', '.layer.is-selected, .layer', { right: true }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'ctx-onglet', '.dock-tab[data-panel="layers"]', { right: true }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'ctx-page', '.page-row', { right: true }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'panneau-options', '.dock-group .icon-btn[aria-label="Options du panneau"], .dock-group .icon-btn[title="Options du panneau"]'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
// Pages : réglages de la page, nouvelle page, nouveau dossier.
await page.hover('.page-row'); await page.waitForTimeout(200);
try { await pop(page, 'page-parametres', '.page-row .row-action', { close: null, wait: 900 }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
console.log('volet :', await page.evaluate(() => { const t = document.querySelector('.page-settings-title'); let p = t; const out = []; while (p && out.length < 5) { out.push(p.tagName.toLowerCase() + '.' + [...p.classList].join('.')); p = p.parentElement; } return out.join(' < ') + ' | boutons : ' + [...(t.closest('[class*=page-settings]') || t.parentElement.parentElement).querySelectorAll('button')].slice(0, 6).map((b) => b.className + '«' + (b.getAttribute('aria-label') || b.textContent.trim()).slice(0, 20) + '»').join(' ; '); }));
try { await scrollAudit(page, '.page-settings-title', 'page-parametres-defile', { max: 8 }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
await page.keyboard.press('Escape'); await page.waitForTimeout(400);
if (await page.$('.page-settings-title')) { console.log('ÉCHAP NE FERME PAS le volet des paramètres de la page'); await page.locator('.page-settings-title').locator('xpath=..').locator('button').first().click().catch(() => {}); await page.waitForTimeout(400); }
if (await page.$('.page-settings-title')) { console.log('volet toujours ouvert'); }
try { await pop(page, 'page-nouvelle', '.icon-btn[aria-label="Nouvelle page"], .icon-btn[title="Nouvelle page"]'); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
// Composants, médias.
try { await page.click('.dock-tab[data-panel="components"]'); await page.waitForTimeout(400); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'ctx-composant', '.dock-panel.is-on[data-panel="components"] .list-row', { right: true }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await page.click('.dock-tab[data-panel="assets"]'); await page.waitForTimeout(600); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await pop(page, 'ctx-media', '.asset', { right: true }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await scrollAudit(page, '.media .asset', 'medias-defile', { max: 3 }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
// Ajouter.
try { await page.click('.dock-tab[data-panel="insert"]'); await page.waitForTimeout(400); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
try { await scrollAudit(page, '.insert', 'ajouter-defile', { max: 8 }); } catch (e) { console.log("ÉTAPE EN ÉCHEC : " + e.message.split("\n")[0].slice(0, 140)); }
// Panneaux fermés par défaut : Variables, CMS.
for (const name of ['Variables', 'CMS']) {
  await page.locator('.editor-toolbar summary[aria-label="Panneaux"], .editor-toolbar summary[title="Panneaux"]').first().click(); await page.waitForTimeout(250);
  await page.locator('.menu-item--check', { hasText: name }).first().click().catch((e) => console.log(name + ' : ' + e.message.slice(0, 60))); await page.waitForTimeout(600);
  await page.keyboard.press('Escape'); await page.waitForTimeout(200);
  await audit(page, 'panneau-' + name.toLowerCase());
}
console.log('onglets :', (await page.$$eval('.dock-tab', (t) => t.map((x) => x.getAttribute('data-panel')))).join(','));
save(OUT + '/audit.json');
fs.writeFileSync(OUT + '/echap-s3.json', JSON.stringify(escapes));
console.log('erreurs :', errors.length, errors.slice(0, 5).join(' || '));
await browser.close();
