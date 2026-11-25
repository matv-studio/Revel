#!/usr/bin/env node
// Essais de bout en bout des panneaux et des composants (D-195, D-196), interface en anglais
// (langue par défaut de Revel) : interface servie (dist de Trunk) avec l'API Tauri simulée, reliée
// par `revel canvas-serve --bridge` à un site neuf du modèle Portfolio auquel `--prepare` ajoute
// une seconde collection (contenus fictifs).
//   - Nouveau site : état des modèles ;
//   - panneaux : molette sur les onglets, Ctrl+molette dans Assets, menus du vide de Pages,
//     Components et Assets (nouvelle page, copier puis coller, replier, taille des vignettes,
//     inutilisés seulement) ;
//   - composants : créé depuis une carte de liste (une variable par champ, « Undo »), variables
//     montrées sur la racine seulement, bouton de liaison et nom d'après le calque, types nombre,
//     choix et icône réglés sur une utilisation ;
//   - outil « CMS field » hors d'une liste : calque et étiquette, menus Collections et Field,
//     contenu lu.
//
// Usage : node tools/panels-e2e.mjs --prepare DOSSIER_DU_SITE          (avant de lancer le pont)
//         node tools/panels-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES]

import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

if (process.argv[2] === '--prepare') {
  const dir = process.argv[3];
  const file = path.join(dir, 'site.revel.json');
  const site = JSON.parse(fs.readFileSync(file, 'utf8'));
  if (!site.collections.some((c) => c.id === 'journal')) {
    site.collections.push({ id: 'journal', folder: 'content/journal' });
    fs.writeFileSync(file, JSON.stringify(site, null, 1));
  }
  fs.mkdirSync(path.join(dir, 'content/journal'), { recursive: true });
  const post = (slug, title, author, date) => fs.writeFileSync(path.join(dir, 'content/journal', slug + '.md'),
    `---\ntitle: ${title}\nslug: ${slug}\ndate: ${date}\nauthor: ${author}\n---\nSample post.\n`);
  post('opening', 'Studio opening', 'Camille Roy', '2026-10-02');
  post('fair', 'Back from the fair', 'Noah Martin', '2026-11-12');
  console.log('site prepared: journal collection, 2 posts');
  process.exit(0);
}

const { chromium } = createRequire(import.meta.url)('playwright');
const [ui, bridge, shots] = process.argv.slice(2);
if (!ui || !bridge) {
  console.error('usage : panels-e2e.mjs ADRESSE_INTERFACE ADRESSE_PONT [DOSSIER_CAPTURES]');
  process.exit(2);
}
const mock = fs.readFileSync(new URL('../apps/revel-ui/dev/mock-tauri.js', import.meta.url), 'utf8');
if (shots) fs.mkdirSync(shots, { recursive: true });
const results = [];
const check = (name, ok, detail = '') => {
  results.push(!!ok);
  console.log(`${ok ? 'ok ' : 'ÉCHEC'} ${name}${detail ? ' — ' + detail : ''}`);
};
const invoke = async (cmd, args = {}) => {
  const r = await fetch(bridge.replace(/\/$/, '') + '/__revel/invoke/' + cmd, { method: 'POST', body: JSON.stringify(args) });
  return r.json();
};
const site = async () => (await invoke('edit_open')).site;
const until = async (f, ms = 6000) => {
  const t = Date.now();
  for (;;) {
    try { const v = await f(); if (v) return v; } catch { /* pas encore */ }
    if (Date.now() - t > ms) return false;
    await new Promise((r) => setTimeout(r, 150));
  }
};

