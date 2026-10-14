#!/usr/bin/env node
// Essais de bout en bout de l'éditeur (étape 2.3) : interface servie (dist de Trunk) avec l'API
// Tauri simulée, reliée à un vrai cœur par `revel canvas-serve --bridge` (copie du site : les
// essais le modifient puis annulent).
//
// Usage : node tools/editor-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES]
// Code de sortie 1 si un essai échoue.

import { createRequire } from 'node:module';
import fs from 'node:fs';

const { chromium } = createRequire(import.meta.url)('playwright');
const [ui, bridge, shots] = process.argv.slice(2);
if (!ui || !bridge) {
  console.error('usage : editor-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES]');
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
  await page.addInitScript(mock);
  await page.goto(`${ui.replace(/\/$/, '')}/#lang=fr&theme=dark&bridge=${encodeURIComponent(bridge)}`);
  await page.click('tr.row-link');
  await page.click('.nav-item--sub >> nth=0');
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
  const canUndo = await page.$eval('.editor-toolbar .icon-btn >> nth=0', (b) => !b.disabled);
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
  await page.click('.editor-toolbar .icon-btn >> nth=0');
  f = await waitFrame(() => document.readyState === 'complete' && document.querySelector('h1') && !document.querySelector('h1').hasAttribute('data-essai'), 'annulation');
  const disk = await invoke('edit_open');
  check('annulé : document identique à l\'ouverture', JSON.stringify(disk.site) === JSON.stringify(doc.site));

  // ------------------------------------------------------------------ calques (étape 2.4)
  // Sélection dans le canevas -> ligne des calques sélectionnée et visible.
  f = frameOf();
  await page.frameLocator('iframe.canvas-frame').locator('h1').first().click();
  await page.waitForSelector('.layers .layer.is-selected');
  const selLabel = (await page.textContent('.layers .layer.is-selected')).trim();
  const inView = await page.$eval('.layers .layer.is-selected', (el) => {
    const r = el.getBoundingClientRect();
    const t = el.closest('.layers-tree').getBoundingClientRect();
    return r.top >= t.top && r.bottom <= t.bottom;
  });
  check('calques : élément cliqué sélectionné et visible dans l\'arbre', inView, selLabel);

  // Ligne cliquée -> sélection dans le canevas (barre d'état).
  await page.click('.layers .layer:has-text("section.hero")');
  await page.waitForFunction(() => document.querySelector('.editor-status')?.textContent.includes('section.hero'));
  check('calques : ligne cliquée sélectionnée dans le canevas', true);

  const docBefore = await invoke('edit_open');
  const bodyOf = (site) => JSON.stringify(site.pages.find((p) => p.id === 'index').body);
  // Dupliquer (bouton), puis annuler.
  await page.click('.layers .panel-head .icon-btn >> nth=3');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 2, 'section dupliquée');
  const dup = await invoke('edit_open');
  check('dupliquer : section copiée (nouveaux identifiants), sélectionnée', bodyOf(dup.site).length > bodyOf(docBefore.site).length
    && (await page.textContent('.editor-status')).includes('section.hero'));
  await page.click('.editor-toolbar .icon-btn >> nth=0');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 1, 'annulation de la copie');

  // Descendre (bouton) : la section passe après sa voisine.
  await page.click('.layers .layer:has-text("section.hero")');
  const orderBefore = await f.evaluate(() => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(','));
  await page.click('.layers .panel-head .icon-btn >> nth=1');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') !== ob, 'déplacement', orderBefore);
  check('descendre : ordre modifié dans le canevas', true);
  await page.click('.editor-toolbar .icon-btn >> nth=0');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') === ob, 'annulation du déplacement', orderBefore);

  // Glisser-déposer : section.hero déposée après la section suivante.
  const info = await page.$$eval('.layers .layer', (rs) => rs.map((e) => [e.textContent.trim(), parseInt(e.style.paddingLeft, 10)]));
  const si = info.findIndex(([t]) => t === 'section.hero');
  const di = info.findIndex(([t, pad], i) => i > si && t.startsWith('section') && pad === info[si][1]);
  if (si >= 0 && di >= 0) {
    const rowsL = page.locator('.layers .layer');
    const box = await rowsL.nth(di).boundingBox();
    await rowsL.nth(si).dragTo(rowsL.nth(di), { targetPosition: { x: 40, y: box.height - 3 } });
    f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') !== ob, 'glisser-déposer', orderBefore);
    check('glisser-déposer : section déplacée', true);
    await page.click('.editor-toolbar .icon-btn >> nth=0');
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

  check('aucune erreur dans l\'interface', errors.length === 0, errors.join(' | '));
} catch (e) {
  check('déroulement', false, e.message);
} finally {
  await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
