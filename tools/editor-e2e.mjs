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
  await page.click('.template-choice[data-template="new-site-template-showcase"]');
  check('nouveau site : modèles proposés (vide, vitrine, blog)', (await page.$$('.template-choice')).length === 3
    && (await page.getAttribute('.template-choice[data-template="new-site-template-showcase"]', 'aria-checked')) === 'true');
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

  // Lien : nouvel onglet (target + rel). Le lien est dans le composant de l'en-tête : un clic
  // sélectionne l'utilisation, un double-clic ouvre le composant sur le lien.
  await page.frameLocator('iframe.canvas-frame').locator('header nav a', { hasText: 'Agence' }).first().click();
  await page.waitForSelector('.dock-panel[data-panel="settings"] .instance-name');
  check('composant : un clic sélectionne l\'utilisation entière', (await page.textContent('.dock-panel[data-panel="settings"] .instance-name')).trim() === 'header');
  await page.frameLocator('iframe.canvas-frame').locator('header nav a', { hasText: 'Agence' }).first().dblclick();
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
    // Onglet « Ajouter » déposé dans le groupe de droite.
    const hint = await dragTo('.dock-tab[data-panel="insert"]', '.dock-col--right .dock-tabs');
    check('espace de travail : onglet glissé dans une autre colonne', hint && !!(await inCol('right', 'insert')) && !(await inCol('left', 'insert')));
    const saved = await page.evaluate(() => localStorage.getItem('revel.workspace.v1') || '');
    check('espace de travail : disposition gardée', saved.includes('"insert"') && JSON.parse(saved).right[0].tabs.includes('insert'));
    // Fenêtre flottante : onglet « Médias » déposé sur le canevas.
    await page.click('.dock-tab[data-panel="assets"]');
    await dragTo('.dock-tab[data-panel="assets"]', '.canvas-area', { x: 200, y: 160 });
    check('espace de travail : panneau détaché en fenêtre flottante', !!(await page.$('.dock-float .dock-tab[data-panel="assets"]')));
    // Fenêtre déplacée par son bandeau (onglet seul compris), puis ancrée avec Maj.
    const fpos = () => page.$eval('.dock-float', (f) => [parseFloat(f.style.left), parseFloat(f.style.top)]);
    const f0 = await fpos();
    const ft = await page.locator('.dock-float .dock-tab').boundingBox();
    await page.mouse.move(ft.x + ft.width / 2, ft.y + ft.height / 2);
    await page.mouse.down();
    await page.mouse.move(ft.x + ft.width / 2 + 80, ft.y + ft.height / 2 + 40, { steps: 6 });
    await page.mouse.up();
    const f1 = await fpos();
    check('fenêtre détachée : déplacée par son bandeau', Math.abs(f1[0] - f0[0] - 80) < 2 && Math.abs(f1[1] - f0[1] - 40) < 2, `${f0} → ${f1}`);
    const fh = await page.locator('.dock-float .dock-head').boundingBox();
    const rt = await page.locator('.dock-col--right .dock-tabs').first().boundingBox();
    await page.mouse.move(fh.x + fh.width - 40, fh.y + fh.height / 2);
    await page.mouse.down();
    await page.keyboard.down('Shift');
    await page.mouse.move(rt.x + rt.width / 2, rt.y + rt.height / 2, { steps: 10 });
    const shiftHint = !!(await page.$('.dock-hint'));
    await page.mouse.up();
    await page.keyboard.up('Shift');
    await page.waitForTimeout(150);
    check('fenêtre détachée : Maj montre l\'ancrage et range la fenêtre', shiftHint && !!(await inCol('right', 'assets')) && !(await page.$('.dock-float')));
    // Menu contextuel d'un onglet : détacher, ancrer.
    await page.click('.dock-tab[data-panel="assets"]', { button: 'right' });
    await page.waitForSelector('.ctx-menu');
    const labels = await page.$$eval('.ctx-menu .menu-label', (ls) => ls.map((l) => l.textContent));
    await page.click('.ctx-menu .menu-item:has-text("Détacher")');
    await page.waitForSelector('.dock-float .dock-tab[data-panel="assets"]');
    await page.click('.dock-float .dock-tab[data-panel="assets"]', { button: 'right' });
    await page.click('.ctx-menu .menu-item:has-text("Ancrer")');
    await page.waitForFunction(() => !document.querySelector('.dock-float'));
    check('onglet : menu contextuel (détacher, ancrer, fermer)', labels.join('|') === 'Détacher|Fermer', labels.join('|'));
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
    check('espace de travail : disposition par défaut rétablie', !!(await inCol('left', 'insert')) && !(await page.$('.dock-float')));
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
    // Une ligne par action : icône et libellé alignés, raccourci à droite, sans répétition.
    const rowsOk = await page.$$eval('.context-menu .context-item', (items) => items.every((it) => {
      const i = it.querySelector('.menu-icon').getBoundingClientRect(), l = it.querySelector('.menu-label').getBoundingClientRect();
      return Math.abs((i.top + i.bottom) / 2 - (l.top + l.bottom) / 2) < 3 && !/\(/.test(l.textContent);
    }));
    check('calques : menu en lignes (icône, libellé, raccourci)', rowsOk);
    await page.mouse.click(700, 400);
    await page.waitForTimeout(100);
    check('calques : menu contextuel fermé par un clic ailleurs', !(await page.$('.context-menu')));
    await page.click('.layers .layer:has-text("h1.hero__title")');
    await page.click('.node-path .node-crumb:has-text("section.hero")');
    await page.waitForTimeout(200);
    check('fil d\'Ariane : parent sélectionné', (await page.textContent('.layers .layer.is-selected')).trim() === 'section.hero');
    // Variables : affectations de la page (plus dans les calques), valeur calculée au rendu ;
    // expression modifiée sur place, refusée si sa syntaxe est fausse, annulée par Ctrl+Z.
    const varLayers = await page.$$eval('.layers .layer', (r) => r.filter((e) => /^variable /.test(e.textContent.trim())).length);
    // Panneau fermé par défaut : ouvert par le menu « Panneaux », à côté des calques.
    const varHidden = !(await page.$('.dock-tab[data-panel="variables"]'));
    await page.click('.panels-menu > summary');
    await page.click('.panels-pop .menu-item--check:has-text("Variables")');
    await page.click('.panels-menu > summary');
    const besideLayers = await page.$eval('.dock-tab[data-panel="variables"]', (t) => { const ps = [...t.closest('.dock-tabs').querySelectorAll('.dock-tab')].map((x) => x.dataset.panel); return ps.indexOf('variables') === ps.indexOf('layers') + 1; });
    check('variables : panneau fermé par défaut, rouvert à côté des calques', varHidden && besideLayers);
    await page.click('.dock-tab[data-panel="variables"]');
    await page.waitForSelector('.variables .var[data-var="canonical"] .var-value:not(.is-muted)');
    const owners = await page.$$eval('.variables .var-owner .list-label', (r) => r.map((e) => e.textContent.trim()));
    const canon = '.variables .var[data-var="canonical"]';
    const v0 = (await page.textContent(canon + ' .var-value')).trim();
    check('variables : rangées par gabarit, page et composant, valeur calculée', varLayers === 0 && owners.includes('base') && owners.includes('header') && /^https?:\/\//.test(v0), `${owners.join(', ')} ; canonical = ${v0}`);
    const docV = JSON.stringify((await invoke('edit_open')).site);
    await page.click(canon + ' .var-head');
    await page.fill(canon + ' .var-expr', 'page.url |');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.notice--error');
    check('variables : expression invalide refusée, rien n\'est écrit', JSON.stringify((await invoke('edit_open')).site) === docV, (await page.textContent('.notice--error')).trim());
    await page.click('.notice--error .icon-btn');
    await page.click(canon + ' .var-head');
    await page.fill(canon + ' .var-expr', '"https://essai.example/"');
    await page.keyboard.press('Enter');
    await page.waitForFunction((sel) => document.querySelector(sel + ' .var-value')?.textContent.trim() === 'https://essai.example/', canon);
    check('variables : expression modifiée, valeur recalculée', true);
    await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
    await page.keyboard.press('Control+z');
    await page.waitForFunction(([sel, v]) => document.querySelector(sel + ' .var-value')?.textContent.trim() === v, [canon, v0]);
    check('variables : Ctrl+Z, document identique', JSON.stringify((await invoke('edit_open')).site) === docV);
    // Échap dans l'expression : rien n'est appliqué.
    await page.click(canon + ' .var-head');
    await page.fill(canon + ' .var-expr', 'x');
    await page.keyboard.press('Escape');
    await page.waitForTimeout(200);
    check('variables : Échap abandonne la saisie', !(await page.$(canon + '.is-open')) && JSON.stringify((await invoke('edit_open')).site) === docV);
    // Médias : filtre par type, tri ; dossier créé, média rangé (glisser), renommé, supprimé.
    const lib = async () => (await invoke('edit_open')).site.library || {};
    await page.click('.dock-tab[data-panel="assets"]');
    await page.waitForSelector('.media .asset');
    const nAll = await page.$$eval('.media .asset', (a) => a.length);
    await page.selectOption('.media .mini-select >> nth=0 >> select', 'document');
    await page.waitForTimeout(150);
    const docs = await page.$$eval('.media .asset', (a) => a.map((x) => x.dataset.url));
    await page.selectOption('.media .mini-select >> nth=0 >> select', '');
    await page.selectOption('.media .mini-select >> nth=1 >> select', 'name');
    await page.waitForTimeout(150);
    const names = await page.$$eval('.media .asset figcaption', (a) => a.slice(0, 20).map((x) => x.textContent.toLowerCase()));
    check('médias : fichiers du site, filtre par type et tri par nom', nAll > 100 && docs.length > 0 && docs.every((u) => /\.pdf$/i.test(u)) && names.every((n, i) => i === 0 || names[i - 1] <= n), `${nAll} médias, ${docs.length} documents`);
    await page.selectOption('.media .mini-select >> nth=1 >> select', 'recent');
    await page.click('.media .panel-foot .icon-btn:not(.is-planned)');
    await page.keyboard.type('Logos');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.media .list-folder[data-folder="Logos"]');
    const firstUrl = await page.getAttribute('.media .asset >> nth=0', 'data-url');
    await page.dragAndDrop('.media .asset >> nth=0', '.media .list-folder[data-folder="Logos"]');
    await page.waitForFunction((u) => !document.querySelector(`.media .asset[data-url="${u}"]`), firstUrl);
    const l1 = await lib();
    check('médias : dossier créé, média glissé dedans (fichier inchangé)', JSON.stringify(l1.media) === JSON.stringify({ folders: ['Logos'], items: { [firstUrl]: 'Logos' } }), JSON.stringify(l1.media));
    await page.click('.media .list-folder[data-folder="Logos"]', { button: 'right' });
    await page.click('.ctx-menu .menu-item:has-text("Renommer")');
    await page.keyboard.press('Control+a');
    await page.keyboard.type('Marques');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.media .list-folder[data-folder="Marques"]');
    await page.click('.media .list-folder[data-folder="Marques"]');
    await page.waitForSelector(`.media .asset[data-url="${firstUrl}"]`);
    const crumbs = await page.$$eval('.media-crumb', (c) => c.map((x) => x.textContent));
    check('médias : dossier renommé, ouvert (fil d\'Ariane)', crumbs.join('|') === 'Médias|Marques' && (await lib()).media.items[firstUrl] === 'Marques', crumbs.join(' › '));
    await page.click('.media-crumb >> nth=0');
    await page.click('.media .list-folder[data-folder="Marques"]', { button: 'right' });
    await page.click('.ctx-menu .menu-item:has-text("Supprimer le dossier")');
    await page.waitForSelector(`.media .asset[data-url="${firstUrl}"]`);
    check('médias : dossier supprimé, média revenu à la racine', !(await lib()).media);
    // Médias : import (sélecteur, dépôt de fichiers du système), détails (texte alternatif par
    // défaut, remplacement), placement dans la page, choix d'une image dans les réglages.
    if (siteDir) {
      const until = async (fn, ms = 8000) => {
        const end = Date.now() + ms;
        while (Date.now() < end) {
          if (await fn()) return true;
          await page.waitForTimeout(100);
        }
        return false;
      };
      const PNG_A = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==', 'base64');
      const PNG_B = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');
      const mediaDir = path.join(siteDir, 'src/media');
      const essais = ['essai-import.png', 'essai-depot.png', 'essai-choix.png'];
      const docM = JSON.stringify((await invoke('edit_open')).site);
      const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.click('.media .panel-foot .icon-btn >> nth=1')]);
      await chooser.setFiles({ name: 'Essai Import.png', mimeType: 'image/png', buffer: PNG_A });
      await page.waitForSelector('.media .asset[data-url="/media/essai-import.png"]');
      check('médias : fichier importé par le sélecteur (nom simplifié, document inchangé)',
        fs.existsSync(path.join(mediaDir, 'essai-import.png')) && JSON.stringify((await invoke('edit_open')).site) === docM);
      await page.evaluate((b64) => {
        const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
        const dt = new DataTransfer();
        dt.items.add(new File([bytes], 'essai-depot.png', { type: 'image/png' }));
        const el = document.querySelector('.media .list-scroll');
        el.dispatchEvent(new DragEvent('dragover', { dataTransfer: dt, bubbles: true, cancelable: true }));
        el.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }));
      }, PNG_A.toString('base64'));
      await page.waitForSelector('.media .asset[data-url="/media/essai-depot.png"]');
      check('médias : fichier déposé depuis le système', fs.existsSync(path.join(mediaDir, 'essai-depot.png')));
      // Détails : texte alternatif par défaut, remplacement (même adresse, ancien à la corbeille).
      await page.click('.media .asset[data-url="/media/essai-import.png"]');
      await page.waitForSelector('.media-details .media-facts');
      await page.fill('.media-details .field input', 'Carré rouge');
      await page.keyboard.press('Enter');
      const altOk = await until(async () => (await lib()).alt?.['/media/essai-import.png'] === 'Carré rouge');
      if (shots) await page.screenshot({ path: `${shots}/editeur-media-details.png` });
      check('médias : texte alternatif par défaut enregistré', altOk);
      const [ch2] = await Promise.all([page.waitForEvent('filechooser'), page.click('.media-details .page-settings-foot .btn >> nth=0')]);
      await ch2.setFiles({ name: 'autre.png', mimeType: 'image/png', buffer: PNG_B });
      const replaced = await until(() => fs.readFileSync(path.join(mediaDir, 'essai-import.png')).equals(PNG_B));
      const trash = path.join(siteDir, '.revel/corbeille');
      check('médias : fichier remplacé (même adresse, ancien gardé à la corbeille)',
        replaced && fs.existsSync(trash) && fs.readdirSync(trash).some((f) => f.endsWith('-essai-import.png')));
      await page.click('.media-details .page-settings-head .icon-btn');
      await page.waitForFunction(() => !document.querySelector('.media-details'));
      // Glisser la vignette sur la page : image placée avec le texte alternatif par défaut.
      const tileBox = await page.locator('.media .asset[data-url="/media/essai-import.png"]').boundingBox();
      const frameBox = await page.locator('iframe.canvas-frame').boundingBox();
      await page.mouse.move(tileBox.x + tileBox.width / 2, tileBox.y + tileBox.height / 2);
      await page.mouse.down();
      for (let i = 1; i <= 12; i++) {
        await page.mouse.move(tileBox.x + (frameBox.x + frameBox.width / 2 - tileBox.x) * i / 12, tileBox.y + (frameBox.y + 300 - tileBox.y) * i / 12);
      }
      await page.mouse.up();
      const placed = () => invoke('edit_open').then((o) => JSON.stringify(o.site));
      const inserted = await until(async () => (await placed()).includes('"src":"\\"/media/essai-import.png\\"","alt":"\\"Carré rouge\\""'));
      check('médias : vignette glissée sur la page (image et texte alternatif par défaut)', inserted);
      // Réglages de l'image : choix parmi les médias, puis import depuis la fenêtre (choisi aussitôt).
      await page.click('.dock-tab[data-panel="settings"]');
      await page.click('.dock-panel[data-panel="settings"] .field-line .icon-btn');
      await page.waitForSelector('.picker .asset[data-url="/media/essai-depot.png"]');
      const onlyImages = await page.$$eval('.picker .asset', (a) => a.every((x) => /\.(png|jpe?g|gif|webp|avif|svg)$/i.test(x.dataset.url)));
      if (shots) await page.screenshot({ path: `${shots}/editeur-choix-image.png` });
      await page.click('.picker .asset[data-url="/media/essai-depot.png"]');
      const picked = await until(async () => (await placed()).includes('"src":"\\"/media/essai-depot.png\\""'));
      check('choix d\'une image : fenêtre (images seulement), image choisie, fenêtre fermée', onlyImages && picked && !(await page.$('.picker')));
      await page.click('.dock-panel[data-panel="settings"] .field-line .icon-btn');
      await page.waitForSelector('.picker .asset');
      const [ch3] = await Promise.all([page.waitForEvent('filechooser'), page.click('.picker-foot .btn')]);
      await ch3.setFiles({ name: 'essai-choix.png', mimeType: 'image/png', buffer: PNG_A });
      const pickedNew = await until(async () => (await placed()).includes('"src":"\\"/media/essai-choix.png\\""'));
      check('choix d\'une image : fichier importé depuis la fenêtre et choisi', pickedNew && fs.existsSync(path.join(mediaDir, 'essai-choix.png')) && !(await page.$('.picker')));
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      for (let i = 0; i < 4; i++) await page.keyboard.press('Control+z');
      const back = await until(async () => (await placed()) === docM);
      check('médias : Ctrl+Z (choix, placement, texte alternatif), document identique', back);
      for (const f of essais) fs.rmSync(path.join(mediaDir, f), { force: true });
    }
    // Composants : créer à partir de la sélection, ouvrir (modification isolée), relier un texte
    // à une propriété, valeur propre à une utilisation, glisser depuis le panneau, détacher, tout
    // annuler.
    {
      const frame = page.frameLocator('iframe.canvas-frame');
      const untilC = async (fn, ms = 8000) => {
        const end = Date.now() + ms;
        while (Date.now() < end) {
          if (await fn()) return true;
          await page.waitForTimeout(100);
        }
        return false;
      };
      const docC = JSON.stringify((await invoke('edit_open')).site);
      const site = async () => (await invoke('edit_open')).site;
      const comp = async () => (await site()).components.find((c) => c.id === 'bouton-pause');
      const usesOf = async () => JSON.stringify((await site()).pages).split('"component":"bouton-pause"').length - 1;
      await frame.locator('.hero__pause').click();
      await frame.locator('.hero__pause').click({ button: 'right' });
      await page.click('.ctx-menu .menu-item:has-text("Créer un composant")');
      await page.waitForSelector('.dialog input');
      await page.fill('.dialog input', 'Bouton pause');
      await page.keyboard.press('Enter');
      await untilC(async () => !!(await comp()));
      await page.click('.dock-tab[data-panel="settings"]');
      await page.waitForSelector('.instance-name');
      const made = await comp();
      check('composants : créé à partir de la sélection, son utilisation sélectionnée',
        made && made.name === 'Bouton pause' && made.body[0].tag === 'button' && (await usesOf()) === 1
          && (await page.textContent('.instance-name')).trim() === 'Bouton pause');
      await frame.locator('.hero__pause').dblclick();
      await page.waitForSelector('.iso-bar');
      await page.click('.bind-row[data-bind="component-bind-text"] .bind-btn');
      await page.click('.ctx-menu .menu-item >> nth=0');
      await untilC(async () => ((await comp()).props || []).length === 1);
      const bound = await comp();
      if (shots) await page.screenshot({ path: `${shots}/editeur-composant-ouvert.png` });
      check('composants : ouvert (reste estompé), texte relié à une nouvelle propriété (valeur par défaut : le texte)',
        bound.props[0].name === 'texte' && bound.props[0].default === 'Mettre la vidéo en pause'
          && bound.body[0].children[0].expr === 'texte' && !!(await page.$('.iso-bar')));
      await page.click('.iso-bar .iso-done');
      await page.waitForFunction(() => !document.querySelector('.iso-bar'));
      await frame.locator('.hero__pause').click();
      await page.waitForSelector('.prop-row[data-prop="texte"] input');
      await page.fill('.prop-row[data-prop="texte"] input', 'Pause');
      await page.keyboard.press('Enter');
      const own = await untilC(async () => ((await frame.locator('.hero__pause').first().textContent()) || '').trim() === 'Pause');
      check('composants : valeur propre à une utilisation', own);
      // Glisser depuis le panneau Composants : nouvelle utilisation, valeur par défaut.
      await page.click('.dock-tab[data-panel="components"]');
      const rowSel = '.components .component-row[data-component="bouton-pause"]';
      await page.waitForSelector(rowSel);
      const rb = await page.locator(rowSel).boundingBox();
      const fb = await page.locator('iframe.canvas-frame').boundingBox();
      await page.mouse.move(rb.x + 30, rb.y + rb.height / 2);
      await page.mouse.down();
      for (let i = 1; i <= 12; i++) {
        await page.mouse.move(rb.x + 30 + (fb.x + fb.width / 2 - rb.x - 30) * i / 12, rb.y + (fb.y + 320 - rb.y) * i / 12);
      }
      await page.mouse.up();
      const two = await untilC(async () => (await usesOf()) === 2 && (await frame.locator('.hero__pause').count()) === 2);
      const texts = await frame.locator('.hero__pause').allTextContents();
      check('composants : glissé depuis le panneau, deuxième utilisation (valeur par défaut)',
        two && texts.map((t) => t.trim()).sort().join('|') === 'Mettre la vidéo en pause|Pause', texts.join(' | '));
      // Détacher la première : copie modifiable séparément, valeur écrite.
      await frame.locator('.hero__pause', { hasText: /^\s*Pause\s*$/ }).click({ button: 'right' });
      await page.click('.ctx-menu .menu-item:has-text("Détacher")');
      const detached = await untilC(async () => (await usesOf()) === 1);
      check('composants : utilisation détachée (copie avec sa valeur)', detached
        && JSON.stringify((await site()).pages).includes('"text":"Pause"'));
      if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-composants-panneau.png` });
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      for (let i = 0; i < 5; i++) {
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(250);
      }
      const backC = await untilC(async () => JSON.stringify(await site()) === docC);
      check('composants : Ctrl+Z (détacher, glisser, valeur, propriété, création), document identique', backC);
    }
    // Éléments (3.7) : liste de collection (tri, nombre, texte relié à un champ), grille avec ses
    // styles, icône, code HTML ; tout annulé.
    {
      const frame = page.frameLocator('iframe.canvas-frame');
      const untilE = async (fn, ms = 8000) => {
        const end = Date.now() + ms;
        while (Date.now() < end) {
          if (await fn()) return true;
          await page.waitForTimeout(100);
        }
        return false;
      };
      const siteE = async () => (await invoke('edit_open')).site;
      const docE = JSON.stringify(await siteE());
      const pageJson = async () => JSON.stringify((await siteE()).pages);
      await page.click('.dock-tab[data-panel="layers"]');
      await page.click('.layers .layer-label:has-text("section.intro-agency")');
      await page.click('.dock-tab[data-panel="insert"]');
      await page.click('.insert-item[data-item="insert-collection-list"]');
      const listed = await untilE(async () => (await frame.locator('.liste-collection__element').count()) > 2);
      await page.click('.dock-tab[data-panel="settings"]');
      await page.waitForSelector('.collection-list select');
      await page.selectOption('.collection-list select >> nth=2', 'date');
      await untilE(async () => (await pageJson()).includes('sort(attribute=\\"date\\", reverse=true)'));
      await page.fill('.collection-list input[type=number]', '2');
      await page.keyboard.press('Tab');
      const two = await untilE(async () => (await frame.locator('.liste-collection__element').count()) === 2);
      check('éléments : liste de collection (collection, tri par date décroissante, 2 contenus)', listed && two
        && (await pageJson()).includes('[:2]'));
      await frame.locator('.liste-collection__element h3').first().click();
      await page.waitForSelector('[data-bound="block-bound-text-label"] select');
      await page.waitForFunction(() => document.querySelectorAll('[data-bound="block-bound-text-label"] select option').length > 2);
      const boundTitle = await page.inputValue('[data-bound="block-bound-text-label"] select');
      const opts = await page.$$eval('[data-bound="block-bound-text-label"] select option', (o) => o.map((x) => x.value));
      const other = opts.find((v) => v && v !== 'title');
      await page.selectOption('[data-bound="block-bound-text-label"] select', other);
      const rebound = await untilE(async () => (await pageJson()).includes(`"expr":"item.data.${other}"`));
      check('éléments : texte d\'un élément de la liste relié à un champ du contenu', boundTitle === 'title' && rebound, `${boundTitle} -> ${other}`);
      if (shots) await page.screenshot({ path: `${shots}/editeur-liste-collection.png` });
      // Grille : élément et ses règles de style.
      await page.click('.dock-tab[data-panel="layers"]');
      await page.click('.layers .layer-label:has-text("section.intro-agency")');
      await page.click('.dock-tab[data-panel="insert"]');
      await page.click('.insert-item[data-item="insert-grid"]');
      const grid = await untilE(async () => (await frame.locator('.grille .grille__cellule').count()) === 2);
      const gridCss = await frame.locator('.grille').evaluate((g) => getComputedStyle(g).display);
      check('éléments : grille insérée avec ses styles', grid && gridCss === 'grid', gridCss);
      // Icône : dessin choisi dans les réglages.
      await page.click('.insert-item[data-item="insert-icon"]');
      await page.click('.dock-tab[data-panel="settings"]');
      await page.waitForSelector('.icon-choice[data-icon="coeur"]');
      await page.click('.icon-choice[data-icon="coeur"]');
      const heart = await untilE(async () => (await frame.locator('svg.icone[data-icon="coeur"]').count()) === 1);
      check('éléments : icône insérée, dessin changé', heart);
      // Code HTML écrit tel quel.
      await page.click('.dock-tab[data-panel="insert"]');
      await page.click('.insert-item[data-item="insert-embed"]');
      await page.click('.dock-tab[data-panel="settings"]');
      await page.waitForSelector('.embed-code');
      await page.fill('.embed-code', '<p class="essai-code">Bonjour</p>');
      await page.click('.dock-panel[data-panel="settings"] .panel-title >> nth=0');
      const embed = await untilE(async () => (await frame.locator('.code-html .essai-code').count()) === 1);
      check('éléments : code HTML écrit tel quel', embed);
      // Bloc prêt à l'emploi : section complète avec ses icônes et ses styles.
      await page.click('.dock-tab[data-panel="layers"]');
      await page.click('.layers .layer-label:has-text("section.intro-agency")');
      await page.click('.dock-tab[data-panel="insert"]');
      await page.locator('.insert-item[data-item="insert-block-features"]').scrollIntoViewIfNeeded();
      await page.click('.insert-item[data-item="insert-block-features"]');
      const block = await untilE(async () => (await frame.locator('.bloc-avantages .bloc-avantage .icone').count()) === 3);
      const blockCss = block && await frame.locator('.bloc-avantages__grille').evaluate((g) => getComputedStyle(g).display);
      check('éléments : bloc prêt à l\'emploi (avantages, icônes, styles)', block && blockCss === 'grid', String(blockCss));
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      for (let i = 0; i < 12; i++) {
        if (JSON.stringify(await siteE()) === docE) break;
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(300);
      }
      check('éléments : Ctrl+Z, document identique', await untilE(async () => JSON.stringify(await siteE()) === docE));
    }
    // Composants : dossier, rangement par le menu contextuel.
    await page.click('.dock-tab[data-panel="components"]');
    await page.waitForSelector('.components .component-row');
    await page.click('.components .panel-foot .icon-btn:not(.is-planned)');
    await page.keyboard.type('Cartes');
    await page.keyboard.press('Enter');
    await page.waitForSelector('.components .list-folder[data-folder="Cartes"]');
    await page.click('.components .component-row[data-component="partials/card-project"]', { button: 'right' });
    await page.click('.ctx-menu .menu-item:has-text("Cartes")');
    await page.waitForSelector('.components .list-folder[data-folder="Cartes"] + .component-row[data-component="partials/card-project"]');
    await page.click('.components .list-folder[data-folder="Cartes"]');
    await page.waitForTimeout(150);
    const hiddenRow = !(await page.$('.components .component-row[data-component="partials/card-project"]'));
    await page.click('.components .list-folder[data-folder="Cartes"]');
    check('composants : dossier créé, composant rangé, dossier replié', hiddenRow && (await lib()).components.items['partials/card-project'] === 'Cartes');
    if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-composants.png` });
    for (let i = 0; i < 6; i++) {
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      await page.keyboard.press('Control+z');
      await page.waitForTimeout(120);
    }
    await page.waitForFunction(async () => true);
    check('rangement : tout annulé, document identique', JSON.stringify((await invoke('edit_open')).site) === docV);
    // Pages : pages fixes et collections (modèle) ; contenu affiché sous sa collection repliée ;
    // design propre créé puis retiré (fichier du contenu, .pages.yml), annulé ; menu des pages
    // dans la barre d'outils quand le panneau Pages est fermé.
    await page.click('.dock-tab[data-panel="pages"]');
    await page.waitForSelector('.pages .collection-row[data-collection="projets"]');
    const entriesListed = await page.$$eval('.pages .entry-row', (r) => r.length);
    await page.click('.pages .collection-row[data-collection="projets"]');
    await page.waitForSelector('.pages .collection-row[data-collection="projets"].is-current');
    const entryUrl0 = await page.$eval('.pages .entry-row.is-on', (r) => r.dataset.url);
    const shownRows = await page.$$eval('.pages .entry-row', (r) => r.length);
    check('pages : collections (modèles) sans leurs contenus ; contenu affiché sous sa collection repliée', entriesListed === 0 && shownRows === 1 && !(await page.$('.page-pick')), entryUrl0);
    await waitFrame(() => document.readyState === 'complete' && !!document.querySelector('[data-rv]'), 'contenu');
    const pinfo = (await invoke('edit_pages')).find((p) => p.url === entryUrl0);
    const colFolder = (await invoke('edit_open')).site.collections.find((c) => c.id === pinfo.collection).folder;
    const entryFile = path.join(siteDir, colFolder, pinfo.file + '.md');
    const entry0 = fs.readFileSync(entryFile, 'utf8');
    const pagesYml = path.join(siteDir, '.pages.yml');
    const yml0 = fs.existsSync(pagesYml) ? fs.readFileSync(pagesYml, 'utf8') : null;
    const docP = JSON.stringify((await invoke('edit_open')).site);
    check('pages : modèle de la collection signalé dans la barre d\'outils', !!(await page.$('.design-chip:not(.is-own)')));
    await page.click('.design-chip .icon-btn');
    await page.waitForSelector('.design-chip.is-own', { timeout: 15000 });
    const own = (await invoke('edit_pages')).find((p) => p.url === entryUrl0).layout;
    const entry1 = fs.readFileSync(entryFile, 'utf8');
    const yml1 = yml0 === null ? null : fs.readFileSync(pagesYml, 'utf8');
    check('pages : design propre (copie du modèle, clé layout du contenu, champ caché dans .pages.yml)',
      /--/.test(own) && entry1.includes('layout: ' + own) && !!(await invoke('edit_open')).site.layouts.find((l) => l.id === own)
        && (yml0 === null || yml1.includes('name: layout')) && !!(await page.$('.pages .entry-row.is-on .list-badge')), own);
    await page.click('.design-chip .icon-btn');
    await page.waitForSelector('.design-chip:not(.is-own)', { timeout: 15000 });
    check('pages : retour au modèle (clé et gabarit retirés)', !fs.readFileSync(entryFile, 'utf8').includes('layout:') && !(await invoke('edit_open')).site.layouts.find((l) => l.id === own));
    for (let i = 0; i < 2; i++) {
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      await page.keyboard.press('Control+z');
      await page.waitForTimeout(400);
    }
    await page.waitForFunction(() => !!document.querySelector('.design-chip:not(.is-own)'));
    check('pages : annulé, contenu, .pages.yml et document identiques', fs.readFileSync(entryFile, 'utf8') === entry0
      && (yml0 === null || fs.readFileSync(pagesYml, 'utf8') === yml0) && JSON.stringify((await invoke('edit_open')).site) === docP);
    await page.click('.dock-tab[data-panel="pages"]', { button: 'right' });
    await page.click('.ctx-menu .menu-item:has-text("Fermer")');
    await page.click('.page-pick > summary');
    await page.waitForSelector('.page-pop .page-row');
    await page.click('.page-pop .page-row[data-url="/"]');
    await page.waitForFunction(() => !document.querySelector('.page-pick[open]'));
    await waitFrame(() => document.readyState === 'complete' && location.pathname === '/', 'accueil');
    check('pages : menu des pages dans la barre d\'outils (panneau fermé)', !(await page.$('.design-chip')));
    if (shots) await page.screenshot({ path: `${shots}/editeur-pages-menu.png` });
    await page.click('.panels-menu > summary');
    await page.click('.panels-pop .menu-item--check:has-text("Pages")');
    await page.click('.panels-menu > summary');
    await page.waitForFunction(() => !document.querySelector('.page-pick'));
    // Pages (3.4) : nouvelle page ouverte, paramètres (référencement, adresse avec redirection,
    // brouillon), dossier, page rangée ; référencement d'une page lu dans un fichier de données ;
    // tout annulé (document et fichiers identiques).
    {
      const doc34 = JSON.stringify((await invoke('edit_open')).site);
      const agenceYml = path.join(siteDir, 'src/_data/pages/agence.yml');
      const agence0 = fs.readFileSync(agenceYml, 'utf8');
      const site34 = () => invoke('edit_open').then((o) => o.site);
      await page.click('.dock-tab[data-panel="pages"]');
      await page.click('.pages .panel-foot .icon-btn[title="Nouvelle page"]');
      await page.keyboard.type('Offres d\'été');
      await page.keyboard.press('Enter');
      await page.waitForSelector('.pages .page-row[data-url="/offres-d-ete/"]');
      await waitFrame(() => location.pathname === '/offres-d-ete/' && !!document.querySelector('h1'), 'nouvelle page');
      const np = (await site34()).pages.find((p) => p.path === '/offres-d-ete/');
      check('pages : nouvelle page (gabarit du site, titre, ouverte)', !!np && np.layout === 'layouts/base' && np.name === 'Offres d\'été');
      await page.hover('.pages .page-row[data-url="/offres-d-ete/"]');
      await page.click('.pages .page-row[data-url="/offres-d-ete/"] .row-action');
      await page.waitForSelector('.page-settings');
      const seoTitle = page.locator('.page-settings-seo .field').nth(0).locator('input');
      await seoTitle.fill('Nos offres');
      await seoTitle.press('Enter');
      await page.waitForFunction(() => document.querySelector('.serp-title')?.textContent === 'Nos offres');
      const slugIn = page.locator('.path-input input');
      await slugIn.fill('emplois-ete');
      await slugIn.blur();
      await waitFrame(() => location.pathname === '/emplois-ete/', 'adresse changée');
      await page.waitForTimeout(300);
      let s34 = await site34();
      const moved = s34.pages.find((p) => p.id === np.id);
      check('pages : référencement et adresse (redirection de l\'ancienne)', moved.props.seo.title === 'Nos offres' && moved.path === '/emplois-ete/'
        && s34.redirects.some((r) => r.from === '/offres-d-ete/' && r.to === '/emplois-ete/'));
      await page.click('.page-settings .view-line:has-text("Brouillon") .switch');
      await page.waitForTimeout(600);
      check('pages : brouillon', (await site34()).pages.find((p) => p.id === np.id).draft === true);
      await page.click('.page-settings-head .icon-btn');
      await page.click('.pages .panel-foot .icon-btn[title="Nouveau dossier"]');
      await page.keyboard.type('Recrutement');
      await page.keyboard.press('Enter');
      await page.waitForSelector('.pages .list-folder[data-folder="recrutement"]');
      await page.click('.pages .page-row[data-url="/emplois-ete/"]', { button: 'right' });
      await page.click('.ctx-menu .menu-item:has-text("recrutement")');
      await page.waitForFunction(() => !!document.querySelector('.pages .page-row[data-url="/recrutement/emplois-ete/"]'));
      check('pages : dossier créé, page rangée (adresse dans le dossier)', (await site34()).pages.some((p) => p.path === '/recrutement/emplois-ete/'));
      if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-pages-dossier.png` });
      // Page dont le référencement vient d'un fichier de données (site importé).
      await page.hover('.pages .page-row[data-url="/agence/"]');
      await page.click('.pages .page-row[data-url="/agence/"] .row-action');
      await page.waitForSelector('.page-settings-note');
      const desc = page.locator('.page-settings-seo textarea');
      await desc.fill('Agence de communication plurielle, essai.');
      await desc.blur();
      await page.waitForFunction(() => document.querySelector('.serp-desc')?.textContent === 'Agence de communication plurielle, essai.');
      const agence1 = fs.readFileSync(agenceYml, 'utf8');
      check('pages : référencement écrit dans le fichier de données de la page', agence1 !== agence0 && agence1.includes('Agence de communication plurielle, essai.'));
      if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-page-parametres.png` });
      await page.click('.page-settings-head .icon-btn');
      for (let i = 0; i < 7; i++) {
        await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(350);
      }
      await page.waitForTimeout(800);
      check('pages : tout annulé, document et fichier de données identiques', JSON.stringify(await site34()) === doc34 && fs.readFileSync(agenceYml, 'utf8') === agence0);
      await waitFrame(() => location.pathname === '/' || !!document.querySelector('[data-rv]'), 'retour');
      await page.click('.pages .page-row[data-url="/"]');
    }
    await page.click('.dock-tab[data-panel="layers"]');
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
    await page.locator('.insert-item[data-item="insert-paragraph"]').scrollIntoViewIfNeeded();
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
    await page.waitForSelector('.ctx-menu');
    check('clic droit dans le canevas : menu de l\'éditeur', true);
    if (shots) await page.screenshot({ path: `${shots}/editeur-menu-canevas.png` });
    await page.click('.ctx-menu .menu-item:has-text("Copier")');
    await page.waitForFunction(() => !document.querySelector('.ctx-menu'));
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
    // Alt+molette hors du cadre, juste après un défilement : zoom, action du navigateur empêchée.
    await page.evaluate(() => { window.__wheel = []; document.addEventListener('wheel', (e) => window.__wheel.push(e.defaultPrevented)); });
    const s0 = await page.$eval('iframe.canvas-frame', (e) => e.style.transform);
    await page.keyboard.down('Alt');
    await page.mouse.wheel(0, -200);
    await page.keyboard.up('Alt');
    await page.waitForFunction((t) => document.querySelector('iframe.canvas-frame').style.transform !== t, s0);
    check('Alt+molette hors du cadre : zoom', (await page.evaluate(() => window.__wheel)).every(Boolean), await page.$eval('iframe.canvas-frame', (e) => e.style.transform));
    await page.keyboard.press('Shift+Digit1');
    await page.waitForFunction(() => document.querySelector('.editor-zoom select').value === '');
    // Règles (Ctrl+R) et guides de colonnes.
    await page.mouse.click(area.x + area.width - 6, area.y + area.height - 10);
    await page.keyboard.press('Control+r');
    await page.waitForSelector('.canvas-rulers canvas.ruler--top');
    await page.click('.view-menu summary');
    await page.click('.view-menu .view-line:nth-child(2) .switch');
    await page.fill('.view-menu .view-guides .view-line:nth-child(1) input', '4');
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => document.querySelectorAll('.column-guide').length === 4);
    await page.mouse.click(area.x + area.width - 6, area.y + area.height - 10);
    check('règles (Ctrl+R), guides de colonnes, menu fermé au clic ailleurs', !(await page.$('.view-menu[open]')));
    await page.keyboard.press('Control+r');
    await page.click('.view-menu summary');
    await page.click('.view-menu .view-line:nth-child(2) .switch');
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('.canvas-rulers') && !document.querySelector('.column-guide') && !document.querySelector('.view-menu[open]'));
    // Aperçu dans l'espace de travail, à la page affichée ; Échap le ferme.
    await page.click('.preview-split .split-main');
    await page.waitForSelector('.preview-pane iframe.preview-frame');
    const psrc = await page.getAttribute('.preview-frame', 'src');
    const pframe = await (await page.$('.preview-frame')).contentFrame();
    await pframe.waitForLoadState();
    const ptitle = await pframe.title();
    await page.click('.preview-pane .segmented button:nth-child(3)');
    await page.waitForFunction(() => Math.abs(document.querySelector('.preview-frame').getBoundingClientRect().width - 390) < 2).catch(() => {});
    const pw = await page.$eval('.preview-frame', (f) => f.getBoundingClientRect().width);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('.preview-pane'));
    check('aperçu dans l\'espace de travail (page affichée, largeur mobile, Échap)', psrc.includes('/__preview/') && ptitle.length > 0 && Math.abs(pw - 390) < 2, `${psrc} « ${ptitle} » ${pw}`);

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
    await page.click('.dock-tab[data-panel="pages"]');
    const entryUrl = await (await page.waitForSelector('.pages .entry-row.is-on')).getAttribute('data-url');
    f = await waitFrame((u) => location.pathname === u && !!window.__revelSlot, 'page du contenu', entryUrl);
    check('vue CMS : « Ouvrir la page » affiche le contenu dans l\'éditeur', entryUrl !== '/', entryUrl);
    await page.click('.pages .page-row[data-url="/"]');
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
  // Fin du glissement du cadre vers la nouvelle largeur.
  await page.waitForFunction(() => !document.querySelector('.canvas-area.is-resizing'));
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
  await page.click('.bp-edit[data-bp="max-480"] .icon-btn');
  await page.waitForFunction(() => ![...document.querySelectorAll('.editor-toolbar .segmented .bp-width')].some((e) => e.textContent === '480'));
  // Largeur modifiée dans sa case : les règles du point de rupture suivent ; annulée.
  await page.fill('.bp-edit[data-bp="max-600"] input', '620');
  await page.press('.bp-edit[data-bp="max-600"] input', 'Enter');
  await page.waitForFunction(() => [...document.querySelectorAll('.editor-toolbar .segmented .bp-width')].some((e) => e.textContent === '620'));
  const site620 = (await invoke('edit_open')).site;
  const media620 = rulesOf(site620, '.hero__title').map(([it]) => it.media).filter(Boolean);
  check('points de rupture : largeur modifiée, règles déplacées avec elle', media620.includes('(max-width: 620px)') && !media620.includes('(max-width: 600px)'), media620.join(' ; '));
  await page.click('.editor-toolbar .btn-undo');
  await page.waitForFunction(() => [...document.querySelectorAll('.editor-toolbar .segmented .bp-width')].some((e) => e.textContent === '600'));
  await page.click('.bp-manage > summary');
  await page.click('.bp-edit[data-bp="max-600"] .icon-btn');
  await page.waitForSelector('.notice--error');
  check('points de rupture : retrait refusé quand des styles l\'utilisent', JSON.stringify((await invoke('edit_open')).site.breakpoints).includes('max-600'));
  await page.click('.notice--error .icon-btn');
  check('points de rupture : menu fermé par un clic ailleurs', !(await page.$('.bp-menu[open]')));
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
    const row = (name) => `.cms-item-body > [data-field="${name}"]`;
    await page.waitForSelector(row('title') + ' input');
    await page.fill(row('title') + ' input', 'Titre essai CMS');
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
    // Blocs (3.7) : une carte par bloc (type, champs), ordre changé, bloc ajouté, annulés.
    const blocksRow = row('blocks');
    await page.waitForSelector(`${blocksRow} .cms-repeat-item`);
    const types = () => (fs.readFileSync(file, 'utf8').match(/^- type: \w+/gm) || []).map((l) => l.slice(8));
    const types0 = types();
    const cards = await page.$$eval(`${blocksRow} .cms-repeat-title`, (t) => t.map((x) => x.textContent));
    await page.click(`${blocksRow} .cms-repeat-item[data-index="0"] .cms-repeat-head .icon-btn >> nth=1`);
    await until(() => types()[0] === types0[1] && types()[1] === types0[0], 'bloc descendu');
    await page.click(`${blocksRow} .cms-repeat-add`);
    await page.click('.ctx-menu .menu-item:has-text("Texte")');
    await until(() => types().length === types0.length + 1 && types().at(-1) === 'text', 'bloc ajouté');
    check('CMS : blocs en cartes (type et champs), ordre changé, bloc ajouté',
      cards.length === types0.length && cards[0] === 'Citation', `${cards.length} blocs : ${cards.slice(0, 3).join(', ')}`);
    for (let i = 0; i < 2; i++) {
      await page.click('.cms-head .icon-btn[title^="Annuler"]');
      await page.waitForTimeout(300);
    }
    await until(() => fs.readFileSync(file, 'utf8') === original, 'annulation des blocs');
    check('CMS : blocs annulés, fichier identique', true);

    // ---------------------------------------------------------------- Schéma de Pages CMS (3.3)
    const heads = await page.$$eval('.cms-table thead th', (ths) => ths.map((t) => t.textContent.trim()));
    check('CMS : nom, colonnes et tri du schéma (.pages.yml)',
      (await page.textContent('.cms-col[data-collection="projets"] .list-label')) === 'Projets'
        && heads[0] === 'Titre' && heads[1] === 'Client' && heads[2] === 'Date↓' && heads[3] === 'Publié', heads.join(' | '));
    const newest = fs.readdirSync(dir).filter((f) => f.endsWith('.md'))
      .map((f) => [f.slice(0, -3), (fs.readFileSync(path.join(dir, f), 'utf8').match(/^date: '?([^'\n]+)/m) || [])[1] || ''])
      .sort((a, b) => b[1].localeCompare(a[1]))[0][0];
    check('CMS : tri par date décroissante', first === newest, `${first} / ${newest}`);
    // Champ obligatoire vidé, adresse hors motif : refusés, rien n'est écrit.
    check('CMS : champ obligatoire marqué', !!(await page.$(row('title') + ' .cms-req')));
    const title0 = await page.inputValue(row('title') + ' input');
    await page.fill(row('title') + ' input', '');
    await page.keyboard.press('Tab');
    await page.waitForSelector(row('title') + ' .field-error');
    const slug0 = await page.inputValue(row('slug') + ' input');
    await page.fill(row('slug') + ' input', 'Pas Valide');
    await page.keyboard.press('Tab');
    await page.waitForSelector(row('slug') + ' .field-error');
    await page.waitForTimeout(300);
    check('CMS : valeur obligatoire vide et motif non respecté refusés', fs.readFileSync(file, 'utf8') === original);
    await page.fill(row('title') + ' input', title0);
    await page.fill(row('slug') + ' input', slug0);
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => !document.querySelector('.cms-item .field-error'));
    // Date avec heure, écrite au format du schéma.
    const dateIn = row('date') + ' input';
    const d0 = await page.inputValue(dateIn);
    check('CMS : date et heure lues au format du schéma', (await page.getAttribute(dateIn, 'type')) === 'datetime-local' && /^\d{4}-\d\d-\d\dT\d\d:\d\d$/.test(d0), d0);
    await page.fill(dateIn, '2026-03-20T08:15');
    await page.click('.cms-item-head h2');
    await until(() => /^date: '?2026-03-20T08:15:00'?$/m.test(fs.readFileSync(file, 'utf8')), 'date écrite');
    check('CMS : date écrite au format du schéma', true);
    // Références : libellés des contenus liés, ajout depuis la liste.
    const refs = await page.$$eval(row('expertises') + ' .chip--tag', (cs) => cs.map((c) => c.textContent.trim()));
    const refValues = await page.$$eval(row('expertises') + ' .chip--tag', (cs) => cs.map((c) => c.title));
    check('CMS : références affichées par leur titre', refs.length > 0 && refs.every((r, i) => r !== refValues[i]), refs.join(', '));
    const option = await page.$eval(row('expertises') + ' .tags-pick option:nth-child(2)', (o) => o.value);
    await page.selectOption(row('expertises') + ' .tags-pick', option);
    await until(() => fs.readFileSync(file, 'utf8').includes('- ' + option + '\n'), 'référence ajoutée');
    check('CMS : référence ajoutée', true);
    check('CMS : objet (référencement) en sous-fiche, blocs en cartes, champ caché replié',
      !!(await page.$('.cms-item-body > .cms-object[data-field="seo"] legend'))
        && !!(await page.$(row('blocks') + ' .cms-repeat-item'))
        && !!(await page.$('.cms-more [data-field="wp_id"]')));
    if (shots) await page.screenshot({ path: `${shots}/cms-schema.png` });
    for (let i = 0; i < 2; i++) {
      await page.click('.cms-head .icon-btn[title^="Annuler"]');
      await page.waitForTimeout(300);
    }
    await until(() => fs.readFileSync(file, 'utf8') === original, 'annulation (schéma)');
    check('CMS : date et référence annulées, fichier identique', true);
    // Nouveau contenu, puis supprimé ; annulé : dossier identique.
    await page.click('.cms-new');
    await page.fill('.dialog input', 'Essai Revel');
    await page.press('.dialog input', 'Enter');
    const created = path.join(dir, 'essai-revel.md');
    await until(() => fs.existsSync(created), 'contenu créé');
    await page.waitForSelector('.cms-item[data-entry="essai-revel"]');
    check('CMS : nouveau contenu créé et ouvert, valeurs du schéma (adresse, publié, date du jour)',
      /^---\ntitle: Essai Revel\nslug: essai-revel\npublished: true\ndate: '?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d'?\n/.test(fs.readFileSync(created, 'utf8')),
      fs.readFileSync(created, 'utf8').split('\n').slice(0, 5).join(' | '));
    await page.click('.cms-item-foot .icon-btn');
    await page.click('.dialog .btn--danger');
    await until(() => !fs.existsSync(created), 'contenu supprimé');
    check('CMS : contenu supprimé', true);
    await page.keyboard.press('Control+z');
    await until(() => fs.existsSync(created), 'suppression annulée (Ctrl+Z)');
    await page.keyboard.press('Control+z');
    await until(() => !fs.existsSync(created) && listing() === list0, 'création annulée');
    check('CMS : Ctrl+Z rétablit puis retire le contenu, dossier identique', true);

    // ---------------------------------------------------------------- Champs de la collection (3.3)
    const cfgPath = path.join(siteDir, '.pages.yml');
    const cfg0 = fs.readFileSync(cfgPath, 'utf8');
    const cfg = () => fs.readFileSync(cfgPath, 'utf8');
    const lineDiff = (a, b) => { const x = a.split('\n'), y = b.split('\n'); return y.filter((l, i) => l !== x[i]); };
    await page.click('.cms-fields-btn');
    await page.waitForSelector('.schema-row[data-field="client"]');
    const rows0 = await page.$$eval('.schema-row', (rs) => rs.map((r) => r.dataset.field));
    check('champs : liste du schéma', rows0.length === 17 && rows0[0] === 'title', rows0.join(','));
    await page.click('.schema-row[data-field="client"]');
    await page.waitForSelector('.schema-item[data-field="client"]');
    await page.fill('.schema-item [data-setting="label"] input', 'Client (marque)');
    await page.keyboard.press('Tab');
    await until(() => cfg().includes('label: Client (marque)'), 'libellé écrit');
    check('champs : libellé modifié, seule sa ligne change dans .pages.yml', lineDiff(cfg0, cfg()).length === 1, lineDiff(cfg0, cfg()).join(' | '));
    await page.click('.schema-item [data-setting="required"] .switch');
    await until(() => /- name: client\n {8}label: Client \(marque\)\n {8}type: string\n {8}required: true\n/.test(cfg()), 'obligatoire');
    check('champs : rendu obligatoire', true);
    // Ordre : flèche, puis glisser la poignée au-dessus du premier champ.
    await page.click('.schema-item .icon-btn[title="Monter"]');
    await until(() => cfg().indexOf('- name: client') < cfg().indexOf('- name: date'), 'monté');
    const uiOrder = () => page.$$eval('.schema-row', (rs) => rs.map((r) => r.dataset.field).join(','));
    await page.waitForFunction(() => [...document.querySelectorAll('.schema-row')].map((r) => r.dataset.field).indexOf('client') === 3);
    const grip = await page.$('.schema-row[data-field="client"] .schema-grip');
    const gb = await grip.boundingBox();
    const top = await (await page.$('.schema-row[data-field="title"]')).boundingBox();
    await page.mouse.move(gb.x + gb.width / 2, gb.y + gb.height / 2);
    await page.mouse.down();
    await page.mouse.move(gb.x + gb.width / 2, top.y + 10, { steps: 8 });
    await page.waitForSelector('.schema-row.is-drop-before[data-field="title"]');
    await page.mouse.up();
    await until(() => /fields:\n {6}- name: client\n/.test(cfg()), 'glissé en tête');
    await page.waitForFunction(() => document.querySelector('.schema-row')?.dataset.field === 'client');
    const rows1 = (await uiOrder()).split(',');
    check('champs : ordre changé (flèche, glisser)', rows1[0] === 'client' && rows1[1] === 'title', rows1.slice(0, 3).join(','));
    // Nouveau champ (choix), un choix ajouté, puis retiré.
    await page.click('.new-field .btn');
    await page.click('.new-field-menu [data-type="select"]');
    await page.fill('.dialog input', 'Couleur dominante');
    await page.press('.dialog input', 'Enter');
    await until(() => cfg().includes('- name: couleur_dominante\n'), 'nouveau champ');
    await page.waitForSelector('.schema-item[data-field="couleur_dominante"]');
    await page.fill('.schema-item [data-setting="values"] .tags-add', 'Bleu nuit');
    await page.press('.schema-item [data-setting="values"] .tags-add', 'Enter');
    await until(() => /options:\n {10}values:\n {12}- name: bleu-nuit\n {14}label: Bleu nuit\n/.test(cfg()), 'choix ajouté');
    check('champs : nouveau champ « Choix » et sa valeur', true);
    await page.click('.schema-item .cms-item-foot .btn--danger-ghost');
    await page.click('.dialog .btn--danger');
    await until(() => !cfg().includes('couleur_dominante'), 'champ retiré');
    check('champs : champ retiré', true);
    // Groupe de champs (3.7) : créé avec un premier champ, un champ ajouté, puis retiré.
    await page.click('.new-field .btn');
    await page.click('.new-field-menu [data-type="object"]');
    await page.fill('.dialog input', 'Galerie');
    await page.press('.dialog input', 'Enter');
    await until(() => /- name: galerie\n {8}label: Galerie\n {8}type: object\n {8}fields:\n {10}- name: texte\n/.test(cfg()), 'groupe créé');
    await page.waitForSelector('.schema-item [data-setting="fields"] .schema-sub-add');
    await page.click('.schema-item [data-setting="fields"] .schema-sub-add');
    await page.fill('.dialog input', 'Légende');
    await page.press('.dialog input', 'Enter');
    await until(() => /- name: legende\n {12}label: Légende\n {12}type: string\n/.test(cfg()), 'champ du groupe ajouté');
    check('champs : groupe de champs créé, champ du groupe ajouté', true);
    await page.click('.schema-item .cms-item-foot .btn--danger-ghost');
    await page.click('.dialog .btn--danger');
    await until(() => !cfg().includes('galerie'), 'groupe retiré');
    if (shots) await page.screenshot({ path: `${shots}/cms-champs.png` });
    // Fiche d'un contenu : le nouveau libellé et l'ordre du schéma.
    await page.click('.cms-fields-btn');
    await page.click(`.cms-table tbody tr[data-entry="${first}"]`);
    await page.waitForSelector('.cms-item-body > [data-field="client"] .cms-req');
    const formOrder = await page.$$eval('.cms-item-body > [data-field]', (fs) => fs.slice(0, 2).map((f) => f.dataset.field));
    check('champs : fiche d\'un contenu dans l\'ordre et avec les réglages du schéma', formOrder[0] === 'client', formOrder.join(','));
    await page.click('.cms-item-head h2');
    for (let i = 0; i < 10; i++) {
      await page.click('.cms-head .icon-btn[title^="Annuler"]');
      await page.waitForTimeout(250);
    }
    await until(() => cfg() === cfg0, 'annulation du schéma');
    check('champs : tout annulé, .pages.yml identique octet pour octet', true);

    // ---------------------------------------------------------------- Nouvelle collection (3.3)
    const doc0 = fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
    await page.click('.cms-col-new');
    await page.fill('.dialog input', 'Offres de stage');
    await page.press('.dialog input', 'Enter');
    await until(() => cfg().includes('- name: offres_de_stage\n    label: Offres de stage\n    type: collection\n    path: src/content/offres-de-stage\n'), 'collection décrite');
    await page.waitForSelector('.cms-col.is-on[data-collection="offres_de_stage"]');
    await page.waitForSelector('.schema-row[data-field="slug"]');
    check('collection : créée (site, .pages.yml), ouverte dans « Champs »', fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8').includes('"offres_de_stage"'));
    await page.click('.cms-fields-btn');
    await page.click('.cms-new');
    await page.fill('.dialog input', 'Stage design graphique');
    await page.press('.dialog input', 'Enter');
    const stage = path.join(siteDir, 'src/content/offres-de-stage/stage-design-graphique.md');
    await until(() => fs.existsSync(stage), 'premier contenu');
    check('collection : premier contenu (titre, adresse)', fs.readFileSync(stage, 'utf8') === '---\ntitle: Stage design graphique\nslug: stage-design-graphique\n---\n', fs.readFileSync(stage, 'utf8').replace(/\n/g, ' | '));
    if (shots) await page.screenshot({ path: `${shots}/cms-collection.png` });
    await page.click('.cms-item-head h2');
    for (let i = 0; i < 2; i++) {
      await page.click('.cms-head .icon-btn[title^="Annuler"]');
      await page.waitForTimeout(300);
    }
    await until(() => cfg() === cfg0 && fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8') === doc0 && !fs.existsSync(path.dirname(stage)), 'collection annulée');
    check('collection : annulée, fichiers identiques', true);
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