const browser = await chromium.launch();
let page = null;
try {
  page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(`${e.message} (after check ${results.length})`));
  page.on('console', (m) => { if (m.type() === 'error' && /panicked/.test(m.text())) errors.push(m.text().slice(0, 300)); });
  await page.addInitScript(mock);
  await page.goto(`${ui.replace(/\/$/, '')}/#lang=en&theme=dark&simple=1&bridge=${encodeURIComponent(bridge)}`);
  const STY = '.dock-panel[data-panel="styles"]';
  const w = (ms = 350) => page.waitForTimeout(ms);
  const shot = async (n) => { if (shots) await page.screenshot({ path: `${shots}/${n}.png` }); };
  const tab = async (id) => { await page.click(`.dock-tab[data-panel="${id}"]`); await w(250); };
  const menu = () => page.$$eval('.context-item', (l) => l.map((e) => ({ label: [...e.querySelectorAll('span')].map((s) => s.textContent.trim()).find((t) => t) || '', note: e.querySelector('.menu-note')?.textContent || '', off: e.disabled, sep: e.previousElementSibling?.tagName === 'HR' || e.classList.contains('has-sep') })));
  const pick = async (label) => { await page.locator('.context-item', { hasText: label }).first().click(); await w(450); };
  const layerNames = () => page.$$eval('.layers-tree .layer', (l) => l.map((e) => (e.querySelector('.layer-label')?.textContent || '').trim()));
  const selected = () => page.$eval('.layers-tree .layer.is-selected', (e) => [e.querySelector('.layer-label')?.textContent.trim(), e.querySelector('.layer-preview')?.textContent.trim() || '', e.querySelector('.layer-icon')?.classList.contains('is-cms') || e.querySelector('.layer-icon')?.innerHTML.includes('ellipse')]).catch(() => []);
  const emptyMenu = async (id) => {
    await tab(id);
    // Un point du panneau qui n'est sur aucune ligne ni vignette.
    const pt = await page.evaluate((id) => {
      const host = document.querySelector(`.dock-panel[data-panel="${id}"] [data-menu]`);
      const b = host.getBoundingClientRect();
      for (let y = b.bottom - 4; y > b.top; y -= 6) {
        for (let x = b.left + 3; x < b.right; x += 9) {
          const e = document.elementFromPoint(x, y);
          if (e && host.contains(e) && !e.closest('.list-row, .asset, .layer, button, input, select, figure, .folder-row, .page-row, [draggable]')) return [x, y];
        }
      }
      return null;
    }, id);
    if (!pt) throw new Error('no empty space in ' + id);
    await page.mouse.click(pt[0], pt[1], { button: 'right' });
    await w(300);
    return menu();
  };

  // ---------------------------------------------------------------- Nouveau site : état des modèles
  await page.click('.page-head .btn--primary');
  await page.waitForSelector('.new-site');
  await page.locator('.new-site button', { hasText: /^More$/ }).click();
  const state = await until(() => page.$eval('.new-site__tpl-state', (e) => e.dataset.state), 15000);
  const revelTab = await page.locator('.new-site button', { hasText: /^Revel$/ }).count();
  check('new site: templates state shown next to the Revel tab', (state === 'online' || state === 'offline') && revelTab === 1, `${state}`);
  await shot('new-site-state');
  await page.keyboard.press('Escape'); await w(300);
  if (await page.$('.new-site')) await page.locator('.dialog .btn', { hasText: /Cancel/ }).first().click().catch(() => {});

  // ---------------------------------------------------------------- ouverture du site
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame');
  const frameOf = () => page.frames().find((f) => f.url().includes('rv='));
  await until(() => frameOf()?.evaluate(() => document.readyState === 'complete' && !!document.querySelector('[data-rv]')), 20000);
  await w(800);

  // ---------------------------------------------------------------- molette sur les onglets (D-195)
  {
    const group = await page.evaluate(() => document.querySelector('.dock-tab[data-panel="pages"]').closest('.dock-tabs').getAttribute('data-tabs'));
    const active = () => page.evaluate((g) => document.querySelector(`.dock-tabs[data-tabs="${g}"] .dock-tab.is-on`)?.getAttribute('data-panel'), group);
    await tab('pages');
    const head = await page.locator(`.dock-tabs[data-tabs="${group}"]`).boundingBox();
    await page.mouse.move(head.x + 40, head.y + 10);
    const seq = [await active()];
    for (const d of [100, 100, -100]) { await page.mouse.wheel(0, d); await w(250); seq.push(await active()); }
    check('panels: wheel over the tabs switches to the next and previous tab', seq[1] !== seq[0] && seq[2] !== seq[1] && seq[3] === seq[1], seq.join(' → '));
  }

  // ---------------------------------------------------------------- menus du vide (D-196)
  {
    const pages0 = (await site()).pages.length;
    let m = await emptyMenu('pages');
    const labels = m.map((i) => i.label);
    const pasteOff = m.find((i) => /Paste page/.test(i.label))?.off;
    check('pages: empty-space menu (new page, new folder, new CMS page, paste, collapse, site map)',
      ['New page', 'New folder', 'New CMS page…', 'Paste page', 'Collapse all folders', 'Open site map'].every((l) => labels.includes(l)) && pasteOff === true, labels.join(' | '));
    await shot('menu-pages');
    await pick('New page');
    const input = await until(() => page.$('.dock-panel[data-panel="pages"] input.rename-input, .dock-panel[data-panel="pages"] .list-row input'));
    if (input) { await page.keyboard.type('Studio'); await page.keyboard.press('Enter'); }
    const created = await until(async () => (await site()).pages.length === pages0 + 1);
    // Copier une page (son menu), la coller par le menu du vide.
    await page.locator('.dock-panel[data-panel="pages"] .list-row', { hasText: 'Studio' }).first().click({ button: 'right' }); await w(300);
    await pick('Copy');
    m = await emptyMenu('pages');
    const pasteOn = m.find((i) => /Paste page/.test(i.label))?.off === false;
    await pick('Paste page');
    const pasted = await until(async () => (await site()).pages.length === pages0 + 2);
    check('pages: new page from the menu, page copied then pasted', created && pasteOn && pasted, JSON.stringify({ created, pasteOn, pasted }));
    for (let i = 0; i < 2; i++) { await page.click('.editor-toolbar .btn-undo'); await w(500); }
  }
  {
    const n0 = (await site()).components.length;
    let m = await emptyMenu('components');
    const labels = m.map((i) => i.label);
    check('components: empty-space menu (new component, new folder, from selection, paste, collapse)',
      ['New component', 'New folder', 'Create from selection', 'Paste component', 'Collapse all folders'].every((l) => labels.includes(l)) && m.find((i) => /Paste component/.test(i.label))?.off === true, labels.join(' | '));
    await shot('menu-components');
    await page.keyboard.press('Escape'); await w(200);
    await page.locator('.dock-panel[data-panel="components"] .component-row').first().click({ button: 'right' }); await w(300);
    await pick('Copy');
    m = await emptyMenu('components');
    await pick('Paste component');
    const pasted = await until(async () => (await site()).components.length === n0 + 1);
    const copy = (await site()).components.at(-1);
    check('components: component copied then pasted (new name, new ids)', pasted && /\(copy\)$/.test(copy?.name || ''), copy?.name);
    await page.click('.editor-toolbar .btn-undo'); await w(500);
  }
  {
    let m = await emptyMenu('assets');
    const labels = m.map((i) => i.label);
    check('assets: empty-space menu (import, new folder, paste, thumbnails, unused only, file explorer)',
      ['Import files…', 'New folder', 'Paste from clipboard', 'Larger thumbnails', 'Smaller thumbnails', 'Show unused only', 'Show in file explorer'].every((l) => labels.includes(l)), labels.join(' | '));
    await shot('menu-assets');
    const thumb = () => page.$eval('.asset-grid', (e) => parseInt(e.style.getPropertyValue('--thumb'), 10));
    const t0 = await thumb();
    await pick('Larger thumbnails');
    const t1 = await thumb();
    // Ctrl + molette (D-195).
    const grid = await page.locator('.asset-grid').first().boundingBox();
    await page.mouse.move(grid.x + 30, grid.y + 30);
    await page.keyboard.down('Control'); await page.mouse.wheel(0, 240); await w(250); await page.keyboard.up('Control');
    const t2 = await thumb();
    check('assets: larger thumbnails from the menu, Ctrl+wheel makes them smaller', t1 === t0 + 16 && t2 < t1, `${t0} → ${t1} → ${t2}`);
    const count = () => page.$$eval('.dock-panel[data-panel="assets"] .asset', (l) => l.length);
    const all = await count();
    await emptyMenu('assets'); await pick('Show unused only');
    const unused = await until(async () => { const c = await count(); return c < all ? c + 1 : false; });
    await emptyMenu('assets'); await pick('Show unused only');
    const back = await until(async () => (await count()) === all);
    check('assets: unused only hides the media the site uses', unused !== false && back, `${all} → ${unused === false ? '?' : unused - 1} → ${all}`);
  }

  // ---------------------------------------------------------------- composant depuis une carte de liste (D-195)
  {
    const s0 = await site();
    await tab('layers');
    const names = await layerNames();
    const i = names.findIndex((n) => /^(List|Collection list)/.test(n));
    if (i < 0) throw new Error('no collection list among the layers: ' + names.join(','));
    // Première carte de la liste : utilisation d'un composant, détachée pour repartir d'un élément.
    const row = page.locator('.layers-tree .layer').nth(i + 1);
    await row.click(); await w(500);
    const isUse = !!(await page.$(`${STY} [title^="Detach"]`));
    if (isUse) { await page.click(`${STY} [title^="Detach"]`); await w(900); }
    await tab('components');
    await emptyMenu('components'); await pick('Create from selection');
    await page.waitForSelector('.dialog input');
    await page.locator('.dialog input').fill('Project card'); await page.keyboard.press('Enter');
    const made = await until(async () => (await site()).components.find((c) => c.name === 'Project card'));
    const props = made ? made.props.map((p) => p.name) : [];
    const noRead = made && !JSON.stringify(made.body).includes('item.');
    const notice = await until(() => page.$('[data-action="notice-undo"]'));
    check('components: created from a list card, one variable per field read, body free of list reads', !!made && props.length >= 3 && noRead && !!notice, props.join(','));
    await shot('component-from-list');
    await page.click('[data-action="notice-undo"]'); await w(1200);
    const undone = (await site()).components.length === s0.components.length + 0 || !(await site()).components.some((c) => c.name === 'Project card');
    check('components: “Undo” in the message removes the component in one step', undone);
    if (isUse) { await page.click('.editor-toolbar .btn-undo'); await w(600); }
  }

  // ---------------------------------------------------------------- variables d'un composant (D-195, D-196)
  {
    await emptyMenu('components'); await pick('New component');
    await page.waitForSelector('.dialog input');
    await page.locator('.dialog input').fill('Badge'); await page.keyboard.press('Enter');
    const opened = await until(async () => (await site()).components.some((c) => c.id === 'badge') && page.$('.comp-bar, .component-canvas, [data-comp-open]').then(() => true));
    await w(1200);
    // Contenu : un texte et une icône dans le cadre vide.
    await tab('layers'); await page.locator('.layers-tree .layer').first().click(); await w(300);
    for (const it of ['insert-paragraph', 'insert-icon']) {
      await tab('insert');
      const el = page.locator(`.insert-item[data-item="${it}"]`);
      if (await el.count()) { await el.dblclick(); await w(900); }
      await tab('layers'); await page.locator('.layers-tree .layer').first().click(); await w(300);
    }
    const kids = await layerNames();
    // Sous-élément : pas de liste des variables, son bouton de liaison seulement.
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Paragraph$/ }).first().click(); await w(400);
    const subList = await page.locator(`${STY} .prop-def-label`).count();
    const btn = page.locator(`${STY} .bind-btn[data-bind="component-bind-text"]`);
    await btn.click(); await w(300);
    const first = (await menu())[0]?.label;
    await page.locator('.context-item').first().click(); await w(700);
    let c = (await site()).components.find((x) => x.id === 'badge');
    const named = c.props.find((p) => p.type === 'text');
    check('components: bind button on the layer, new variable named after the layer', subList === 0 && /Paragraph/.test(first || '') && named?.label === 'Paragraph' && named?.name === 'paragraph', `${first} → ${JSON.stringify(named)}`);
    const iconRow = page.locator('.layers-tree .layer .layer-label', { hasText: /^Icon$/ });
    let iconBound = null;
    if (await iconRow.count()) {
      await iconRow.first().click(); await w(400);
      await page.click(`${STY} .bind-btn[data-bind="component-bind-icon"]`); await w(300);
      await page.locator('.context-item').first().click(); await w(800);
      c = (await site()).components.find((x) => x.id === 'badge');
      iconBound = JSON.stringify(c.body).includes('"expr":"icon | icon"') && c.props.some((p) => p.type === 'icon' && p.name === 'icon');
      const drawn = await frameOf().evaluate(() => (document.querySelector('svg[data-icon]')?.innerHTML || '').includes('<path'));
      check('components: icon linked to an “Icon” variable, still drawn', iconBound && drawn, kids.join(','));
    } else check('components: icon linked to an “Icon” variable, still drawn', false, 'no icon layer: ' + kids.join(','));
    // Racine : liste des variables, types nombre et choix ajoutés.
    await page.locator('.layers-tree .layer').first().click(); await w(400);
    const rootList = await page.locator(`${STY} .prop-def-label`).count();
    await page.click(`${STY} [title="Add a variable"]`); await w(250);
    const kinds = (await menu()).map((i) => i.label);
    await pick('Number');
    await page.click(`${STY} [title="Add a variable"]`); await w(250); await pick('Choice');
    for (const v of ['Small', 'Large']) { const i = page.locator(`${STY} .prop-def[data-prop="choix"] .prop-options input`).last(); await i.fill(v); await i.press('Enter'); await w(450); }
    c = (await site()).components.find((x) => x.id === 'badge');
    const choice = c.props.find((p) => p.type === 'choice');
    check('components: variables listed on the root only; number, choice and icon types', rootList >= 2 && ['Number', 'Choice', 'Icon'].every((k) => kinds.includes(k)) && JSON.stringify(choice?.options) === '["Small","Large"]' && choice?.default === 'Small', `${rootList} | ${JSON.stringify(choice)}`);
    await shot('component-variables');
    // Texte relié au choix, puis réglé sur une utilisation.
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Paragraph$/ }).first().click(); await w(400);
    await page.locator(`${STY} .bind-btn[data-bind="component-bind-text"]`).first().click(); await w(300);
    const offered = (await menu()).map((i) => i.label);
    await pick('Choice');
    check('components: a text can read a text, number or choice variable', ['Number', 'Choice'].every((k) => offered.includes(k)) && JSON.stringify((await site()).components.find((x) => x.id === 'badge').body).includes('"expr":"choix"'), offered.join(' | '));
    for (let i = 0; i < 20 && (await site()).components.some((x) => x.id === 'badge'); i++) { await page.click('.editor-toolbar .btn-undo'); await w(350); }
    check('components: every step undone', !(await site()).components.some((x) => x.id === 'badge'));
    await w(800);
  }

  // ---------------------------------------------------------------- utilisation sans champ du CMS (retour du 24/11)
  {
    await tab('layers');
    const src = async (label) => {
      await page.locator('.layers-tree .layer .layer-label', { hasText: label }).first().click(); await w(600);
      return page.evaluate(() => [!!document.querySelector('.instance-content'), !!document.querySelector('.instance-content.has-source'), !!document.querySelector('.instance-content .content-collection, .instance-content .content-source')]);
    };
    const card = await src(/^Projet de la grille$/);
    const nav = await src(/^Header$/);
    check('instances: “Content” shown only when the instance reads a CMS field', card[0] && card[1] && nav[0] && !nav[1] && !nav[2], JSON.stringify({ card, nav }));
  }

  // ---------------------------------------------------------------- Collection list : mêmes menus que « CMS field » (D-197)
  {
    const body = async () => JSON.stringify((await site()).pages.find((p) => p.path === '/').body);
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^List/ }).first().click(); await w(600);
    await page.click(`${STY} [data-action="list-collections"]`); await w(300);
    const m = await menu();
    await pick('Journal');
    const two = await until(async () => (await body()).includes(' + collections.journal'));
    const label = await page.$eval(`${STY} [data-action="list-collections"]`, (e) => e.textContent.trim());
    check('collection list: “Collections” menu with “All” first, second collection checked', m.map((i) => i.label).join('|') === 'All|Projets|Journal' && two && label === 'All' && !(await page.$(`${STY} .list-also`)), `${m.map((i) => i.label).join('|')} → ${label}`);
    await shot('list-collections');
    await page.click('.editor-toolbar .btn-undo'); await w(500);
  }

  // ---------------------------------------------------------------- outil « CMS field » hors d'une liste (D-196)
  {
    const body = async () => JSON.stringify((await site()).pages.find((p) => p.path === '/').body);
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Paragraph$/ }).first().click(); await w(400);
    await tab('insert'); await page.dblclick('.insert-item[data-item="insert-cms-field"]'); await w(1200);
    await tab('layers');
    const placed = await selected();
    const crumb = await page.$$eval('.editor-status .node-crumb', (l) => l.at(-1)?.textContent.trim());
    const marked = (await body()).includes('"data-revel-base","value":"cms-field"');
    const noText = !(await page.$(`${STY} .text-fields`));
    check('CMS field: placed outside a list as a “CMS field” layer with the CMS icon, same name in the status bar', placed[0] === 'CMS field' && placed[2] && crumb === 'CMS field' && marked && noText, JSON.stringify({ placed, crumb, marked, noText }));
    await shot('cms-field-placed');
    await page.click(`${STY} [data-action="cms-field-collections"]`); await w(300);
    const cols = await menu();
    await shot('cms-field-collections');
    await page.keyboard.press('Escape'); await w(200);
    await page.click(`${STY} [data-action="cms-field-pick"]`); await w(300);
    const all = await menu();
    await shot('cms-field-fields');
    await page.keyboard.press('Escape'); await w(200);
    const author = all.find((i) => i.label === 'Author');
    const title = all.find((i) => /^(Titre|Title)$/.test(i.label));
    check('CMS field: “Collections” menu with “All” first, fields shared first then those of a single collection, each naming its collection',
      cols.map((i) => i.label).join('|') === 'All|Projets|Journal' && /Projets · Journal/.test(title?.note || '') && author?.note === 'Journal' && all.indexOf(author) > all.indexOf(title) && !all.some((i) => /only/i.test(i.note)),
      JSON.stringify({ cols: cols.map((i) => i.label), title, author }));
    // Une seule collection cochée : valeur du premier contenu à côté de chaque champ.
    await page.click(`${STY} [data-action="cms-field-collections"]`); await w(300); await pick('Journal');
    const shown = await page.$eval(`${STY} [data-action="cms-field-collections"]`, (e) => e.textContent.trim());
    await page.click(`${STY} [data-action="cms-field-pick"]`); await w(300);
    const one = await menu();
    await pick('Author');
    const bound = await until(async () => (await body()).includes('"expr":"collections.journal[0].data.author"'));
    const named = await until(async () => { const s = await selected(); return s[0] === 'Author' && /Camille Roy/.test(s[1]) ? s : false; });
    const drawn = await until(() => frameOf().evaluate(() => [...document.querySelectorAll('p')].some((e) => e.textContent.trim() === 'Camille Roy')));
    check('CMS field: field chosen in one collection (value previewed), layer named after the field, value drawn',
      shown === 'Journal' && one.find((i) => i.label === 'Author')?.note === 'Camille Roy' && bound && !!named && drawn, JSON.stringify({ shown, bound, named, drawn }));
    await shot('cms-field-bound');
    await page.selectOption(`${STY} [data-bound="cms-field-item"] select`, 'fair');
    const other = await until(async () => (await body()).includes('entry(\\"journal\\", \\"fair\\").data.author'));
    const drawn2 = await until(() => frameOf().evaluate(() => [...document.querySelectorAll('p')].some((e) => e.textContent.trim() === 'Noah Martin')));
    const rows = await page.evaluate((s) => ['cms-field-collection', 'cms-field-item', 'cms-field-as', 'cms-field-empty'].map((k) => !!document.querySelector(`${s} [data-bound="${k}"]`)), STY);
    check('CMS field: another item read; collection, item, shown as and if-empty settings', other && drawn2 && rows.every(Boolean), JSON.stringify({ other, drawn2, rows }));
    for (let i = 0; i < 6 && (await body()).includes('cms-field'); i++) { await page.click('.editor-toolbar .btn-undo'); await w(400); }
    check('CMS field: every step undone', !(await body()).includes('cms-field'));
  }
  // ---------------------------------------------------------------- logique des outils (D-199)
  {
    const home = async () => (await site()).pages.find((p) => p.path === '/');
    const before = JSON.stringify((await home()).body);
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Paragraph$/ }).first().click(); await w(400);
    await tab('insert'); await page.dblclick('.insert-item[data-item="insert-form"]'); await w(1500);
    // Section complète : posée après la section de la sélection, pas dedans (audit M2).
    // Balises des éléments qui contiennent le formulaire, de la racine vers lui.
    const trail = (nodes, path = []) => { for (const n of nodes) { if (n.tag === 'form') return path; for (const k of ['children', 'then', 'otherwise', 'body']) if (Array.isArray(n[k])) { const r = trail(n[k], [...path, n.tag || n.node]); if (r) return r; } } return null; };
    const around = trail((await home()).body) || [];
    check('tools: a full section is placed after the selected section, not inside it', around.filter((t) => t === 'section').length === 1, around.join(' › '));
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Form$/ }).first().click(); await w(700);
    const rows = () => page.$$eval(`${STY} .form-fields .items-row`, (l) => l.map((e) => e.querySelector('.items-name').textContent.trim() + ':' + e.dataset.kind));
    const start = await rows();
    await page.click(`${STY} [data-action="form-field-add"]`); await w(300);
    const kinds = (await menu()).map((i) => i.label);
    await pick('Dropdown');
    // Saisie écrite dans le champ lui-même : le curseur y arrive une image plus tard.
    await page.locator(`${STY} .items-rename`).fill('Subject'); await page.locator(`${STY} .items-rename`).press('Enter');
    const added = await until(async () => (await rows()).some((r) => /^Subject( \*)?:dropdown$/.test(r)));
    await page.locator(`${STY} .form-fields .items-row[data-kind="dropdown"]`).hover();
    await page.locator(`${STY} .form-fields .items-row[data-kind="dropdown"] .form-required`).click();
    const required = await until(async () => /"tag":"select","attrs":\[[^\]]*"name":"required"/.test(JSON.stringify((await home()).body)));
    await page.locator(`${STY} .form-fields .items-row[data-kind="dropdown"] .items-main`).click(); await w(400);
    const options = await page.$$eval(`${STY} .form-options input`, (l) => l.map((e) => e.value).filter(Boolean));
    check('tools: form fields listed; a dropdown added, named, made required, its options shown',
      start.length >= 3 && ['Text', 'Email', 'Phone', 'Long text', 'Dropdown', 'Checkbox', 'Radio buttons', 'Date', 'File'].every((k) => kinds.includes(k)) && added && required && options.join('|') === 'Option 1|Option 2',
      JSON.stringify({ start, added, required, options }));
    await shot('form-fields');
    // Après l'envoi : message, adresse de réception, puis page ouverte (D-201).
    if (!(await page.$(`${STY} [data-form="after"]`))) { await page.locator(`${STY} [data-role="form"] .switch`).click(); await w(900); }
    const formAttr = async (n) => { const m = JSON.stringify((await home()).body).match(new RegExp(`"name":"${n}","value":"([^"]*)"`)); return m ? m[1] : null; };
    await page.fill(`${STY} [data-form="message"] input`, 'Thank you'); await page.keyboard.press('Tab'); await w(700);
    await page.fill(`${STY} [data-form="to"] input`, 'quotes@example.com'); await page.keyboard.press('Tab'); await w(700);
    const own = await until(async () => (await formAttr('data-revel-success')) === 'Thank you' && (await formAttr('data-revel-to')) === 'quotes@example.com');
    await page.click(`${STY} [data-after="page"]`); await w(900);
    const pageUrls = await page.$$eval(`${STY} [data-form="page"] option`, (l) => l.map((o) => o.value));
    await page.selectOption(`${STY} [data-form="page"] select`, pageUrls.at(-1)); await w(700);
    const goes = await until(async () => (await formAttr('data-revel-redirect')) === pageUrls.at(-1));
    await page.click(`${STY} [data-after="message"]`); await w(900);
    const backToMessage = await until(async () => (await formAttr('data-revel-redirect')) === null && !!(await page.$(`${STY} [data-form="message"] input`)));
    check('tools: form shows a message or opens a page after sending, with its own address', own && pageUrls.length >= 2 && goes && backToMessage, JSON.stringify({ own, pageUrls, goes, backToMessage }));
    // Barre de navigation : lien ajouté vers une page, puis renommé.
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Paragraph$/ }).first().click(); await w(400);
    await tab('insert'); await page.dblclick('.insert-item[data-item="insert-navbar"]'); await w(1500);
    await tab('layers');
    await page.locator('.layers-tree .layer .layer-label', { hasText: /^Navbar$/ }).first().click(); await w(700);
    const links = () => page.$$eval(`${STY} .items-row`, (l) => l.map((e) => e.querySelector('.items-name').textContent.trim() + '→' + (e.querySelector('.items-target')?.textContent || '')));
    const l0 = await links();
    await page.click(`${STY} [data-action="items-add-link"]`); await w(300);
    const choices = await menu();
    const target = choices.find((i) => i.note && i.note !== '/');
    await pick(target.label);
    const l1 = await until(async () => { const l = await links(); return l.length === l0.length + 1 && l.at(-1) === `${target.label}→${target.note}` ? l : false; });
    await page.locator(`${STY} .items-row`).last().hover();
    await page.locator(`${STY} .items-row`).last().locator('[data-action="items-rename"]').click();
    // Saisie écrite dans le champ lui-même : le curseur y arrive une image plus tard.
    await page.locator(`${STY} .items-rename`).fill('Shop'); await page.locator(`${STY} .items-rename`).press('Enter');
    const renamed = await until(async () => (await links()).at(-1) === `Shop→${target.note}`);
    check('tools: navbar link added to a chosen page (its address shown), then renamed in place',
      !!l1 && renamed && choices.some((i) => i.label === 'Web address…') && choices.some((i) => i.label === 'New page…'), JSON.stringify({ l0, target, renamed }));
    await shot('navbar-links');
    // Poignée de glisser (D-200) : dernier lien glissé en tête ; puis remis par le clavier.
    const rowsSel = `${STY} .items-list--links [data-sort-row]`;
    const grip = page.locator(`${rowsSel} .sort-grip`).last();
    const g = await grip.boundingBox();
    const top = await page.locator(rowsSel).first().boundingBox();
    await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2);
    await page.mouse.down();
    await page.mouse.move(top.x + 40, top.y + 6, { steps: 8 });
    const marked = await page.$$eval(`${rowsSel}.is-drop-before`, (l) => l.length);
    await page.mouse.up();
    const dragged = await until(async () => (await links())[0] === `Shop→${target.note}`);
    await page.locator(`${rowsSel} .sort-grip`).first().focus();
    for (let k = 0; k < l0.length; k++) { await page.keyboard.press('ArrowDown'); await w(350); await page.locator(`${rowsSel} .sort-grip`).nth(k + 1).focus(); }
    const back = await until(async () => (await links()).at(-1) === `Shop→${target.note}`);
    const noArrows = (await page.$$(`${STY} [title="Move up"], ${STY} [title="Move down"]`)).length === 0;
    check('tools: reorder by drag handle (drop line shown) and by keyboard, no up/down arrows', marked === 1 && dragged && back && noArrows, JSON.stringify({ marked, dragged, back, noArrows }));
    for (let i = 0; i < 24 && JSON.stringify((await home()).body) !== before; i++) { await page.click('.editor-toolbar .btn-undo'); await w(400); }
    check('tools: every step undone', JSON.stringify((await home()).body) === before);
  }
  check('no error in the interface', errors.length === 0, errors.join(' | '));
} catch (e) {
  check('run', false, e.message.split('\n')[0]);
  if (shots && page) await page.screenshot({ path: `${shots}/failure.png` }).catch(() => {});
} finally {
  await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
