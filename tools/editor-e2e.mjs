#!/usr/bin/env node
// Essais de bout en bout de l'éditeur (étape 2.3) : interface servie (dist de Trunk) avec l'API
// Tauri simulée, reliée à un vrai cœur par `revel canvas-serve --bridge` (copie du site : les
// essais le modifient puis annulent).
//
// Usage : node tools/editor-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES]
// Code de sortie 1 si un essai échoue.

import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const { chromium } = createRequire(import.meta.url)('playwright');
const [ui, bridge, shots, siteDir] = process.argv.slice(2);
if (!ui || !bridge) {
  console.error('usage : editor-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES] [DOSSIER_DU_SITE]');
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

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error') console.log('console', m.text().slice(0, 300)); });
  await page.addInitScript(mock);
  await page.goto(`${ui.replace(/\/$/, '')}/#lang=fr&theme=dark&bridge=${encodeURIComponent(bridge)}`);
  // Nouveau site : dossier proposé d'après le nom, sans fenêtre du système.
  await page.click('.page-head .btn--primary');
  await page.waitForSelector('.dialog--form input');
  await page.keyboard.type('Atelier Lumière');
  await page.waitForFunction(() => /atelier-lumiere$/.test(document.querySelector('.path-value')?.textContent || ''));
  check('nouveau site : dossier proposé d\'après le nom', true, await page.textContent('.path-value'));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.querySelector('.dialog--form'));
  // Un clic sur un site l'ouvre dans l'éditeur.
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame');
  const frameOf = () => page.frames().find((f) => f.url().includes('rv='));
  const waitFrame = async (pred, what, arg) => {
    for (let i = 0; i < 150; i++) {
      const f = frameOf();
      if (f) {
        try { if (await f.evaluate(pred, arg)) return f; } catch { /* rechargement en cours */ }
      }
      await page.waitForTimeout(100);
    }
    throw new Error('canevas : ' + what);
  };
  let f = await waitFrame(() => document.readyState === 'complete' && !!document.querySelector('[data-rv]'), 'chargement');
  check('canevas chargé dans l\'éditeur', true, f.url());
  if (shots) await page.screenshot({ path: `${shots}/editeur-accueil.png` });

  // Sélection dans le canevas -> barre d'état.
  const h1 = page.frameLocator('iframe.canvas-frame').locator('h1').first();
  await h1.click();
  await page.waitForFunction(() => /(Page|Gabarit|Composant) /.test(document.querySelector('.editor-status')?.textContent || ''));
  const status = await page.textContent('.editor-status');
  check('clic dans le canevas : élément et propriétaire affichés', /h1|span|strong/.test(status) && /(Page|Gabarit|Composant) /.test(status), status.trim().replace(/\s+/g, ' '));

  // Points de rupture : largeur du cadre.
  const bps = await page.$$('.editor-toolbar .segmented button');
  await bps[bps.length - 1].click();
  await page.waitForTimeout(300);
  const w = await page.$eval('iframe.canvas-frame', (e) => e.style.width);
  check('point de rupture le plus étroit : largeur du cadre', w === '600px', w);
  if (shots) await page.screenshot({ path: `${shots}/editeur-600.png` });
  await bps[0].click();

  // Zoom.
  await page.selectOption('.editor-zoom select', '0.5');
  await page.waitForTimeout(200);
  const tr = await page.$eval('iframe.canvas-frame', (e) => e.style.transform);
  check('zoom 50 %', tr === 'scale(0.5)', tr);
  await page.selectOption('.editor-zoom select', '');

  // Modification de style : feuille rechargée sans recharger la page.
  const doc = await invoke('edit_open');
  const selector = await f.evaluate(() => {
    const h = document.querySelector('h1');
    return h.className ? '.' + h.className.trim().split(/\s+/)[0] : 'h1';
  });
  const rule = doc.site.stylesheet.items.find((i) => i.item === 'rule' && i.selector === selector && !i.media);
  check('règle de style du titre trouvée', !!rule, `${selector} (${rule && rule.id})`);
  f = frameOf();
  await f.evaluate(() => { window.__mark = 1; });
  const before = await f.evaluate(() => getComputedStyle(document.querySelector('h1')).color);
  await page.evaluate((cmd) => window.__revelTest.apply(JSON.stringify(cmd), false),
    { cmd: 'set_decl', rule: rule.id, prop: 'color', value: 'rgb(255, 0, 0)' });
  f = await waitFrame(() => getComputedStyle(document.querySelector('h1')).color === 'rgb(255, 0, 0)', 'couleur modifiée');
  check('style modifié : appliqué en direct, sans rechargement', await f.evaluate(() => window.__mark === 1), before + ' → rgb(255, 0, 0)');
  const canUndo = await page.$eval('.editor-toolbar .btn-undo', (b) => !b.disabled);
  check('« Annuler » disponible', canUndo);

  // Annuler au clavier (focus dans l'interface).
  await page.click('.editor-status');
  await page.keyboard.press('Control+z');
  f = await waitFrame((c) => getComputedStyle(document.querySelector('h1')).color !== 'rgb(255, 0, 0)', 'couleur rétablie');
  const after = await f.evaluate(() => getComputedStyle(document.querySelector('h1')).color);
  check('Ctrl+Z : style d\'origine, toujours sans rechargement', after === before && (await f.evaluate(() => window.__mark === 1)), after);

  // Modification de structure : page rechargée, sélection conservée.
  const h1id = await f.evaluate(() => document.querySelector('h1').getAttribute('data-rv'));
  const owner = await (async () => {
    for (const kind of ['page', 'layout', 'component']) {
      const list = kind === 'page' ? doc.site.pages : kind === 'layout' ? doc.site.layouts : doc.site.components;
      for (const o of list) if (JSON.stringify(o.body).includes(`"${h1id}"`)) return { kind, id: o.id };
    }
    return null;
  })();
  await page.evaluate((cmd) => window.__revelTest.apply(JSON.stringify(cmd), false),
    { cmd: 'set_attr', at: { owner, node: h1id }, name: 'data-essai', value: 'oui' });
  f = await waitFrame(() => document.querySelector('h1')?.getAttribute('data-essai') === 'oui' && window.__mark === undefined, 'rechargement');
  check('attribut modifié : page rechargée avec la modification', true);
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame(() => document.readyState === 'complete' && document.querySelector('h1') && !document.querySelector('h1').hasAttribute('data-essai'), 'annulation');
  const disk = await invoke('edit_open');
  check('annulé : document identique à l\'ouverture', JSON.stringify(disk.site) === JSON.stringify(doc.site));

  // ------------------------------------------------------------------ calques (étape 2.4)
  // Sélection dans le canevas -> ligne des calques sélectionnée et visible.
  f = frameOf();
  await page.frameLocator('iframe.canvas-frame').locator('h1').first().click();
  await page.waitForSelector('.layers .layer.is-selected');
  await page.waitForTimeout(300);
  const selLabel = (await page.textContent('.layers .layer.is-selected')).trim();
  const inView = await page.$eval('.layers .layer.is-selected', (el) => {
    const r = el.getBoundingClientRect();
    const t = el.closest('.layers-tree').getBoundingClientRect();
    return r.top >= t.top - 1 && r.bottom <= t.bottom + 1;
  });
  check('calques : élément cliqué sélectionné et visible dans l\'arbre', inView, selLabel);

  // Ligne cliquée -> sélection dans le canevas (barre d'état).
  await page.click('.layers .layer:has-text("section.hero")');
  await page.waitForFunction(() => document.querySelector('.editor-status')?.textContent.includes('section.hero'));
  check('calques : ligne cliquée sélectionnée dans le canevas', true);

  const docBefore = await invoke('edit_open');
  const bodyOf = (site) => JSON.stringify(site.pages.find((p) => p.id === 'index').body);
  // Dupliquer (bouton), puis annuler.
  await page.click('.layers .layers-foot .icon-btn >> nth=5');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 2, 'section dupliquée');
  const dup = await invoke('edit_open');
  check('dupliquer : section copiée (nouveaux identifiants), sélectionnée', bodyOf(dup.site).length > bodyOf(docBefore.site).length
    && (await page.textContent('.editor-status')).includes('section.hero'));
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 1, 'annulation de la copie');

  // Descendre (bouton) : la section passe après sa voisine.
  await page.click('.layers .layer:has-text("section.hero")');
  const orderBefore = await f.evaluate(() => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(','));
  await page.click('.layers .layers-foot .icon-btn >> nth=3');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') !== ob, 'déplacement', orderBefore);
  check('descendre : ordre modifié dans le canevas', true);
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') === ob, 'annulation du déplacement', orderBefore);

  // Glisser-déposer : section.hero (repliée) déposée après la section suivante.
  await page.click('.layers .layer:has-text("section.hero") .layer-toggle');
  await page.waitForTimeout(150);
  const info = await page.$$eval('.layers .layer', (rs) => rs.map((e) => [e.textContent.trim(), parseInt(e.dataset.depth, 10)]));
  const si = info.findIndex(([t]) => t === 'section.hero');
  const di = info.findIndex(([t, pad], i) => i > si && t.startsWith('section') && pad === info[si][1]);
  if (si >= 0 && di >= 0) {
    const rowsL = page.locator('.layers .layer');
    // Source et cible visibles ensemble (sinon le défilement pendant le glisser change la source).
    await rowsL.nth(si).evaluate((e) => e.scrollIntoView({ block: 'start' }));
    const box = await rowsL.nth(di).boundingBox();
    await rowsL.nth(si).dragTo(rowsL.nth(di), { targetPosition: { x: 40, y: box.height - 3 } });
    f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') !== ob, 'glisser-déposer', orderBefore);
    check('glisser-déposer : section déplacée', true);
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') === ob, 'annulation du glisser-déposer', orderBefore);
  } else {
    check('glisser-déposer : lignes trouvées', false);
  }

  // Supprimer (touche Suppr dans les calques), puis annuler.
  await page.click('.layers .layer:has-text("section.hero")');
  await page.keyboard.press('Delete');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 0, 'suppression');
  check('Suppr : section retirée, plus de sélection', !(await page.$('.layers .layer.is-selected')));
  await page.keyboard.press('Control+z');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 1, 'annulation de la suppression');
  const docAfter = await invoke('edit_open');
  check('calques : tout annulé, document identique', JSON.stringify(docAfter.site) === JSON.stringify(docBefore.site));
  if (shots) await page.screenshot({ path: `${shots}/editeur-calques.png` });

  // ------------------------------------------------------------------ raccourcis dans le canevas (étape 2.9)
  {
    const canvas = page.frameLocator('iframe.canvas-frame');
    const baseK = await invoke('edit_open');
    // Le clic atteint l'élément le plus profond : on compte les éléments repérés.
    const marked = () => document.readyState === 'complete' && window.__revelSlot ? document.querySelectorAll('[data-rv]').length : -1;
    const count = await frameOf().evaluate(marked);
    await canvas.locator('h1').first().click();
    await page.waitForSelector('.layers .layer.is-selected');
    await page.keyboard.press('Delete');
    f = await waitFrame((n) => document.readyState === 'complete' && window.__revelSlot && document.querySelectorAll('[data-rv]').length < n, 'Suppr dans le canevas', count);
    check('Suppr dans le canevas : élément retiré', true);
    await page.waitForFunction(() => /Enregistré à \d\d:\d\d/.test(document.querySelector('.save-state')?.textContent || ''));
    check('état d\'enregistrement affiché', true, (await page.textContent('.save-state')).trim());
    await canvas.locator('main').first().click();
    await page.keyboard.press('Control+z');
    f = await waitFrame((n) => document.readyState === 'complete' && window.__revelSlot && document.querySelectorAll('[data-rv]').length === n, 'Ctrl+Z dans le canevas', count);
    check('Ctrl+Z dans le canevas : élément rétabli', true);
    await canvas.locator('h1').first().click();
    await page.keyboard.press('Control+d');
    f = await waitFrame((n) => document.readyState === 'complete' && window.__revelSlot && document.querySelectorAll('[data-rv]').length > n, 'Ctrl+D dans le canevas', count);
    check('Ctrl+D dans le canevas : élément dupliqué', true);
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame((n) => document.readyState === 'complete' && window.__revelSlot && document.querySelectorAll('[data-rv]').length === n, 'annulation de Ctrl+D', count);
    await canvas.locator('h1').first().click();
    await page.waitForSelector('.layers .layer.is-selected');
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('.layers .layer.is-selected'));
    check('Échap dans le canevas : sélection retirée', true);
    const docK = await invoke('edit_open');
    check('raccourcis : tout annulé, document identique', JSON.stringify(docK.site) === JSON.stringify(baseK.site));
  }


  // ------------------------------------------------------------------ réglages et ajout (étape 2.5)
  const base25 = await invoke('edit_open');
  const nodeById = (site, id) => {
    let found = null;
    const walk = (nodes) => { for (const n of nodes || []) { if (found) return; if ((n.id || (n.image && n.image.id)) === id) { found = n; return; } walk(n.children); walk(n.then); walk(n.otherwise); walk(n.empty); } };
    for (const o of [...site.pages, ...site.layouts, ...site.components]) walk(o.body);
    return found;
  };
  await page.click('.layers .layer:has-text("section.hero")');
  await page.click('.dock-tab[data-panel="settings"]');
  await page.waitForFunction(() => document.querySelector('.dock-panel[data-panel="settings"] input')?.value === 'section');
  check('réglages : balise de l\'élément sélectionné', true);
  const secId = await f.evaluate(() => document.querySelector('section.hero').getAttribute('data-rv'));
  // Identifiant (Entrée).
  const idInput = page.locator('.dock-panel[data-panel="settings"] .field >> nth=1').locator('input');
  await idInput.fill('essai-id');
  await idInput.press('Enter');
  f = await waitFrame(() => !!document.querySelector('section#essai-id'), 'identifiant');
  check('réglages : identifiant écrit dans la page', true);
  // Classe ajoutée puis retirée.
  await page.fill('.dock-panel[data-panel="settings"] .classes-add', 'essai-classe');
  await page.press('.dock-panel[data-panel="settings"] .classes-add', 'Tab');
  f = await waitFrame(() => document.querySelector('section.hero')?.classList.contains('essai-classe'), 'classe ajoutée');
  const cls = nodeById((await invoke('edit_open')).site, secId).attrs.find((a) => a.name === 'class').value;
  check('réglages : classe ajoutée', cls.split(' ').includes('essai-classe') && cls.split(' ')[0] === 'hero', cls);
  await page.click('.dock-panel[data-panel="settings"] .chip--class:has-text("essai-classe") .chip-x');
  f = await waitFrame(() => !document.querySelector('section.hero')?.classList.contains('essai-classe'), 'classe retirée');
  check('réglages : classe retirée', true);

  // Ajouter un titre dans la section sélectionnée.
  await page.click('.dock-tab[data-panel="insert"]');
  await page.click('.insert-item:has-text("Titre")');
  f = await waitFrame(() => [...document.querySelectorAll('section.hero h2')].some((h) => h.textContent === 'Nouveau titre'), 'titre ajouté');
  await page.waitForFunction(() => document.querySelector('.editor-status .node-crumb.is-on')?.textContent === 'h2');
  check('ajouter : titre inséré dans la section, sélectionné', true);
  await page.click('.dock-tab[data-panel="layers"]');

  // Lien : nouvel onglet (target + rel).
  await page.frameLocator('iframe.canvas-frame').locator('header nav a', { hasText: 'Agence' }).first().click();
  await page.waitForSelector('.dock-panel[data-panel="settings"] .row-switch');
  await page.click('.dock-panel[data-panel="settings"] .row-switch .switch');
  f = await waitFrame(() => [...document.querySelectorAll('header nav a')].some((a) => a.textContent.trim() === 'Agence' && a.target === '_blank' && a.rel === 'noopener'), 'nouvel onglet');
  check('lien : ouverture dans un nouvel onglet', true);

  // Image : champs de l'image.
  await page.locator('.layers .layer-label', { hasText: /^image / }).first().click();
  await page.waitForFunction(() => /Texte alternatif|Image \(/.test(document.querySelector('.dock-panel[data-panel="settings"]')?.textContent || ''));
  check('image : source et texte alternatif proposés', true);

  // Tout annuler.
  for (let i = 0; i < 6; i++) {
    const st = await invoke('edit_open');
    if (JSON.stringify(st.site) === JSON.stringify(base25.site)) break;
    await page.click('.editor-toolbar .btn-undo');
    await page.waitForTimeout(400);
  }
  check('réglages et ajout : tout annulé, document identique', JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(base25.site));
  if (shots) await page.screenshot({ path: `${shots}/editeur-reglages.png` });


  // ------------------------------------------------------------------ espace de travail (D-21)
  {
    const inCol = (side, panel) => page.$(`.dock-col--${side} .dock-tab[data-panel="${panel}"]`);
    const dragTo = async (from, to, pos) => {
      const a = await page.locator(from).boundingBox();
      const b = await page.locator(to).first().boundingBox();
      await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
      await page.mouse.down();
      await page.mouse.move(a.x + a.width / 2 + 12, a.y + a.height / 2 + 4, { steps: 3 });
      const tx = pos ? b.x + pos.x : b.x + b.width / 2;
      const ty = pos ? b.y + pos.y : b.y + b.height / 2;
      await page.mouse.move(tx, ty, { steps: 8 });
      await page.waitForTimeout(150);
      const hint = !!(await page.$('.dock-hint'));
      await page.mouse.up();
      await page.waitForTimeout(200);
      return hint;
    };
    // Onglet « CMS » déposé dans le groupe de droite.
    const hint = await dragTo('.dock-tab[data-panel="cms"]', '.dock-col--right .dock-tabs');
    check('espace de travail : onglet glissé dans une autre colonne', hint && !!(await inCol('right', 'cms')) && !(await inCol('left', 'cms')));
    const saved = await page.evaluate(() => localStorage.getItem('revel.workspace.v1') || '');
    check('espace de travail : disposition gardée', saved.includes('"cms"') && JSON.parse(saved).right[0].tabs.includes('cms'));
    // Fenêtre flottante : onglet « Médias » déposé sur le canevas.
    await page.click('.dock-tab[data-panel="assets"]');
    await dragTo('.dock-tab[data-panel="assets"]', '.canvas-area', { x: 200, y: 160 });
    check('espace de travail : panneau détaché en fenêtre flottante', !!(await page.$('.dock-float .dock-tab[data-panel="assets"]')));
    // Groupe redimensionné (poignée entre deux groupes de la colonne de gauche).
    const g2 = page.locator('.dock-col--left .dock-group').nth(1);
    const h0 = (await g2.boundingBox()).height;
    const sp = await page.locator('.dock-col--left .dock-split').first().boundingBox();
    await page.mouse.move(sp.x + sp.width / 2, sp.y + sp.height / 2);
    await page.mouse.down();
    await page.mouse.move(sp.x + sp.width / 2, sp.y + sp.height / 2 + 60, { steps: 6 });
    await page.mouse.up();
    const h1 = (await g2.boundingBox()).height;
    check('espace de travail : groupes redimensionnés', Math.abs(h0 - 60 - h1) < 4, `${h0} → ${h1}`);
    // Colonne redimensionnée.
    const lw0 = (await page.locator('.dock-col--left').boundingBox()).width;
    const rz = await page.locator('.dock-col--left .dock-resize').boundingBox();
    await page.mouse.move(rz.x + 3, rz.y + 200);
    await page.mouse.down();
    await page.mouse.move(rz.x + 43, rz.y + 200, { steps: 5 });
    await page.mouse.up();
    const lw1 = (await page.locator('.dock-col--left').boundingBox()).width;
    check('espace de travail : colonne élargie', Math.abs(lw1 - lw0 - 40) < 3, `${lw0} → ${lw1}`);
    // Fermer un panneau (menu du groupe), le rouvrir (menu « Panneaux »).
    await page.click('.dock-tab[data-panel="settings"]');
    await page.click('.dock-col--right .dock-group:has(.dock-tab[data-panel="settings"]) .dock-menu-btn');
    await page.click('.dock-menu .menu-item:has-text("Fermer")');
    check('espace de travail : panneau fermé', !(await page.$('.dock-tab[data-panel="settings"]')));
    await page.click('.panels-menu > summary');
    await page.click('.panels-pop .menu-item--check:has-text("Réglages")');
    check('espace de travail : panneau rouvert', !!(await page.$('.dock-tab[data-panel="settings"]')));
    await page.click('.panels-menu > summary');
    // Ctrl+\ : panneaux masqués puis affichés.
    await page.locator('.canvas-area').click({ position: { x: 5, y: 5 } });
    await page.keyboard.press('Control+Backslash');
    await page.waitForTimeout(150);
    const hidden = (await page.locator('.dock-col--left').boundingBox()).width;
    await page.keyboard.press('Control+Backslash');
    await page.waitForTimeout(150);
    check('espace de travail : Ctrl+\\ masque et affiche les panneaux', hidden === 0 && (await page.locator('.dock-col--left').boundingBox()).width > 100);
    if (shots) await page.screenshot({ path: `${shots}/editeur-espace-modifie.png` });
    // Disposition par défaut.
    await page.click('.panels-menu > summary');
    await page.click('.panels-pop .menu-item:has-text("Disposition par défaut")');
    await page.click('.panels-menu > summary');
    check('espace de travail : disposition par défaut rétablie', !!(await inCol('left', 'cms')) && !(await page.$('.dock-float')));
  }

  // Calques : recherche, clavier, menu contextuel ; fil d'Ariane ; zoom au clavier.
  {
    await page.click('.dock-tab[data-panel="layers"]');
    const n0 = await page.$$eval('.layers .layer', (r) => r.length);
    await page.fill('.layers .panel-search input', 'hero__title');
    await page.waitForTimeout(200);
    const found = await page.$$eval('.layers .layer', (r) => r.map((e) => e.textContent.trim()));
    check('calques : recherche (lignes trouvées et leurs parents)', found.length < n0 && found.some((t) => t === 'h1.hero__title'), `${n0} → ${found.length}`);
    await page.fill('.layers .panel-search input', '');
    await page.click('.layers .layer:has-text("h1.hero__title")');
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(200);
    const after = (await page.textContent('.layers .layer.is-selected')).trim();
    check('calques : flèche vers le bas, élément suivant', after !== 'h1.hero__title', after);
    await page.click('.layers .layer:has-text("h1.hero__title")', { button: 'right' });
    check('calques : menu contextuel', !!(await page.waitForSelector('.context-menu')));
    await page.mouse.click(700, 400);
    await page.waitForTimeout(100);
    check('calques : menu contextuel fermé par un clic ailleurs', !(await page.$('.context-menu')));
    await page.click('.layers .layer:has-text("h1.hero__title")');
    await page.click('.node-path .node-crumb:has-text("section.hero")');
    await page.waitForTimeout(200);
    check('fil d\'Ariane : parent sélectionné', (await page.textContent('.layers .layer.is-selected')).trim() === 'section.hero');
    await page.keyboard.press('Control+0');
    await page.waitForFunction(() => document.querySelector('iframe.canvas-frame').style.transform === 'scale(1)');
    await page.keyboard.press('Control+1');
    await page.waitForTimeout(150);
    check('zoom : Ctrl+0 à 100 %, Ctrl+1 ajusté', await page.$eval('.editor-zoom select', (s) => s.value === ''));
  }

  // ------------------------------------------------------------------ canevas : glisser-déposer, parcours, vue (2.12)
  {
    const base212 = await invoke('edit_open');
    const canvas = page.frameLocator('iframe.canvas-frame');
    // Élément montré dans le cadre, puis sa position dans la fenêtre.
    const boxOf = async (sel) => {
      await frameOf().evaluate((s) => document.querySelector(s).scrollIntoView({ block: 'center', behavior: 'instant' }), sel);
      await page.waitForTimeout(200);
      return canvas.locator(sel).first().boundingBox();
    };
    const links = () => [...document.querySelectorAll('.intro-agency__links a')].map((a) => a.textContent.trim()).join(' | ');
    const links0 = await frameOf().evaluate(links);

    // Glisser « Paragraphe » du panneau « Ajouter » au-dessus des liens de la présentation.
    await page.click('.dock-tab[data-panel="insert"]');
    const tile = await page.locator('.insert-item[data-item="insert-paragraph"]').boundingBox();
    const lp = await boxOf('.intro-agency__links');
    await page.mouse.move(tile.x + tile.width / 2, tile.y + tile.height / 2);
    await page.mouse.down();
    await page.mouse.move(tile.x + tile.width / 2 + 30, tile.y + 10, { steps: 3 });
    await page.mouse.move(lp.x + lp.width / 2, lp.y + 3, { steps: 10 });
    await page.waitForFunction(() => !document.querySelector('.drag-ghost.is-off'));
    check('glisser depuis « Ajouter » : emplacement montré dans la page', !!(await page.$('.drag-ghost')));
    if (shots) await page.screenshot({ path: `${shots}/editeur-glisser-ajouter.png` });
    await page.mouse.up();
    f = await waitFrame(() => { const p = document.querySelector('.intro-agency__links'); const prev = p && p.previousElementSibling; return !!prev && prev.tagName === 'P' && prev.textContent === 'Nouveau paragraphe.'; }, 'paragraphe déposé');
    check('glisser depuis « Ajouter » : paragraphe déposé à l\'endroit montré', true);
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame(() => ![...document.querySelectorAll('p')].some((p) => p.textContent === 'Nouveau paragraphe.'), 'annulation du dépôt');

    // Déplacer un lien avant l'autre (glisser dans le canevas).
    await page.click('.dock-tab[data-panel="layers"]');
    const a2 = await boxOf('.intro-agency__links a:nth-of-type(2)');
    const a1 = await canvas.locator('.intro-agency__links a').first().boundingBox();
    await page.mouse.move(a2.x + a2.width / 2, a2.y + a2.height / 2);
    await page.mouse.down();
    await page.mouse.move(a2.x + a2.width / 2 - 10, a2.y + a2.height / 2, { steps: 3 });
    await page.mouse.move(a1.x + 6, a1.y + a1.height / 2, { steps: 10 });
    await page.waitForTimeout(150);
    if (shots) await page.screenshot({ path: `${shots}/editeur-deplacer.png` });
    await page.mouse.up();
    f = await waitFrame((l0) => document.readyState === 'complete' && !!window.__revelSlot && (() => { const l = [...document.querySelectorAll('.intro-agency__links a')].map((a) => a.textContent.trim()); return l.length === 2 && l.join(' | ') === l0.split(' | ').reverse().join(' | '); })(), 'lien déplacé', links0);
    check('glisser dans le canevas : élément déplacé', true, await f.evaluate(links));
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame((l0) => document.readyState === 'complete' && !!window.__revelSlot && [...document.querySelectorAll('.intro-agency__links a')].map((a) => a.textContent.trim()).join(' | ') === l0, 'annulation du déplacement', links0);

    // Alt+glisser : copie.
    const b1 = await boxOf('.intro-agency__links a');
    const b2 = await canvas.locator('.intro-agency__links a').nth(1).boundingBox();
    await page.mouse.move(b1.x + b1.width / 2, b1.y + b1.height / 2);
    await page.mouse.down();
    await page.mouse.move(b1.x + b1.width / 2 + 10, b1.y + b1.height / 2, { steps: 3 });
    await page.keyboard.down('Alt');
    await page.mouse.move(b2.x + b2.width - 6, b2.y + b2.height / 2, { steps: 10 });
    await page.mouse.up();
    await page.keyboard.up('Alt');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && document.querySelectorAll('.intro-agency__links a').length === 3, 'copie par Alt+glisser');
    check('Alt+glisser : copie déposée', true, await f.evaluate(links));
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && document.querySelectorAll('.intro-agency__links a').length === 2, 'annulation de la copie');

    // Parcours de la sélection au clavier (comme Figma).
    const crumb = () => page.$eval('.editor-status .node-crumb.is-on', (e) => e.textContent);
    await canvas.locator('.intro-agency__links a').first().click();
    await page.waitForFunction(() => /^a\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.textContent || ''));
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => /button--ghost|a\.button$/.test(document.querySelector('.editor-status .node-crumb.is-on')?.textContent || '') && document.querySelectorAll('.editor-status .node-crumb').length > 0);
    const sib = await crumb();
    await page.keyboard.press('Shift+Enter');
    await page.waitForFunction(() => /^p\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.textContent || ''));
    const par = await crumb();
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => /^a\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.textContent || ''));
    check('clavier : Tab frère suivant, Maj+Entrée parent, Entrée enfant', true, `${sib} → ${par} → ${await crumb()}`);

    // Menu contextuel du canevas : copier, puis coller à côté (Ctrl+V).
    await canvas.locator('.intro-agency__links a').first().click({ button: 'right' });
    await page.waitForSelector('.canvas-menu');
    check('clic droit dans le canevas : menu de l\'éditeur', true);
    if (shots) await page.screenshot({ path: `${shots}/editeur-menu-canevas.png` });
    await page.click('.canvas-menu .menu-item:has-text("Copier")');
    await page.waitForFunction(() => !document.querySelector('.canvas-menu'));
    await canvas.locator('.intro-agency__links a').nth(1).click();
    await page.keyboard.press('Control+v');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && document.querySelectorAll('.intro-agency__links a').length === 3, 'collage');
    check('copier / coller : élément collé à côté de la sélection', true, await f.evaluate(links));
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && document.querySelectorAll('.intro-agency__links a').length === 2, 'annulation du collage');
    // Envelopper (Ctrl+G) puis retirer l'enveloppe (Ctrl+Maj+G).
    const wrapBase = JSON.stringify((await invoke('edit_open')).site);
    await canvas.locator('.intro-agency__links a').first().click();
    await page.keyboard.press('Control+g');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && !!document.querySelector('.intro-agency__links > span > a'), 'enveloppe');
    await page.waitForFunction(() => document.querySelector('.editor-status .node-crumb.is-on')?.textContent === 'span');
    await page.keyboard.press('Control+Shift+g');
    f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot && !document.querySelector('.intro-agency__links > span') && document.querySelectorAll('.intro-agency__links > a').length === 2, 'enveloppe retirée');
    check('Ctrl+G puis Ctrl+Maj+G : enveloppe (span dans un paragraphe) ajoutée puis retirée, document identique', JSON.stringify((await invoke('edit_open')).site) === wrapBase);
    for (let i = 0; i < 2; i++) { await page.click('.editor-toolbar .btn-undo'); await page.waitForTimeout(400); }
    await canvas.locator('.intro-agency__links a').first().click();

    // Vue : zoom sur la sélection, ajusté, déplacement à l'espace, molette autour du cadre.
    await page.keyboard.press('Shift+Digit2');
    await page.waitForFunction(() => document.querySelector('.editor-zoom select').value !== '');
    const zSel = await page.$eval('iframe.canvas-frame', (e) => e.style.transform);
    check('Maj+2 : zoom sur la sélection', /scale\((?!1\))/.test(zSel), zSel);
    await page.keyboard.press('Shift+Digit1');
    await page.waitForFunction(() => document.querySelector('.editor-zoom select').value === '');
    check('Maj+1 : zoom ajusté', true);
    const pos = () => page.$eval('iframe.canvas-frame', (e) => [parseFloat(e.style.left), parseFloat(e.style.top)]);
    const area = await page.locator('.canvas-area').boundingBox();
    const p0 = await pos();
    await page.mouse.move(area.x + area.width / 2, area.y + area.height / 2);
    await page.keyboard.down('Space');
    await page.waitForSelector('.canvas-area.is-space');
    await page.mouse.down();
    await page.mouse.move(area.x + area.width / 2 - 120, area.y + area.height / 2 - 80, { steps: 6 });
    await page.mouse.up();
    await page.keyboard.up('Space');
    const p1 = await pos();
    check('espace+glisser : la vue se déplace', Math.abs(p1[0] - p0[0] + 120) < 2 && Math.abs(p1[1] - p0[1] + 80) < 2, `${p0} → ${p1}`);
    // Bord droit de la zone, hors du cadre (déplacé vers la gauche).
    await page.mouse.move(area.x + area.width - 6, area.y + area.height / 2);
    await page.mouse.wheel(0, 100);
    await page.waitForTimeout(150);
    const p2 = await pos();
    check('molette autour du cadre : la vue se déplace', Math.abs(p2[1] - p1[1] + 100) < 2, `${p1} → ${p2}`);
    await page.keyboard.press('Shift+Digit1');
    await page.waitForFunction(() => document.querySelector('.editor-zoom select').value === '');

    // Vues du site : CMS puis retour à l'éditeur sans recharger le canevas.
    await frameOf().evaluate(() => { window.__keep = 1; });
    await page.click('.view-tab[data-view="view-cms"]');
    await page.waitForSelector('.cms-table tbody tr[data-entry]');
    check('vue CMS : collections et contenus du site', (await page.$$('.cms-col')).length > 0, `${(await page.$$('.cms-col')).length} collections`);
    if (shots) await page.screenshot({ path: `${shots}/vue-cms.png` });
    await page.click('.view-tab[data-view="view-settings"]');
    await page.waitForSelector('.settings-nav');
    await page.click('.view-tab[data-view="view-editor"]');
    await page.waitForSelector('.view-pane--editor:not(.is-hidden)');
    check('retour à l\'éditeur : canevas gardé (pas rechargé)', await frameOf().evaluate(() => window.__keep === 1));
    // Contenu ouvert depuis la vue CMS : sa page s'affiche dans l'éditeur.
    await page.click('.view-tab[data-view="view-cms"]');
    await page.click('.cms-table tbody tr[data-entry] .icon-btn');
    await page.waitForSelector('.view-pane--editor:not(.is-hidden)');
    const entryUrl = await page.$eval('.editor-page select', (s) => s.value);
    f = await waitFrame((u) => location.pathname === u && !!window.__revelSlot, 'page du contenu', entryUrl);
    check('vue CMS : « Ouvrir la page » affiche le contenu dans l\'éditeur', entryUrl !== '/', entryUrl);
    await page.selectOption('.editor-page select', '/');
    f = await waitFrame(() => location.pathname === '/' && !!window.__revelSlot, 'retour à l\'accueil');

    check('canevas : tout annulé, document identique', JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(base212.site));
  }

  // ------------------------------------------------------------------ styles et points de rupture (2.6, 2.8)
  const base26 = await invoke('edit_open');
  const rulesOf = (site, sel) => site.stylesheet.items.map((it, i) => [it, i]).filter(([it]) => it.item === 'rule' && it.selector === sel);
  await page.click('.layers .layer:has-text("h1.hero__title")');
  await page.click('.dock-tab[data-panel="styles"]');
  await page.waitForFunction(() => document.querySelector('.styles .sel-chip.is-on')?.textContent.trim() === 'hero__title');
  check('styles : sélecteur de la classe de l\'élément', true);
  const styleInput = (prop) => page.locator(`.style-row[data-prop="${prop}"]`).locator('input, select').first();
  f = frameOf();
  await f.evaluate(() => { window.__mark2 = 1; });
  await styleInput('color').fill('rgb(10, 20, 30)');
  await styleInput('color').press('Enter');
  f = await waitFrame(() => getComputedStyle(document.querySelector('h1.hero__title')).color === 'rgb(10, 20, 30)', 'couleur du titre');
  check('styles : couleur appliquée en direct (sans rechargement)', await f.evaluate(() => window.__mark2 === 1));
  const r1 = rulesOf((await invoke('edit_open')).site, '.hero__title').find(([it]) => !it.media);
  check('styles : déclaration écrite dans la règle de base', r1 && r1[0].decls.some((d) => d.prop === 'color' && d.value === 'rgb(10, 20, 30)'));

  // Panneau de styles (D-20) : contrôles visuels, une seule étape d'annulation par geste.
  const titleRule = async () => rulesOf((await invoke('edit_open')).site, '.hero__title').find(([it]) => !it.media)[0];
  const decl = (rule, prop) => (rule.decls.filter((d) => d.prop === prop).pop() || {}).value;
  const cs = async (prop) => (await frameOf().evaluate((p) => getComputedStyle(document.querySelector('h1.hero__title')).getPropertyValue(p), prop));
  const dragBy = async (sel, dx, dy) => {
    await page.locator(sel).first().scrollIntoViewIfNeeded();
    const b = await page.locator(sel).first().boundingBox();
    await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
    await page.mouse.down();
    await page.mouse.move(b.x + b.width / 2 + dx / 2, b.y + b.height / 2 + dy / 2, { steps: 4 });
    await page.mouse.move(b.x + b.width / 2 + dx, b.y + b.height / 2 + dy, { steps: 4 });
    return async () => page.mouse.up();
  };
  // Boîte des marges : glisser la marge du haut de 20 px vers le haut.
  {
    const before = await titleRule();
    const mt0 = parseFloat(await cs('margin-top'));
    const release = await dragBy('.bm-side[data-prop="margin-top"] .bm-value', 0, -20);
    await page.waitForTimeout(250);
    const during = parseFloat(await cs('margin-top'));
    const unchanged = JSON.stringify(await titleRule()) === JSON.stringify(before);
    await release();
    await page.waitForFunction(() => document.querySelector('.save-state')?.textContent?.length > 0);
    await page.waitForTimeout(600);
    const after = await titleRule();
    const mt = decl(after, 'margin-top') || decl(after, 'margin');
    check('styles : marge glissée, aperçu sans écriture puis une valeur écrite', during === mt0 + 20 && unchanged && !!mt && parseFloat(await cs('margin-top')) === mt0 + 20,
      `${mt0} → ${during} (aperçu), règle : ${mt}`);
    await page.click('.editor-toolbar .btn-undo');
    await page.waitForTimeout(600);
    check('styles : un seul Ctrl+Z annule le glisser', JSON.stringify(await titleRule()) === JSON.stringify(before));
  }
  // Sélecteur de couleur : glisser dans le plan saturation-luminosité.
  {
    const before = await titleRule();
    await page.locator('.style-row[data-prop="color"] .color-swatch').click();
    await page.waitForSelector('.cp-plane');
    const c0 = await cs('color');
    const release = await dragBy('.cp-plane', -40, 30);
    await page.waitForTimeout(250);
    const during = await cs('color');
    await release();
    await page.waitForTimeout(800);
    const after = decl(await titleRule(), 'color');
    check('styles : couleur choisie au glisser, aperçu en direct puis écrite', during !== c0 && /^#[0-9a-f]{6}$/.test(after || ''), `${c0} → ${during}, règle : ${after}`);
    if (shots) await page.screenshot({ path: `${shots}/editeur-couleur.png` });
    await page.keyboard.press('Escape');
    check('styles : Échap ferme le sélecteur sans désélectionner', !(await page.$('.cp')) && !!(await page.$('.style-row[data-prop="color"]')));
    // Alt+clic sur le libellé : valeur retirée.
    await page.locator('.style-row[data-prop="color"] .style-label').click({ modifiers: ['Alt'] });
    await page.waitForTimeout(600);
    check('styles : Alt+clic sur le libellé réinitialise', decl(await titleRule(), 'color') === undefined);
    for (let i = 0; i < 2; i++) { await page.click('.editor-toolbar .btn-undo'); await page.waitForTimeout(500); }
    check('styles : couleur annulée', JSON.stringify(await titleRule()) === JSON.stringify(before));
  }
  // Menu d'unités et boutons à icônes.
  {
    const before = await titleRule();
    await page.locator('.style-row[data-prop="font-size"] .nbox-unit').click();
    await page.locator('.pop .menu-item:text-is("rem")').click();
    await page.waitForTimeout(600);
    const fs1 = decl(await titleRule(), 'font-size') || '';
    check('styles : unité choisie dans le menu', /rem$/.test(fs1), fs1);
    await page.locator('.style-row[data-prop="text-align"] button[role="radio"]').nth(1).click();
    await page.waitForTimeout(600);
    check('styles : alignement du texte par bouton', decl(await titleRule(), 'text-align') === 'center' && (await cs('text-align')) === 'center');
    for (let i = 0; i < 2; i++) { await page.click('.editor-toolbar .btn-undo'); await page.waitForTimeout(500); }
    check('styles : unité et alignement annulés', JSON.stringify(await titleRule()) === JSON.stringify(before));
  }

  // Point de rupture 600 : nouvelle règle placée dans la cascade.
  const bps2 = await page.$$('.editor-toolbar .segmented button');
  await bps2[bps2.length - 1].click();
  await page.waitForTimeout(400);
  await styleInput('font-size').fill('40px');
  await styleInput('font-size').press('Enter');
  f = await waitFrame(() => getComputedStyle(document.querySelector('h1.hero__title')).fontSize === '40px', 'taille à 600 px');
  const after600 = (await invoke('edit_open')).site;
  const rs = rulesOf(after600, '.hero__title');
  const r600 = rs.find(([it]) => it.media === '(max-width: 600px)');
  const wider = rs.filter(([it]) => !it.media || /max-width: (9|8)\d\dpx/.test(it.media)).map(([, i]) => i);
  check('styles : règle créée pour 600 px, après les règles plus larges du même sélecteur', !!r600 && wider.every((i) => i < r600[1]), `position ${r600 && r600[1]} > ${wider.join(',')}`);
  await bps2[0].click();
  await page.waitForTimeout(400);
  f = frameOf();
  const desktopSize = await f.evaluate(() => getComputedStyle(document.querySelector('h1.hero__title')).fontSize);
  check('styles : la base n\'est pas touchée par la règle 600 px', desktopSize !== '40px', desktopSize);

  // Points de rupture : ajout, retrait ; retrait refusé s'il a des styles.
  await page.click('.bp-menu > summary');
  await page.fill('.bp-add input', '480');
  await page.click('.bp-add .btn');
  await page.waitForFunction(() => [...document.querySelectorAll('.editor-toolbar .segmented .bp-width')].some((e) => e.textContent === '480'));
  check('points de rupture : 480 px ajouté', JSON.stringify((await invoke('edit_open')).site.breakpoints).includes('max-480'));
  await page.click('.bp-line:has-text("480") .icon-btn');
  await page.waitForFunction(() => ![...document.querySelectorAll('.editor-toolbar .segmented .bp-width')].some((e) => e.textContent === '480'));
  await page.click('.bp-line:has-text("600") .icon-btn');
  await page.waitForSelector('.notice--error');
  check('points de rupture : retrait refusé quand des styles l\'utilisent', JSON.stringify((await invoke('edit_open')).site.breakpoints).includes('max-600'));
  await page.click('.notice--error .icon-btn');
  await page.click('.bp-menu > summary');
  if (shots) await page.screenshot({ path: `${shots}/editeur-styles.png` });
  // Aucun libellé tronqué dans l'inspecteur (sections dépliées).
  for (const d of await page.$$('.dock-panel[data-panel="styles"] details:not([open]) > summary')) await d.click().catch(() => {});
  const truncated = () => page.$$eval('.dock-panel[data-panel="styles"] :is(.style-label, .style-label-text, .label, .seg button span, .mini-label), .dock-tab-label', (els) =>
    els.filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => e.textContent.trim()));
  const cut = await truncated();
  check('styles : aucun libellé tronqué (français)', cut.length === 0, cut.join(', '));
  if (shots) await page.locator('.dock-col--right').screenshot({ path: `${shots}/editeur-styles-panneau-fr.png` });
  await page.evaluate(() => window.__revelTest.lang('en'));
  await page.waitForFunction(() => document.querySelector('.dock-tab[data-panel="settings"]')?.textContent.includes('Settings'));
  const cutEn = await truncated();
  check('styles : aucun libellé tronqué (anglais)', cutEn.length === 0, cutEn.join(', '));
  if (shots) await page.locator('.dock-col--right').screenshot({ path: `${shots}/editeur-styles-panneau-en.png` });
  await page.evaluate(() => window.__revelTest.lang('fr'));

  // Tout annuler.
  for (let i = 0; i < 20; i++) {
    if (JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(base26.site)) break;
    await page.click('.editor-toolbar .btn-undo');
    await page.waitForTimeout(400);
  }
  check('styles : tout annulé, document identique', JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(base26.site));


  // ------------------------------------------------------------------ textes (étape 2.7)
  const base27 = await invoke('edit_open');
  // Position (dans la page de l'interface) du repère n du canevas, cadre mis à l'échelle compris.
  const slotPoint = async (pred) => {
    await page.waitForTimeout(300);
    f = await waitFrame(() => typeof window.__revelSlot === 'function' && document.readyState === 'complete', 'canevas prêt');
    const r = await f.evaluate((src) => {
      const test = new Function('s', 'return ' + src);
      for (let n = 0; n < 5000; n++) {
        const x = window.__revelSlot(n);
        if (!x.slot) return null;
        if (!x.range || !test(x.slot)) continue;
        const b = [...x.range.getClientRects()].find((q) => q.width > 4 && q.height > 4 && q.top > 0 && q.bottom < innerHeight);
        if (b) return { n, x: b.left + Math.min(b.width / 2, 20), y: b.top + b.height / 2, text: x.range.toString(), slot: x.slot };
      }
      return null;
    }, pred);
    if (!r) return null;
    const box = await page.locator('iframe.canvas-frame').boundingBox();
    const w = parseFloat(await page.$eval('iframe.canvas-frame', (e) => e.style.width));
    const k = box.width / w;
    return { ...r, px: box.x + r.x * k, py: box.y + r.y * k };
  };
  // Texte écrit dans la page.
  const t1 = await slotPoint("s.slot === 'text' && s.parent.owner.kind === 'page'");
  check('textes : texte de la page repéré', !!t1, t1 && t1.text.trim());
  if (t1) {
    await page.mouse.dblclick(t1.px, t1.py);
    f = await waitFrame(() => !!document.querySelector('[data-rv-edit]'), 'mode édition');
    await page.keyboard.press('Control+a');
    await page.keyboard.type('Texte essai');
    await page.keyboard.press('Enter');
    let saved = false;
    for (let i = 0; i < 50 && !saved; i++) {
      saved = JSON.stringify((await invoke('edit_open')).site).includes('Texte essai');
      if (!saved) await page.waitForTimeout(100);
    }
    f = await waitFrame(() => document.body.textContent.includes('Texte essai') && !document.querySelector('[data-rv-edit]'), 'texte enregistré');
    check('textes : texte de la page modifié sur place (SetText)', saved);
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame(() => !document.body.textContent.includes('Texte essai'), 'annulation du texte');
  }
  // Texte lié à l'en-tête d'un contenu (fichier Markdown).
  const t2 = await slotPoint("s.slot === 'expr' && s.binding && s.binding.source.kind === 'entry' && !s.transformed && !s.html");
  check('textes : texte lié à un contenu repéré', !!t2, t2 && `${t2.text.trim()} (${t2.slot.binding.source.folder}/${t2.slot.binding.source.file})`);
  if (t2 && siteDir) {
    const file = path.join(siteDir, t2.slot.binding.source.folder, t2.slot.binding.source.file + '.md');
    const before = fs.readFileSync(file, 'utf8');
    await page.mouse.dblclick(t2.px, t2.py);
    f = await waitFrame(() => !!document.querySelector('[data-rv-edit]'), 'mode édition (lié)');
    await page.keyboard.press('Control+a');
    await page.keyboard.type('Titre essai');
    await page.keyboard.press('Enter');
    let after = before;
    for (let i = 0; i < 50 && after === before; i++) {
      await page.waitForTimeout(100);
      after = fs.readFileSync(file, 'utf8');
    }
    f = await waitFrame(() => document.body.textContent.includes('Titre essai'), 'texte lié enregistré');
    const changed = after.split('\n').filter((l, i) => l !== before.split('\n')[i]);
    check('textes : seule la ligne du titre change dans le fichier du contenu', changed.length === 1 && changed[0].includes('Titre essai'), changed.join(' | '));
    await page.click('.editor-toolbar .btn-undo');
    f = await waitFrame(() => !document.body.textContent.includes('Titre essai'), 'annulation du texte lié');
    let restored = false;
    for (let i = 0; i < 50 && !restored; i++) {
      restored = fs.readFileSync(file, 'utf8') === before;
      if (!restored) await page.waitForTimeout(100);
    }
    check('textes : annulé, fichier identique octet pour octet', restored);

    // Modification hors de Revel (comme un enregistrement de Pages CMS) : page rechargée.
    await page.waitForTimeout(2600);
    fs.writeFileSync(file, before.replace(t2.text.trim(), 'Modifié dehors'));
    f = await waitFrame(() => document.body.textContent.includes('Modifié dehors'), 'relecture du fichier');
    check('textes : fichier modifié hors de Revel, page relue', true);
    fs.writeFileSync(file, before);
    f = await waitFrame(() => !document.body.textContent.includes('Modifié dehors'), 'relecture après remise en état');
  }
  // Échap : abandon sans modification.
  const t3 = await slotPoint("s.slot === 'text'");
  if (t3) {
    await page.mouse.dblclick(t3.px, t3.py);
    f = await waitFrame(() => !!document.querySelector('[data-rv-edit]'), 'mode édition (Échap)');
    await page.keyboard.type('xyz');
    await page.keyboard.press('Escape');
    f = await waitFrame(() => !document.querySelector('[data-rv-edit]'), 'abandon');
    check('textes : Échap abandonne la saisie', !(await f.evaluate(() => document.body.textContent.includes('xyz'))));
  }
  check('textes : document identique', JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(base27.site));

  // ------------------------------------------------------------------ CMS : contenus modifiés (3.1)
  if (siteDir) {
    const dir = path.join(siteDir, 'src/content/projets');
    const listing = () => fs.readdirSync(dir).sort().join(',');
    const list0 = listing();
    const until = async (pred, what) => {
      for (let i = 0; i < 60; i++) { if (pred()) return true; await page.waitForTimeout(100); }
      if (shots) await page.screenshot({ path: `${shots}/cms-echec.png` });
      const notice = await page.$eval('.notice', (n) => n.textContent).catch(() => '');
      throw new Error('CMS : ' + what + (notice ? ` (${notice})` : ''));
    };
    await page.click('.view-tab[data-view="view-cms"]');
    await page.click('.cms-col[data-collection="projets"]');
    await page.waitForSelector('.cms-table tbody tr[data-entry]');
    const first = await page.$eval('.cms-table tbody tr[data-entry]', (r) => r.dataset.entry);
    const file = path.join(dir, first + '.md');
    const original = fs.readFileSync(file, 'utf8');
    await page.click(`.cms-table tbody tr[data-entry="${first}"]`);
    await page.waitForSelector('.cms-item [data-field="title"] input');
    await page.fill('.cms-item [data-field="title"] input', 'Titre essai CMS');
    await page.keyboard.press('Tab');
    await until(() => fs.readFileSync(file, 'utf8').includes('Titre essai CMS'), 'titre écrit');
    const changed = fs.readFileSync(file, 'utf8').split('\n').filter((l, i) => l !== original.split('\n')[i]);
    check('CMS : champ modifié, seule sa ligne change dans le fichier', changed.length === 1 && changed[0].startsWith('title:'), changed.join(' | '));
    // Texte du contenu (texte riche écrit en Markdown) : saisi, puis mis en gras.
    const body = '.cms-item [data-field="--body"] .rt-body';
    await page.click(body);
    await page.keyboard.type('Texte essai CMS');
    await page.click('.cms-item-head h2');
    await until(() => fs.readFileSync(file, 'utf8').endsWith('---\nTexte essai CMS\n'), 'texte écrit');
    await page.click(body);
    await page.keyboard.press('Control+a');
    await page.click('.cms-item [data-field="--body"] .rt-btn[title^="Gras"]');
    await page.click('.cms-item-head h2');
    await until(() => fs.readFileSync(file, 'utf8').endsWith('---\n**Texte essai CMS**\n'), 'gras');
    check('CMS : texte riche saisi et mis en gras, écrit en Markdown', true);
    // Champ HTML importé (texte riche écrit en HTML, bloc littéral gardé).
    await page.click('.cms-item [data-field="intro"] .rt-body');
    await page.keyboard.press('Control+End');
    await page.keyboard.type(' Ajout essai.');
    await page.click('.cms-item-head h2');
    await until(() => /intro: \|-\n[^]*Ajout essai\.<\/p>/.test(fs.readFileSync(file, 'utf8')), 'champ HTML');
    check('CMS : champ HTML modifié au texte riche, bloc littéral gardé', true);
    if (shots) await page.screenshot({ path: `${shots}/cms-fiche.png` });
    for (let i = 0; i < 4; i++) {
      await page.click('.cms-head .icon-btn[title^="Annuler"]');
      await page.waitForTimeout(300);
    }
    await until(() => fs.readFileSync(file, 'utf8') === original, 'annulation');
    check('CMS : annulé, fichier identique octet pour octet', true);
    // Nouveau contenu, puis supprimé ; annulé : dossier identique.
    await page.click('.cms-new');
    await page.fill('.dialog input', 'Essai Revel');
    await page.press('.dialog input', 'Enter');
    const created = path.join(dir, 'essai-revel.md');
    await until(() => fs.existsSync(created), 'contenu créé');
    await page.waitForSelector('.cms-item[data-entry="essai-revel"]');
    check('CMS : nouveau contenu créé et ouvert', fs.readFileSync(created, 'utf8').startsWith('---\ntitle: Essai Revel\n'), fs.readFileSync(created, 'utf8').split('\n').slice(0, 4).join(' | '));
    await page.click('.cms-item-foot .icon-btn');
    await page.click('.dialog .btn--danger');
    await until(() => !fs.existsSync(created), 'contenu supprimé');
    check('CMS : contenu supprimé', true);
    await page.keyboard.press('Control+z');
    await until(() => fs.existsSync(created), 'suppression annulée (Ctrl+Z)');
    await page.keyboard.press('Control+z');
    await until(() => !fs.existsSync(created) && listing() === list0, 'création annulée');
    check('CMS : Ctrl+Z rétablit puis retire le contenu, dossier identique', true);
    await page.click('.view-tab[data-view="view-editor"]');
  }

  check('aucune erreur dans l\'interface', errors.length === 0, errors.join(' | '));
} catch (e) {
  check('déroulement', false, e.message);
} finally {
  await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
