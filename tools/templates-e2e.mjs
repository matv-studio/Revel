#!/usr/bin/env node
// Essais de bout en bout d'un site créé d'après un modèle (D-128) : interface servie (dist de
// Trunk) avec l'API Tauri simulée, reliée par `revel canvas-serve --bridge` à un site neuf du
// modèle Boutique (`revel new … --template shop`).
//   - CMS : galerie d'images (vignettes, retrait, ajout de plusieurs médias, ordre au glisser,
//     annulation), largeur du volet de la fiche, modèle des pages ouvert dans l'éditeur ;
//   - Médias : images en ligne du modèle signalées, copiées dans le site (avec le réseau), annulé.
//
// Usage : node tools/templates-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT DOSSIER_DU_SITE [DOSSIER_CAPTURES]
// `REVEL_E2E_OFFLINE=1` : la copie des images (téléchargement) n'est pas essayée.

import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const { chromium } = createRequire(import.meta.url)('playwright');
const [ui, bridge, siteDir, shots] = process.argv.slice(2);
if (!ui || !bridge || !siteDir) {
  console.error('usage : templates-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT DOSSIER_DU_SITE [DOSSIER_CAPTURES]');
  process.exit(2);
}
const mock = fs.readFileSync(new URL('../apps/revel-ui/dev/mock-tauri.js', import.meta.url), 'utf8');
if (shots) fs.mkdirSync(shots, { recursive: true });
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`${ok ? 'ok ' : 'ÉCHEC'} ${name}${detail ? ' — ' + detail : ''}`);
};
const invoke = async (cmd, args = {}) => {
  const r = await fetch(bridge.replace(/\/$/, '') + '/__revel/invoke/' + cmd, { method: 'POST', body: JSON.stringify(args) });
  return r.json();
};
const products = path.join(siteDir, 'content/products');
const entryFile = (name) => path.join(products, name + '.md');
const galleryOf = (text) => {
  const m = text.match(/^gallery:\n((?:  - .*\n)+)/m);
  return m ? m[1].trim().split('\n').map((l) => l.replace(/^\s*- /, '').replace(/^['"]|['"]$/g, '')) : [];
};

const browser = await chromium.launch();
let page;
try {
  page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.addInitScript(mock);
  await page.goto(`${ui.replace(/\/$/, '')}/#lang=fr&theme=light&bridge=${encodeURIComponent(bridge)}`);
  const until = async (pred, what) => {
    for (let i = 0; i < 80; i++) { if (await pred()) return true; await page.waitForTimeout(100); }
    if (shots) await page.screenshot({ path: `${shots}/echec.png` });
    throw new Error(what);
  };
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame');
  const doc0 = JSON.stringify((await invoke('edit_open')).site);

  // ---------------------------------------------------------------- CMS : galerie d'images
  await page.click('.view-tab[data-view="view-cms"]');
  await page.click('.cms-col[data-collection="products"]');
  await page.waitForSelector('.cms-table tbody tr[data-entry]');
  const file = await page.$eval('.cms-table tbody tr[data-entry]', (r) => r.dataset.entry);
  const original = fs.readFileSync(entryFile(file), 'utf8');
  const g0 = galleryOf(original);
  await page.click(`.cms-table tbody tr[data-entry="${file}"]`);
  const gal = '.cms-item-body [data-field="gallery"] .cms-gallery';
  await page.waitForSelector(gal);
  const thumbs = async () => page.$$eval(`${gal} .cms-gallery__item img`, (i) => i.map((x) => x.getAttribute('src')));
  await until(async () => (await thumbs()).length === g0.length, 'vignettes de la galerie');
  check('CMS : galerie en vignettes (pas de pastilles de chemin)', JSON.stringify(await thumbs()) === JSON.stringify(g0)
    && !(await page.$('.cms-item-body [data-field="gallery"] .chip--tag')), `${g0.length} images`);
  if (shots) await page.screenshot({ path: `${shots}/cms-galerie.png` });
  // Retrait (✕ au survol).
  await page.hover(`${gal} .cms-gallery__item >> nth=0`);
  await page.click(`${gal} .cms-gallery__item >> nth=0 >> .cms-gallery__remove`);
  await until(() => galleryOf(fs.readFileSync(entryFile(file), 'utf8')).join() === g0.slice(1).join(), 'image retirée');
  check('CMS : image retirée de la galerie', true);
  // Ajout de plusieurs médias à la fois (fenêtre des médias en choix multiple) : importés ici.
  await page.click(`${gal} .cms-gallery__add`);
  await page.waitForSelector('.picker .picker-add[disabled]');
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64');
  const [chooser] = await Promise.all([
    page.waitForEvent('filechooser'),
    page.click('.picker-foot .btn >> nth=0'),
  ]);
  await chooser.setFiles([
    { name: 'essai-galerie-a.png', mimeType: 'image/png', buffer: png },
    { name: 'essai-galerie-b.png', mimeType: 'image/png', buffer: png },
  ]);
  await page.waitForSelector('.picker .picker-tile.is-picked >> nth=1');
  const order = await page.$$eval('.picker .picker-tile.is-picked .picker-order', (o) => o.map((x) => x.textContent));
  await page.click('.picker .picker-add');
  await page.waitForFunction(() => !document.querySelector('.picker'));
  const added = () => galleryOf(fs.readFileSync(entryFile(file), 'utf8'));
  await until(() => added().length === g0.length + 1, 'images ajoutées');
  check('CMS : deux images ajoutées en une fois (choix multiple, ordre des clics)', order.join() === '1,2'
    && added().slice(-2).every((u) => /essai-galerie-[ab]\.png$/.test(u)), added().slice(-2).join(', '));
  // Ordre : glisser la dernière vignette en premier.
  const before = added();
  await until(async () => (await page.$$(`${gal} .cms-gallery__item`)).length === before.length, 'vignettes ajoutées');
  await page.waitForTimeout(300);
  const items = page.locator(`${gal} .cms-gallery__item`);
  await items.last().dragTo(items.first());
  await until(() => added()[0] === before[before.length - 1], `ordre changé (${before.map((u) => u.split('/').pop().slice(0, 14)).join(' · ')} → ${added().map((u) => u.split('/').pop().slice(0, 14)).join(' · ')})`);
  check('CMS : ordre changé en glissant une vignette', true, added().map((u) => u.split('/').pop().slice(0, 18)).join(' · '));
  // Clavier : Alt+flèche droite sur la première vignette (galerie relue d'abord).
  await until(async () => (await thumbs())[0]?.endsWith(added()[0].split('/').pop()), 'galerie relue');
  await page.waitForTimeout(300);
  await page.focus(`${gal} .cms-gallery__item >> nth=0 >> .cms-gallery__thumb`);
  const b2 = added();
  await page.keyboard.press('Alt+ArrowRight');
  await until(() => added()[1] === b2[0], 'ordre au clavier');
  check('CMS : ordre changé au clavier (Alt+flèche)', true);
  for (let i = 0; i < 4; i++) {
    await page.click('.cms-head .icon-btn[title^="Annuler"]');
    await page.waitForTimeout(250);
  }
  await until(() => fs.readFileSync(entryFile(file), 'utf8') === original, 'annulation de la galerie');
  check('CMS : galerie annulée, fichier identique', true);

  // ---------------------------------------------------------------- CMS : colonnes du tableau
  const pagesYml = path.join(siteDir, '.pages.yml');
  const yml0 = fs.readFileSync(pagesYml, 'utf8');
  const heads = () => page.$$eval('.cms-table thead th[data-col]', (t) => t.map((x) => x.dataset.col));
  const h0 = await heads();
  await page.hover('.cms-item-body [data-field="description"]');
  await page.click('.cms-item-body [data-field="description"] .cms-pin');
  await until(async () => (await heads()).includes('description'), 'colonne épinglée');
  const pinned = await page.$eval('.cms-item-body [data-field="description"] .cms-pin', (b) => b.classList.contains('is-on'));
  check('CMS : épingle d\'un champ de la fiche, colonne ajoutée au tableau (schéma)', pinned
    && /view:[\s\S]*fields:[\s\S]*- description/.test(fs.readFileSync(pagesYml, 'utf8')), (await heads()).join(', '));
  await page.click('.cms-cols-btn');
  const menuItems = await page.$$eval(".ctx-menu [role=\"menuitem\"]", (b) => b.map((x) => x.textContent.trim()));
  await page.click('.ctx-menu [role="menuitem"]:has-text("Prix")');
  await until(async () => !(await heads()).includes('price'), 'colonne masquée par le menu');
  check('CMS : menu des colonnes (montrer, masquer)', menuItems.length >= 4, menuItems.join(' | '));
  await page.click('.cms-table thead th[data-col="description"]', { button: 'right' });
  await page.click('.ctx-menu [role="menuitem"] >> nth=0');
  await until(async () => !(await heads()).includes('description'), 'colonne masquée par le clic droit');
  check('CMS : clic droit sur une colonne, « Masquer la colonne »', true);
  for (let i = 0; i < 3; i++) {
    await page.click('.cms-head .icon-btn[title^="Annuler"]');
    await page.waitForTimeout(250);
  }
  await until(() => fs.readFileSync(pagesYml, 'utf8') === yml0, 'colonnes annulées');
  check('CMS : colonnes annulées, schéma identique', JSON.stringify(await heads()) === JSON.stringify(h0));
  // Largeur d'une colonne à la poignée ; double-clic : automatique.
  const th = page.locator('.cms-table thead th[data-col="category"]');
  const tw0 = (await th.boundingBox()).width;
  const rh = await page.locator('.cms-table thead th[data-col="category"] .th-resize').boundingBox();
  await page.mouse.move(rh.x + rh.width / 2, rh.y + rh.height / 2);
  await page.mouse.down();
  await page.mouse.move(rh.x + 120, rh.y + rh.height / 2, { steps: 5 });
  await page.mouse.up();
  const tw1 = (await th.boundingBox()).width;
  await page.dblclick('.cms-table thead th[data-col="category"] .th-resize');
  await page.click('.cms-cols-btn');
  await page.click('.ctx-menu [role="menuitem"]:has-text("Largeurs automatiques")');
  check('CMS : largeur d\'une colonne à la poignée, largeurs automatiques', tw1 - tw0 > 100
    && !(await page.$('.cms-table.is-sized')), `${Math.round(tw0)} → ${Math.round(tw1)}`);

  // ---------------------------------------------------------------- CMS : largeur du volet
  const w0 = await page.$eval('.cms-item', (e) => e.getBoundingClientRect().width);
  const handle = await page.$('.cms-resize');
  const hb = await handle.boundingBox();
  await page.mouse.move(hb.x + hb.width / 2, hb.y + 200);
  await page.mouse.down();
  await page.mouse.move(hb.x - 180, hb.y + 200, { steps: 6 });
  await page.mouse.up();
  const w1 = await page.$eval('.cms-item', (e) => e.getBoundingClientRect().width);
  await page.dblclick('.cms-resize');
  const w2 = await page.$eval('.cms-item', (e) => e.getBoundingClientRect().width);
  check('CMS : volet de la fiche élargi à la poignée, double-clic : largeur par défaut',
    Math.abs(w1 - w0 - (hb.width / 2 + 180)) < 3 && Math.abs(w2 - 420) < 2, `${w0} → ${w1} → ${w2}`);
  if (shots) {
    await page.mouse.move(hb.x + hb.width / 2, hb.y + 200);
    await page.mouse.down();
    await page.mouse.move(hb.x - 160, hb.y + 200, { steps: 4 });
    await page.mouse.up();
    await page.screenshot({ path: `${shots}/cms-volet-large.png` });
    await page.dblclick('.cms-resize');
  }

  // ---------------------------------------------------------------- CMS : modèle des pages
  await page.click('.cms-item-head .icon-btn');
  await page.click('.cms-template-btn');
  await page.waitForFunction(() => !!document.querySelector('iframe.canvas-frame') && !document.querySelector('.cms'));
  const shown = await page.waitForFunction(() => document.querySelector('.editor-page-chip, .toolbar-page, .page-menu-btn')?.textContent || location.hash, null, { timeout: 5000 }).then((h) => h.jsonValue()).catch(() => '');
  const frame = () => page.frames().find((f) => f.url().includes('rv='));
  await until(async () => { const f = frame(); return !!f && /\/(produits|products|boutique|shop)\/[^/]+\//.test(f.url()); }, 'page d\'un produit dans l\'éditeur');
  check('CMS : « Modèle des pages » ouvre une page de la collection dans l\'éditeur', true, frame().url().replace(/^.*?\/\/[^/]+/, '').slice(0, 60));
  if (shots) await page.screenshot({ path: `${shots}/modele-editeur.png` });
  // Bouton « Ajouter au panier » : réglages Produit et Prix (reliés aux champs du produit), pas
  // d'attributs bruts.
  await frame().click('[data-revel-cart-add]');
  await page.click('.dock-tab[data-panel="styles"]').catch(() => {});
  await page.waitForSelector('.insp-group[data-role="cart-add"] [data-attr="data-revel-price"]');
  await page.click('.advanced-settings > summary').catch(() => {});
  const rawAttrs = await page.$$eval('.insp-attrs .attr-row', (n) => n.map((x) => x.dataset.attr));
  const cartFields = await page.$$eval('.insp-group[data-role="cart-add"] .field-head .label', (n) => n.map((x) => x.textContent.trim()));
  check('panier : bouton « Ajouter au panier », produit et prix réglés à part (pas d\'attribut brut)',
    cartFields.join() === 'Produit,Prix' && !rawAttrs.some((a) => a.startsWith('data-revel-')), `${cartFields.join(', ')} | ${rawAttrs.join(', ')}`);
  if (shots) {
    await page.locator('.insp-group[data-role="cart-add"]').scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${shots}/panier-reglages.png` });
  }
  void shown;
  // Clic droit sur une collection : entrée du menu.
  await page.click('.view-tab[data-view="view-cms"]');
  await page.click('.cms-col[data-collection="products"]', { button: 'right' });
  const menu = await page.$$eval('.ctx-menu [role="menuitem"], .ctx-menu button', (b) => b.map((x) => x.textContent.trim()));
  check('CMS : clic droit sur une collection, « Modifier le modèle des pages »', menu.some((t) => /Modifier le modèle des pages/.test(t)), menu.join(' | '));
  await page.keyboard.press('Escape');

  // ---------------------------------------------------------------- Médias : images en ligne
  await page.click('.view-tab[data-view="view-editor"]');
  await page.waitForSelector('iframe.canvas-frame');
  await page.click('.dock-tab[data-panel="assets"]');
  await page.waitForSelector('.media-online');
  const online = await invoke('media_online');
  const label = (await page.textContent('.media-online__text')).trim();
  check('médias : images en ligne du modèle signalées sous la recherche', online.length > 3 && label.startsWith(String(online.length)) && await page.$eval('.media .panel-search, .media .search', (el) => el.nextElementSibling?.classList.contains('media-online')).catch(() => false), `${label}`);
  if (shots) await page.screenshot({ path: `${shots}/medias-en-ligne.png` });
  if (!process.env.REVEL_E2E_OFFLINE) {
    await page.click('.media-online__copy');
    await page.waitForFunction(() => !document.querySelector('.media-online'), null, { timeout: 120000 });
    const left = await invoke('media_online');
    const photos = fs.existsSync(path.join(siteDir, 'media/photos')) ? fs.readdirSync(path.join(siteDir, 'media/photos')) : [];
    const entry = fs.readFileSync(entryFile(file), 'utf8');
    check('médias : images téléchargées dans le site, bandeau retiré, adresses remplacées (pages, contenus)', left.length === 0 && photos.length >= online.length
      && !entry.includes('images.unsplash.com') && entry.includes('/media/photos/'), `${photos.length} fichiers, ${left.length} restantes`);
    if (shots) await page.screenshot({ path: `${shots}/medias-copiees.png` });
    await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
    await page.keyboard.press('Control+z');
    await until(async () => (await invoke('media_online')).length === online.length, 'annulation de la copie');
    check('médias : copie annulée, adresses en ligne rétablies', fs.readFileSync(entryFile(file), 'utf8') === original);
  }
  // Fichiers de l'essai retirés, document comparé.
  for (const n of ['essai-galerie-a.png', 'essai-galerie-b.png']) {
    for (const d of ['media', 'media/photos']) fs.rmSync(path.join(siteDir, d, n), { force: true });
  }
  const sample = (await invoke('cms_collections')).find((c) => c.id === 'products');
  check('document et contenus identiques après les essais', JSON.stringify((await invoke('edit_open')).site.collections) === JSON.stringify(JSON.parse(doc0).collections)
    && sample.entries.length >= 3);
  check('aucune erreur JavaScript', errors.length === 0, errors.slice(0, 3).join(' | '));
} catch (e) {
  check('déroulé', false, e.message);
  if (shots && page) await page.screenshot({ path: `${shots}/echec.png` }).catch(() => {});
} finally {
  await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
