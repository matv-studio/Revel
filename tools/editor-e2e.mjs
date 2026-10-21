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
import os from 'node:os';
import { execFileSync } from 'node:child_process';

const { chromium } = createRequire(import.meta.url)('playwright');
const [ui, bridge, shots, siteDir] = process.argv.slice(2);
// Serveur des assistants (MCP) du même cœur (`canvas-serve --mcp-port`), facultatif.
const mcpUrl = process.env.REVEL_E2E_MCP;
const mcpToken = process.env.REVEL_MCP_TOKEN;
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
let shotPage = null;
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  shotPage = page;
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
  // Accueil : supprimer un site (confirmation, dossier mis à la corbeille ; simulé ici).
  {
    const card = page.locator('.site-card', { hasText: 'Maison Oka' });
    await card.hover();
    await card.locator('.site-delete').click();
    await page.waitForSelector('.dialog .btn--danger');
    const title = (await page.textContent('.dialog')).includes('Maison Oka');
    await page.click('.dialog .btn--danger');
    await page.waitForFunction(() => ![...document.querySelectorAll('.site-card')].some((c) => c.textContent.includes('Maison Oka')));
    check('accueil : site supprimé après confirmation', title);
  }
  // Import (étape 7), une boîte pour toutes les sources : dossier de pages HTML examiné puis
  // importé (bilan dans la boîte) ; export Webflow et contenus de son CMS (CSV ou API) ; site
  // lu depuis un lien (avancement, annulation) ; dépôt Git copié (site à construire signalé,
  // dossier produit importable, licence absente signalée, copie abandonnée retirée) ; fichier
  // Figma (jeton, cadres choisis) ; export WordPress (examen, images).
  {
    const openImport = async () => {
      await page.click('.page-head .toolbar .btn:has-text("Importer")');
      await page.waitForSelector('.import-dialog');
    };
    const closeSummary = async () => {
      await page.waitForSelector('.import-summary');
      await page.click('.import-close');
      await page.waitForFunction(() => !document.querySelector('.import-dialog'));
    };
    const stat = (k) => page.$eval(`.import-stat[data-stat="${k}"] .import-stat-value`, (e) => e.textContent.trim()).catch(() => null);
    await openImport();
    const tiles = await page.$$eval('.import-sources .import-tile', (l) => l.map((b) => b.dataset.source).join());
    const runOff = await page.$eval('.import-run', (b) => b.disabled);
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-sources.png` });
    await page.evaluate(() => { window.__revelPick = 'C:\\Sites\\maquette-html'; });
    await page.click('.import-choose');
    await page.waitForSelector('.import-kind.chip--ok');
    const kind = (await page.textContent('.import-kind')).trim();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-dossier.png` });
    await page.click('.import-run');
    await page.waitForSelector('.import-summary');
    const pagesStat = await stat('import-stat-pages');
    const warnCount = (await page.textContent('.import-warnings summary .chip')).trim();
    const title = (await page.textContent('.import-summary h2')).trim();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-bilan.png` });
    await closeSummary();
    const listed = await page.locator('.site-card', { hasText: 'Maquette html' }).count();
    check('import : sources proposées, dossier examiné, importé, bilan dans la boîte',
      tiles === 'folder,link,git,figma,wordpress' && runOff && kind === 'Site HTML · 12 pages' && title === 'Site importé' && pagesStat === '12' && warnCount === '1' && listed === 1,
      `${tiles} · ${kind} · ${pagesStat} · ${warnCount}`);
    // Export Webflow : outil reconnu, fichiers CSV du CMS trouvés, ajoutés, retirés ; API.
    await openImport();
    await page.evaluate(() => { window.__revelPick = 'C:\\Sites\\export-webflow'; });
    await page.click('.import-choose');
    await page.waitForSelector('.import-cms');
    const madeWith = (await page.textContent('.import-status .import-made-with')).trim();
    const found = await page.locator('.import-file').count();
    await page.evaluate(() => { window.__revelPickFiles = ['C:\\Exports\\Atelier - Categories - 1.csv', 'C:\\Exports\\Atelier - Auteurs - 2.csv']; });
    await page.click('.import-cms-add');
    await page.waitForFunction(() => document.querySelectorAll('.import-file').length === 3);
    await page.locator('.import-file', { hasText: 'Auteurs' }).locator('.chip-remove').click();
    await page.waitForFunction(() => document.querySelectorAll('.import-file').length === 2);
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-webflow-cms.png` });
    await page.click('.import-cms-mode button[data-mode="api"]');
    const apiOff = await page.$eval('.import-run', (b) => b.disabled);
    await page.fill('.import-wf-token', 'jeton-du-site');
    const apiOn = !(await page.$eval('.import-run', (b) => b.disabled));
    await page.click('.import-cms-mode button[data-mode="csv"]');
    await page.click('.import-run');
    await page.waitForSelector('.import-summary');
    const sent = await page.evaluate(() => window.__revelImportOptions);
    const entries = await stat('import-stat-entries');
    const forms = await stat('import-stat-forms');
    await closeSummary();
    check('import : export Webflow, contenus du CMS (fichiers CSV ajoutés, retirés ; jeton exigé pour l\'API)',
      madeWith === 'Fait avec Webflow' && found === 1 && apiOff && apiOn && sent.csv.length === 2 && !sent.webflow_token && entries === '24' && forms === '1',
      `${madeWith} · ${found} · ${JSON.stringify(sent)}`);
    // Lien : dossier créé d'après le domaine, avancement, annulation, puis import.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="link"]');
    const linkOff = await page.$eval('.import-run', (b) => b.disabled);
    await page.fill('.import-link', 'https://www.studio.framer.website/');
    await page.waitForFunction(() => /studio-framer-website$/.test(document.querySelector('.import-target')?.textContent.trim() || ''));
    await page.evaluate(() => { window.__revelImportSlow = true; });
    await page.click('.import-run');
    const firstText = await page.waitForSelector('.import-progress-label').then((e) => e.textContent());
    await page.waitForFunction(() => /[1-9]\d* pages · \d+ fichiers · [\d,]+ Mo$/.test(document.querySelector('.import-progress-label')?.textContent || ''));
    const progressText = (await page.textContent('.import-progress-label')).trim();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-lien-lecture.png` });
    await page.click('.import-cancel');
    await page.waitForFunction(() => !document.querySelector('.import-progress'));
    const stillOpen = await page.locator('.import-dialog').count() === 1 && await page.locator('.import-error').count() === 0;
    await page.evaluate(() => { window.__revelImportSlow = false; });
    await page.click('.import-run');
    await page.waitForSelector('.import-summary');
    const framer = (await page.textContent('.import-summary .import-made-with')).trim();
    const anims = await stat('import-stat-animations');
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-lien-bilan.png` });
    await closeSummary();
    check('import : site lu depuis un lien (avancement, annulation sans message, bilan Framer)',
      linkOff && firstText.trim() === 'Lecture du site…' && /^Lecture du site… \d+ pages · \d+ fichiers · [\d,]+ Mo$/.test(progressText) && stillOpen && framer === 'Fait avec Framer' && anims === '37',
      `${progressText} · ${framer} · ${anims}`);
    // Dépôt de l'équipe dans un dossier partagé (D-89) : choisi avec « Dossier… ».
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.evaluate(() => { window.__revelPick = '\\\\NAS\\Equipe\\Sites\\vitrine.git'; });
    await page.click('.import-git-folder');
    await page.waitForFunction(() => /vitrine$/.test(document.querySelector('.import-target')?.textContent.trim() || ''));
    const nasUrl = await page.inputValue('.import-url');
    const nasRun = await page.$eval('.import-run', (b) => !b.disabled);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('.import-dialog'));
    check('import : dépôt de l\'équipe dans un dossier partagé (chemin réseau, nom du site)',
      nasUrl === '\\\\NAS\\Equipe\\Sites\\vitrine.git' && nasRun, nasUrl);
    // Dépôt Git : site à construire, sans licence ; son dossier produit s'importe.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'https://github.com/exemple/svelte-p3r');
    await page.waitForFunction(() => /svelte-p3r$/.test(document.querySelector('.import-target')?.textContent.trim() || ''));
    await page.click('.import-run');
    await page.waitForSelector('.import-kind.chip--warn');
    const build = (await page.textContent('.import-kind')).trim();
    const noLicense = await page.locator('.import-license').count() === 1;
    const output = (await page.textContent('.import-output')).trim();
    const help = await page.getAttribute('.import-kind', 'title');
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-git-a-construire.png` });
    check('import : dépôt Git copié, site à construire signalé, licence absente, dossier produit proposé',
      build === 'Site à construire · SvelteKit' && noLicense && output === 'Importer « build »' && /Construire avec Revel/.test(help || ''), `${build} · ${output}`);
    // Abandon : copie retirée.
    await page.click('.import-cancel');
    await page.waitForFunction(() => !document.querySelector('.import-dialog'));
    const discarded = await page.evaluate(() => window.__revelDiscarded || []);
    check('import : copie abandonnée retirée', discarded.length === 1 && /svelte-p3r$/.test(discarded[0]), discarded.join());
    // Dossier produit importé.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'https://github.com/exemple/svelte-p3r');
    await page.click('.import-run');
    await page.waitForSelector('.import-output');
    await page.click('.import-output');
    await closeSummary();
    check('import : dossier produit par la construction importé', await page.locator('.site-card', { hasText: 'Build' }).count() === 1);
    // Construction par Revel (D-88) : accord (outils, dépendances, code exécuté), étapes, bilan.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'https://github.com/deltea/p3r-pause-menu');
    await page.click('.import-run');
    await page.waitForSelector('.import-build:not([disabled])');
    await page.waitForSelector('.import-published');
    const pub = (await page.textContent('.import-published')).trim();
    await page.click('.import-build');
    await page.waitForSelector('.import-build-panel');
    const items = await page.$$eval('.import-build-item', (l) => l.map((x) => [...x.children].map((c) => c.textContent.trim()).filter(Boolean).join(' ')));
    const risk = (await page.textContent('.import-build-risk')).trim();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-construire.png` });
    await page.evaluate(() => { window.__revelBuildSlow = true; });
    await page.click('.import-build-run');
    await page.waitForSelector('.import-build-progress[data-step="download"]');
    await page.waitForFunction(() => /Mo \/ 58 Mo$/.test(document.querySelector('.import-progress-label')?.textContent || ''));
    const dl = (await page.textContent('.import-progress-label')).trim();
    await page.waitForSelector('.import-build-progress[data-step="build"]');
    const buildLabel = (await page.textContent('.import-progress-label')).trim();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-construction.png` });
    await page.evaluate(() => { window.__revelBuildSlow = false; });
    await page.waitForSelector('.import-summary');
    const builtWarn = (await page.textContent('.import-warnings')).includes('site construit par Revel');
    await closeSummary();
    check('import : dépôt construit par Revel (site publié proposé, accord, téléchargement, construction, bilan)',
      pub === 'Importer le site publié · p3r.deltea.space'
        && items[0] === 'Node.js 24.22.0 58 Mo' && items[1] === 'Dépendances (npm) à télécharger' && /bun\.lock ignoré/.test(items[2] || '')
        && risk === 'Exécute le code du projet' && /^Téléchargement de Node\.js 24\.22\.0 · \d+ Mo \/ 58 Mo$/.test(dl)
        && buildLabel === 'Construction du site (SvelteKit)…' && builtWarn,
      `${items.join(' | ')} · ${dl} · ${buildLabel}`);
    // Site publié trouvé : importé depuis son adresse (copie retirée).
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'https://github.com/deltea/p3r-pause-menu');
    await page.click('.import-run');
    await page.waitForSelector('.import-published');
    const before = (await page.evaluate(() => window.__revelDiscarded || [])).length;
    await page.click('.import-published');
    await page.waitForSelector('.import-summary');
    await closeSummary();
    const after = (await page.evaluate(() => window.__revelDiscarded || [])).length;
    check('import : site publié importé depuis son adresse, copie du dépôt retirée',
      after === before + 1 && await page.locator('.site-card', { hasText: 'p3r.deltea.space' }).count() === 1);
    // Échec de construction : message et journal dans la boîte.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'https://github.com/exemple/svelte-echec');
    await page.click('.import-run');
    await page.waitForSelector('.import-build:not([disabled])');
    await page.click('.import-build');
    await page.click('.import-build-run');
    await page.waitForSelector('.import-error .import-build-log');
    const failText = (await page.textContent('.import-error > span')).trim();
    await page.click('.import-error .import-build-log summary');
    const failLog = (await page.textContent('.import-error pre')).trim();
    await page.click('.import-cancel');
    await page.waitForFunction(() => !document.querySelector('.import-dialog'));
    check('import : échec de construction expliqué, journal affiché', failText === 'Échec de la construction (code 1).' && /Cannot find module/.test(failLog), failText);
    // Adresse d'un dossier sur GitHub : copie puis import direct de ce dossier.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'github.com/exemple/vitrine/tree/main/docs');
    await page.press('.import-url', 'Enter');
    await closeSummary();
    check('import : dossier nommé dans l\'adresse du dépôt, importé directement', await page.locator('.site-card', { hasText: 'Docs' }).count() === 1);
    // Adresse non reconnue : message, boîte gardée.
    await openImport();
    await page.click('.import-sources .import-tile[data-source="git"]');
    await page.fill('.import-url', 'ftp://exemple/depot');
    await page.click('.import-run');
    await page.waitForSelector('.import-dialog .import-error');
    check('import : adresse non reconnue signalée dans la boîte', (await page.textContent('.import-error')).trim() === 'Adresse de dépôt non reconnue.');
    // Figma : jeton demandé, fichier lu, cadres de la première page choisis, import des cadres
    // choisis ; jeton gardé puis oublié.
    await page.click('.import-sources .import-tile[data-source="figma"]');
    const figOff = await page.$eval('.import-run', (b) => b.disabled);
    const readLabel = (await page.textContent('.import-run')).trim();
    await page.fill('.import-figma-url', 'https://www.figma.com/design/AbCdEf1234567890/Maquette');
    const noToken = await page.$eval('.import-run', (b) => b.disabled);
    await page.fill('.import-figma-token', 'mauvais');
    await page.click('.import-run');
    await page.waitForSelector('.import-dialog .import-error');
    const badToken = (await page.textContent('.import-error')).trim();
    await page.fill('.import-figma-token', 'figd_essai');
    await page.click('.import-run');
    await page.waitForSelector('.import-frames');
    const count = (await page.textContent('.import-figma-count')).trim();
    const saved = await page.locator('.import-figma-saved .chip--ok').count() === 1;
    await page.locator('.import-frame', { hasText: 'Contact' }).locator('input').uncheck();
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-figma-cadres.png` });
    await page.click('.import-run');
    await page.waitForSelector('.import-summary');
    const frames = await page.evaluate(() => window.__revelFigmaFrames);
    await closeSummary();
    await openImport();
    await page.click('.import-sources .import-tile[data-source="figma"]');
    await page.waitForSelector('.import-figma-forget');
    await page.click('.import-figma-forget');
    await page.waitForSelector('.import-figma-token');
    check('import : Figma (jeton refusé signalé, cadres lus et choisis, jeton gardé puis oublié)',
      figOff && readLabel === 'Lire le fichier' && noToken && badToken === 'Jeton Figma absent ou refusé.' && count === '3 / 4' && saved && frames.join() === '1:1,1:2',
      `${readLabel} · ${badToken} · ${count} · ${frames}`);
    // WordPress : fichier examiné, images non téléchargées sur demande.
    await page.click('.import-sources .import-tile[data-source="wordpress"]');
    await page.evaluate(() => { window.__revelPickFiles = ['C:\\Exports\\atelier.WordPress.2026-10-17.xml']; });
    await page.click('.import-choose-wxr');
    await page.waitForSelector('.import-wp-probe');
    const counts = (await page.textContent('.import-wp-probe')).replace(/\s+/g, ' ').trim();
    await page.waitForFunction(() => /atelier-lumen$/.test(document.querySelector('.import-target')?.textContent.trim() || ''));
    await page.click('.import-wp-media-row .switch-track');
    await page.waitForFunction(() => !document.querySelector('.import-wp-media').checked);
    if (shots) await page.locator('.import-dialog').screenshot({ path: `${shots}/import-wordpress.png` });
    await page.click('.import-run');
    await page.waitForSelector('.import-summary');
    const media = await page.evaluate(() => window.__revelWpMedia);
    const posts = await stat('import-stat-entries');
    await closeSummary();
    check('import : WordPress (fichier examiné, images gardées en ligne sur demande, articles repris)',
      /Atelier Lumen.*6 pages · 48 articles · 120 médias/.test(counts) && media === false && posts === '48', `${counts} · ${media} · ${posts}`);
    // Liste des sites d'origine pour la suite des essais.
    await page.reload();
    await page.waitForSelector('.site-card');
  }
  // Un clic sur un site l'ouvre dans l'éditeur.
  await page.click('.site-open');
  await page.waitForSelector('iframe.canvas-frame');
  const frameOf = () => page.frames().find((f) => f.url().includes('rv='));
  const waitFrame = async (pred, what, arg, tries = 150) => {
    for (let i = 0; i < tries; i++) {
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
  const techs = await page.$$eval('.editor-status .node-crumb', (l) => l.map((c) => c.dataset.tech).join(' › '));
  check('clic dans le canevas : élément et propriétaire affichés', /h1|span|strong/.test(techs) && /(Page|Gabarit|Composant) /.test(status), `${status.trim().replace(/\s+/g, ' ')} (${techs})`);

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

  // Attribut modifié (classe, animation, lien…) : posé en direct, sans recharger la page.
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
  f = await waitFrame(() => document.querySelector('h1')?.getAttribute('data-essai') === 'oui', 'attribut posé');
  check('attribut modifié : posé en direct, sans rechargement', await f.evaluate(() => window.__mark === 1));
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame(() => document.readyState === 'complete' && document.querySelector('h1') && !document.querySelector('h1').hasAttribute('data-essai'), 'annulation');
  check('attribut annulé : retiré en direct, sans rechargement', await f.evaluate(() => window.__mark === 1));
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
  await page.click('.layers .layer[data-tech*="section.hero"]');
  await page.waitForFunction(() => [...document.querySelectorAll('.editor-status .node-crumb')].some((c) => c.dataset.tech.startsWith('section.hero')));
  check('calques : ligne cliquée sélectionnée dans le canevas', true);
  // Masquer (Ctrl+Maj+H) : l'élément disparaît du canevas aussitôt, sans recharger ; annulé.
  await f.evaluate(() => { window.__hid = 1; });
  await page.keyboard.press('Control+Shift+H');
  f = await waitFrame(() => getComputedStyle(document.querySelector('section.hero')).display === 'none', 'calque masqué');
  check('calque masqué : caché dans le canevas, sans rechargement', await f.evaluate(() => window.__hid === 1));
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame(() => getComputedStyle(document.querySelector('section.hero')).display !== 'none', 'calque montré');

  const docBefore = await invoke('edit_open');
  const bodyOf = (site) => JSON.stringify(site.pages.find((p) => p.id === 'index').body);
  // Dupliquer (bouton), puis annuler.
  await page.click('.layers .layers-foot .icon-btn >> nth=5');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 2, 'section dupliquée');
  const dup = await invoke('edit_open');
  check('dupliquer : section copiée (nouveaux identifiants), sélectionnée', bodyOf(dup.site).length > bodyOf(docBefore.site).length
    && (await page.$$eval('.editor-status .node-crumb', (l) => l.some((c) => c.dataset.tech.startsWith('section.hero')))));
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame(() => document.querySelectorAll('section.hero').length === 1, 'annulation de la copie');

  // Descendre (bouton) : la section passe après sa voisine.
  await page.click('.layers .layer[data-tech*="section.hero"]');
  const orderBefore = await f.evaluate(() => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(','));
  await page.click('.layers .layers-foot .icon-btn >> nth=3');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') !== ob, 'déplacement', orderBefore);
  check('descendre : ordre modifié dans le canevas', true);
  await page.click('.editor-toolbar .btn-undo');
  f = await waitFrame((ob) => [...document.querySelector('main').children].map((e) => e.tagName + '.' + e.className.split(' ')[0]).join(',') === ob, 'annulation du déplacement', orderBefore);

  // Glisser-déposer : section.hero (repliée) déposée après la section suivante.
  await page.click('.layers .layer[data-tech*="section.hero"] .layer-toggle');
  await page.waitForTimeout(150);
  const info = await page.$$eval('.layers .layer', (rs) => rs.map((e) => [e.dataset.tech, parseInt(e.dataset.depth, 10)]));
  const si = info.findIndex(([t]) => t === 'section.hero' || t.startsWith('section.hero.'));
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
  await page.click('.layers .layer[data-tech*="section.hero"]');
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
  await page.click('.layers .layer[data-tech*="section.hero"]');
  // Réglages de l'élément : rubrique « Paramètres avancés » de l'onglet Styles.
  await page.click('.dock-tab[data-panel="styles"]');
  await page.click('.advanced-settings > summary');
  await page.waitForFunction(() => document.querySelector('.advanced-settings .field select.input')?.value === 'section');
  check('réglages : balise de l\'élément sélectionné (paramètres avancés de l\'onglet Styles)', !(await page.$('.dock-tab[data-panel="settings"]')));
  const secId = await f.evaluate(() => document.querySelector('section.hero').getAttribute('data-rv'));
  // Identifiant (Entrée).
  const idInput = page.locator('.advanced-settings .field >> nth=1').locator('input');
  await idInput.fill('essai-id');
  await idInput.press('Enter');
  f = await waitFrame(() => !!document.querySelector('section#essai-id'), 'identifiant');
  check('réglages : identifiant écrit dans la page', true);
  // Classe ajoutée puis retirée (pastilles de l'onglet Styles).
  await page.fill('.dock-panel[data-panel="styles"] .selector-add', 'essai-classe');
  await page.press('.dock-panel[data-panel="styles"] .selector-add', 'Enter');
  f = await waitFrame(() => document.querySelector('section.hero')?.classList.contains('essai-classe'), 'classe ajoutée');
  const cls = nodeById((await invoke('edit_open')).site, secId).attrs.find((a) => a.name === 'class').value;
  check('réglages : classe ajoutée', cls.split(' ').includes('essai-classe') && cls.split(' ')[0] === 'hero', cls);
  await page.click('.dock-panel[data-panel="styles"] .sel-chip:has(.sel-chip-name:text-is("essai-classe")) .sel-chip-x');
  f = await waitFrame(() => !document.querySelector('section.hero')?.classList.contains('essai-classe'), 'classe retirée');
  check('réglages : classe retirée', true);

  // Ajouter un titre dans la section sélectionnée.
  await page.click('.dock-tab[data-panel="insert"]');
  await page.dblclick('.insert-item:has-text("Titre")');
  f = await waitFrame(() => [...document.querySelectorAll('section.hero h2')].some((h) => h.textContent === 'Nouveau titre'), 'titre ajouté');
  await page.waitForFunction(() => document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech === 'h2');
  check('ajouter : titre inséré dans la section, sélectionné', true);
  await page.click('.dock-tab[data-panel="layers"]');

  // Lien : nouvel onglet (target + rel). Le lien est dans le composant de l'en-tête : un clic
  // sélectionne l'utilisation ; « Modifier » ouvre le canevas du composant, où le lien se
  // sélectionne directement.
  await page.frameLocator('iframe.canvas-frame').locator('header nav a', { hasText: 'Agence' }).first().click();
  await page.waitForSelector('.dock-panel[data-panel="styles"] .instance-name');
  check('composant : un clic sélectionne l\'utilisation entière', (await page.textContent('.dock-panel[data-panel="styles"] .instance-name')).trim() === 'Header');
  await page.click('.dock-panel[data-panel="styles"] .instance-actions .btn >> nth=0');
  await page.waitForSelector('.comp-bar');
  await page.waitForSelector('iframe.canvas-frame[data-key="@"]');
  await page.waitForTimeout(800);
  await page.frameLocator('iframe.canvas-frame[data-key="@"]').locator('header nav a', { hasText: 'Agence' }).first().click();
  await page.waitForSelector('.dock-panel[data-panel="styles"] .row-switch');
  check('composant : canevas du composant ouvert (variante principale), lien sélectionné', !!(await page.$('.comp-bar')));
  await page.click('.dock-panel[data-panel="styles"] .row-switch .switch');
  f = await waitFrame(() => [...document.querySelectorAll('header nav a')].some((a) => a.textContent.trim() === 'Agence' && a.target === '_blank' && a.rel === 'noopener'), 'nouvel onglet');
  check('lien : ouverture dans un nouvel onglet', true);
  await page.click('.comp-back');
  await page.waitForFunction(() => !document.querySelector('.comp-bar') && !!document.querySelector('iframe.canvas-frame[data-key="desktop"]'));

  // Image : champs de l'image.
  await page.locator('.layers .layer-label', { hasText: /^Image$/ }).first().click();
  await page.waitForFunction(() => /Texte alternatif|Image \(/.test(document.querySelector('.dock-panel[data-panel="styles"] .content-settings')?.textContent || ''));
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
    await page.click('.dock-tab[data-panel="interactions"]');
    await page.click('.dock-col--right .dock-group:has(.dock-tab[data-panel="interactions"]) .dock-menu-btn');
    await page.click('.dock-menu .menu-item:has-text("Fermer")');
    check('espace de travail : panneau fermé', !(await page.$('.dock-tab[data-panel="interactions"]')));
    await page.click('.panels-menu > summary');
    await page.click('.panels-pop .menu-item--check:has-text("Interactions")');
    check('espace de travail : panneau rouvert', !!(await page.$('.dock-tab[data-panel="interactions"]')));
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
    const found = await page.$$eval('.layers .layer', (r) => r.map((e) => e.dataset.tech));
    check('calques : recherche (lignes trouvées et leurs parents)', found.length < n0 && found.some((t) => t.startsWith('h1.hero__title')), `${n0} → ${found.length}`);
    await page.fill('.layers .panel-search input', '');
    await page.click('.layers .layer[data-tech*="h1.hero__title"]');
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(200);
    const after = await page.getAttribute('.layers .layer.is-selected', 'data-tech');
    check('calques : flèche vers le bas, élément suivant', !after.startsWith('h1.hero__title'), after);
    await page.click('.layers .layer[data-tech*="h1.hero__title"]', { button: 'right' });
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
    await page.click('.layers .layer[data-tech*="h1.hero__title"]');
    await page.click('.node-path .node-crumb[data-tech*="section.hero"]');
    await page.waitForTimeout(200);
    check('fil d\'Ariane : parent sélectionné', (await page.getAttribute('.layers .layer.is-selected', 'data-tech')).startsWith('section.hero'));
    // Composant dans les calques : une ligne sans contenu ; double-clic : son canevas.
    {
      const comps = await page.$$eval('.layers .layer.is-component', (l) => l.map((r) => !!r.querySelector('.layer-toggle.is-hidden')));
      if (comps.length) {
        await page.dblclick('.layers .layer.is-component >> nth=0');
        await page.waitForSelector('.canvas-area.is-comp');
        await page.click('.comp-back');
        await page.waitForSelector('.canvas-area:not(.is-comp)');
        f = await waitFrame(() => document.readyState === 'complete' && !!window.__revelSlot, 'retour à la page');
      }
      check('calques : composant non dépliable, double-clic ouvre son canevas', comps.every(Boolean), `${comps.length} composant(s)`);
      await page.click('.layers .layer[data-tech*="section.hero"]');
    }
    // Interactions (6.1) : animation prête à l'emploi ajoutée, changée, réglée, essayée dans le
    // canevas, retirée ; document identique après annulation.
    {
      const docA = JSON.stringify((await invoke('edit_open')).site);
      await page.click('.dock-tab[data-panel="interactions"]');
      await page.click('.anim-family[data-family="appear"] .anim-add');
      f = await waitFrame((v) => document.querySelector('section.hero')?.getAttribute('data-revel-appear') === v, 'apparition ajoutée', 'fade');
      await page.click('.anim-family[data-family="appear"] .anim-tile[data-preset="up"]');
      f = await waitFrame((v) => document.querySelector('section.hero')?.getAttribute('data-revel-appear') === v, 'animation changée', 'up');
      const dur = page.locator('.anim-family[data-family="appear"] [data-setting="dur"] input');
      await dur.fill('1.2');
      await dur.press('Enter');
      f = await waitFrame((v) => document.querySelector('section.hero')?.getAttribute('data-revel-appear') === v, 'durée réglée', 'up dur=1.2');
      check('interactions : apparition ajoutée, changée, durée réglée', true);
      await page.click('.anim-family[data-family="appear"] .anim-play');
      f = await waitFrame(() => (document.querySelector('section.hero')?.getAnimations().length || 0) > 0, 'aperçu de l\'animation');
      check('interactions : aperçu joué dans le canevas', true);
      if (shots) await page.screenshot({ path: `${shots}/editeur-interactions.png` });
      await page.click('.anim-family[data-family="appear"] .anim-remove');
      f = await waitFrame(() => !document.querySelector('section.hero')?.hasAttribute('data-revel-appear'), 'animation retirée');
      for (let i = 0; i < 6 && JSON.stringify((await invoke('edit_open')).site) !== docA; i++) {
        await page.click('.editor-toolbar .btn-undo');
        await page.waitForTimeout(400);
      }
      check('interactions : retirée puis tout annulé, document identique', JSON.stringify((await invoke('edit_open')).site) === docA);
      // Actions (6.3) : un appui ajoute une classe ; déclencheur changé ; tout annulé.
      await page.click('.act-family .anim-add');
      f = await waitFrame((v) => document.querySelector('section.hero')?.getAttribute('data-revel-on') === v, 'action ajoutée', 'click class:is-open self');
      await page.locator('.act-card select').nth(0).selectOption('hover');
      f = await waitFrame((v) => document.querySelector('section.hero')?.getAttribute('data-revel-on') === v, 'déclencheur changé', 'hover class:is-open self');
      check('interactions : action ajoutée (appui, classe), déclencheur changé (survol)', true);
      for (let i = 0; i < 4 && JSON.stringify((await invoke('edit_open')).site) !== docA; i++) {
        await page.click('.editor-toolbar .btn-undo');
        await page.waitForTimeout(400);
      }
      check('interactions : action annulée, document identique', JSON.stringify((await invoke('edit_open')).site) === docA);
      await page.click('.dock-tab[data-panel="styles"]');
    }
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
    await page.waitForSelector('.notice--error', { state: 'detached' });
    await page.click(canon + ' .var-head');
    await page.waitForSelector(canon + '.is-open .var-expr');
    await page.fill(canon + ' .var-expr', '"https://essai.example/"');
    await page.waitForFunction((sel) => document.querySelector(sel + ' .var-expr')?.value === '"https://essai.example/"', canon);
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
      await page.click('.dock-tab[data-panel="styles"]');
      await page.click('.dock-panel[data-panel="styles"] .content-settings .field-line .icon-btn');
      await page.waitForSelector('.picker .asset[data-url="/media/essai-depot.png"]');
      const onlyImages = await page.$$eval('.picker .asset', (a) => a.every((x) => /\.(png|jpe?g|gif|webp|avif|svg)$/i.test(x.dataset.url)));
      if (shots) await page.screenshot({ path: `${shots}/editeur-choix-image.png` });
      await page.click('.picker .asset[data-url="/media/essai-depot.png"]');
      const picked = await until(async () => (await placed()).includes('"src":"\\"/media/essai-depot.png\\""'));
      check('choix d\'une image : fenêtre (images seulement), image choisie, fenêtre fermée', onlyImages && picked && !(await page.$('.picker')));
      await page.click('.dock-panel[data-panel="styles"] .content-settings .field-line .icon-btn');
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
      await page.click('.dock-tab[data-panel="styles"]');
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
      await page.click('.layers .layer[data-tech*="section.intro-agency"] .layer-label');
      await page.click('.dock-tab[data-panel="insert"]');
      await page.dblclick('.insert-item[data-item="insert-collection-list"]');
      const listed = await untilE(async () => (await frame.locator('.liste-collection__element').count()) > 2);
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.collection-list select');
      await page.selectOption('.collection-list select[data-list="sort"]', 'date');
      await untilE(async () => (await pageJson()).includes('sort(attribute=\\"date\\", reverse=true)'));
      await page.fill('.collection-list input[type=number]', '2');
      await page.keyboard.press('Tab');
      const two = await untilE(async () => (await frame.locator('.liste-collection__element').count()) === 2);
      check('éléments : liste de collection (collection, tri par date décroissante, 2 contenus)', listed && two
        && (await pageJson()).includes('[:2]'));
      // Liste de plusieurs collections (D-97) : « + Autre collection », puis retirée.
      const others2 = await page.$$eval('.list-also select option', (o) => o.map((x) => x.value).filter(Boolean));
      await page.selectOption('.list-also select', others2[0]);
      const multi = await untilE(async () => (await pageJson()).includes(` + collections.${others2[0]})`));
      await page.waitForSelector('.list-also .prop-chip');
      check('éléments : liste de collection à deux collections (+ Autre collection)', multi, others2[0]);
      await page.click('.list-also .prop-chip .chip-x');
      check('éléments : seconde collection retirée', await untilE(async () => !(await pageJson()).includes(' + collections.')));
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
      await page.click('.layers .layer[data-tech*="section.intro-agency"] .layer-label');
      await page.click('.dock-tab[data-panel="insert"]');
      // Un clic seul n'ajoute rien ; le double-clic ajoute, page mise à jour sur place (pas de
      // rechargement : animations non rejouées).
      await frameOf().evaluate(() => { window.__sansRechargement = 1; });
      const before1 = await pageJson();
      await page.click('.insert-item[data-item="insert-grid"]');
      await page.waitForTimeout(500);
      const oneClick = (await pageJson()) === before1;
      await page.dblclick('.insert-item[data-item="insert-grid"]');
      const grid = await untilE(async () => (await frame.locator('.grille .grille__cellule').count()) === 2);
      check('ajouter : un clic n\'ajoute rien, un double-clic ajoute', oneClick && grid);
      check('canevas : page mise à jour sur place après un ajout (sans rechargement)',
        await frameOf().evaluate(() => window.__sansRechargement === 1));
      const gridCss = await frame.locator('.grille').evaluate((g) => getComputedStyle(g).display);
      check('éléments : grille insérée avec ses styles', grid && gridCss === 'grid', gridCss);
      // Icône : dessin choisi dans les réglages.
      await page.dblclick('.insert-item[data-item="insert-icon"]');
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.icon-choice[data-icon="coeur"]');
      await page.click('.icon-choice[data-icon="coeur"]');
      const heart = await untilE(async () => (await frame.locator('svg.icone[data-icon="coeur"]').count()) === 1);
      check('éléments : icône insérée, dessin changé', heart);
      // Code HTML écrit tel quel.
      await page.click('.dock-tab[data-panel="insert"]');
      await page.dblclick('.insert-item[data-item="insert-embed"]');
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.embed-code');
      await page.fill('.embed-code', '<p class="essai-code">Bonjour</p>');
      await page.locator('.embed-code').blur();
      const embed = await untilE(async () => (await frame.locator('.code-html .essai-code').count()) === 1);
      check('éléments : code HTML écrit tel quel', embed);
      // Bloc prêt à l'emploi : section complète avec ses icônes et ses styles.
      await page.click('.dock-tab[data-panel="layers"]');
      await page.click('.layers .layer[data-tech*="section.intro-agency"] .layer-label');
      await page.click('.dock-tab[data-panel="insert"]');
      await page.locator('.insert-item[data-item="insert-block-features"]').scrollIntoViewIfNeeded();
      await page.dblclick('.insert-item[data-item="insert-block-features"]');
      const block = await untilE(async () => (await frame.locator('.bloc-avantages .bloc-avantage .icone').count()) === 3);
      const blockCss = block && await frame.locator('.bloc-avantages__grille').evaluate((g) => getComputedStyle(g).display);
      check('éléments : bloc prêt à l\'emploi (avantages, icônes, styles)', block && blockCss === 'grid', String(blockCss));
      // Onglets et accordéon (6.4), animation Lottie (6.2), ajoutés depuis le panneau Ajouter.
      const addIn = async (key) => {
        await page.click('.dock-tab[data-panel="layers"]');
        await page.click('.layers .layer[data-tech*="section.intro-agency"] .layer-label');
        await page.click('.dock-tab[data-panel="insert"]');
        await page.locator(`.insert-item[data-item="insert-${key}"]`).scrollIntoViewIfNeeded();
        await page.dblclick(`.insert-item[data-item="insert-${key}"]`);
      };
      // Élément vide : icône et hauteur d'environ 50 px dans le canevas seulement.
      await addIn('container');
      const empty = await untilE(async () => (await frame.locator('[data-rv-empty="box"]').count()) > 0);
      const emptyH = empty ? await frame.locator('[data-rv-empty="box"]').first().evaluate((e) => e.getBoundingClientRect().height) : 0;
      check('canevas : élément vide montré (icône, environ 50 px de haut)', empty && emptyH >= 48, String(emptyH));
      await addIn('tabs');
      const tabsIn = await untilE(async () => (await frame.locator('[data-revel-tabs] [data-revel-tab]').count()) === 2
        && (await frame.locator('[data-revel-tabs] [data-revel-panel]').count()) === 2);
      // Élément dans les onglets sans rôle : un seul choix « Rôle » (groupé par élément
      // interactif) ; une fois choisi, les réglages de ce rôle.
      await frame.locator('[data-revel-tabs] [data-revel-panel] p').first().click();
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.insp-group[data-role="picker"] select');
      const groups = await page.$$eval('.insp-group[data-role="picker"] optgroup', (g) => g.map((x) => x.label));
      const others = await page.$$eval('.content-settings .insp-group:not([data-role="picker"]) select option', (o) => o.map((x) => x.textContent).filter((t) => /Onglet|Panneau/.test(t)));
      check('rôle : un seul choix pour un élément sans rôle (groupé)', groups.includes('Dans les onglets') && others.length === 0, groups.join(', '));
      await page.locator('.insp-group[data-role="picker"] select').selectOption({ label: 'Panneau' });
      const panelSet = await untilE(async () => (await frame.locator('[data-revel-panel] p[data-revel-panel]').count()) === 1);
      await page.waitForFunction(() => !document.querySelector('.insp-group[data-role="picker"]'));
      check('rôle : choisi (attribut posé), réglages du rôle à la place du choix', panelSet);
      await addIn('accordion');
      const accIn = await untilE(async () => (await frame.locator('details.accordeon > summary.accordeon__titre').count()) === 1);
      check('éléments : onglets (deux boutons, deux panneaux), accordéon', tabsIn && accIn);
      await addIn('lottie');
      const lottieIn = await untilE(async () => (await frame.locator('.lottie[data-lottie]').count()) === 1);
      await page.click('.dock-tab[data-panel="styles"]');
      await page.locator('.styles label.field:has(> .label:text-is("Lecture")) select').selectOption('hover');
      const hover = await untilE(async () => (await frame.locator('.lottie[data-lottie-play="hover"]').count()) === 1);
      check('éléments : animation Lottie ajoutée, lecture au survol', lottieIn && hover);
      // Média au survol (D-99) : liste de collection et média ajoutés ; le média piloté par la
      // liste choisie comme source ; éléments survolés listés et sélectionnables.
      await addIn('collection-list');
      const listIn = await untilE(async () => (await frame.locator('section.intro-agency .liste-collection a').count()) > 0);
      await addIn('hover-media');
      const mediaIn = await untilE(async () => (await frame.locator('video.media-survol').count()) === 1);
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.hover-media[data-hover="display"] [data-hover="source"] select');
      const listOpt = await page.$eval('.hover-media[data-hover="display"] [data-hover="source"] select',
        (sel) => [...sel.options].map((o) => o.value).find((v) => v.startsWith('list:')));
      await page.selectOption('.hover-media[data-hover="display"] [data-hover="source"] select', listOpt);
      const linked = await untilE(async () => (await frame.locator('.liste-collection a[data-revel-media-for]').count()) > 0);
      await page.waitForSelector('.hover-media[data-hover="display"] .hover-row');
      check('média au survol : piloté par la liste choisie, éléments survolés listés', listIn && mediaIn && linked, String(listOpt));
      if (shots) await page.locator('.hover-media[data-hover="display"]').screenshot({ path: `${shots}/editeur-media-survol.png` });
      await page.click('.hover-media[data-hover="display"] .hover-row .icon-btn');
      await page.waitForSelector('.hover-media[data-hover="trigger"] [data-hover="shown-in"] select');
      check('média au survol : élément survolé sélectionné, média où il s\'affiche', true);
      // Champ de la vidéo : seulement les champs qui en donnent une (fichier, adresse de vidéo).
      const srcOpts = await page.$$eval('.hover-media [data-hover="src-field"] select option', (o) => o.map((x) => x.value).filter(Boolean));
      const noField = await page.$('.hover-media [data-hover="no-field"]');
      check('média au survol : seuls les champs qui donnent une vidéo proposés',
        (srcOpts.length > 0 || !!noField) && !srcOpts.some((v) => /^(title|titre|slug|client|description)$/.test(v)), srcOpts.join(', ') || 'aucun (message)');
      await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
      for (let i = 0; i < 30; i++) {
        if (JSON.stringify(await siteE()) === docE) break;
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(300);
      }
      check('éléments : Ctrl+Z, document identique', await untilE(async () => JSON.stringify(await siteE()) === docE));
    }
    // Tri et filtre des médias : liste ouverte aux couleurs de Revel (sélecteur personnalisable).
    {
      await page.click('.dock-tab[data-panel="assets"]').catch(() => {});
      const sel = await page.$('.media-tools .mini-select select');
      if (sel) {
        const st = await sel.evaluate((s) => ({ supported: CSS.supports('appearance', 'base-select'), appearance: getComputedStyle(s).appearance }));
        check('médias : tri et filtre en listes de Revel (pas la liste du système)', !st.supported || st.appearance === 'base-select', JSON.stringify(st));
      }
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
    // Propriétés du gabarit (D-96) : ajoutée depuis les réglages d'une page (au gabarit le plus
    // extérieur), valeur de cette page, élément du gabarit relié (visibilité), autres pages à la
    // valeur par défaut ; tout annulé.
    {
      const site96 = () => invoke('edit_open').then((o) => o.site);
      const doc96 = JSON.stringify(await site96());
      await page.click('.dock-tab[data-panel="pages"]');
      await page.hover('.pages .page-row[data-url="/agence/"]');
      await page.click('.pages .page-row[data-url="/agence/"] .row-action');
      await page.waitForSelector('.page-settings .page-settings-section-head');
      await page.click('.page-settings-section-head .icon-btn');
      await page.click('.ctx-menu .menu-item:has-text("Visibilité")');
      await page.waitForSelector('.layout-prop');
      const base96 = (s) => s.layouts.find((l) => l.id === 'layouts/base');
      const f96 = (base96(await site96()).fields || [])[0];
      check('gabarit : propriété ajoutée depuis la page (gabarit du site, visibilité)', !!f96 && f96.type === 'switch');
      await page.click('.layout-prop .switch');
      await page.waitForFunction(() => document.querySelector('.layout-prop')?.classList.contains('is-set'));
      const ag96 = (await site96()).pages.find((p) => p.path === '/agence/');
      check('gabarit : valeur choisie pour la page', ag96.props[f96.name] === true);
      if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-gabarit-proprietes.png` });
      await page.click('.page-settings-head .icon-btn');
      await page.click('.pages .page-row[data-url="/agence/"]');
      await waitFrame(() => location.pathname === '/agence/', 'agence');
      await page.click('.dock-tab[data-panel="layers"]');
      await page.locator('.layers .layer[data-tech="main"]').first().click({ position: { x: 60, y: 8 } });
      await page.click('.dock-tab[data-panel="styles"]');
      await page.waitForSelector('.component-bindings .bind-row[data-bind="component-bind-visibility"] .bind-btn');
      await page.click('.component-bindings .bind-row[data-bind="component-bind-visibility"] .bind-btn');
      await page.locator('.ctx-menu .menu-item', { hasText: /^Visibilité$/ }).click();
      let body96 = '';
      for (let i = 0; i < 20 && !body96.includes(`"cond":"${f96.name}"`); i++) {
        await page.waitForTimeout(250);
        body96 = JSON.stringify(base96(await site96()).body);
      }
      check('gabarit : élément relié à la propriété (condition dans le gabarit)', body96.includes(`"cond":"${f96.name}"`));
      await waitFrame(() => !!document.querySelector('main#contenu'), 'contenu affiché (valeur de la page)');
      await page.click('.dock-tab[data-panel="pages"]');
      await page.click('.pages .page-row[data-url="/contact/"]');
      await waitFrame(() => location.pathname === '/contact/' && !document.querySelector('main#contenu'), 'masqué ailleurs (valeur par défaut)');
      check('gabarit : affiché sur la page qui l\'active, masqué ailleurs', true);
      for (let i = 0; i < 6 && JSON.stringify(await site96()) !== doc96; i++) {
        await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(400);
      }
      check('gabarit : tout annulé, document identique', JSON.stringify(await site96()) === doc96);
      await page.click('.pages .page-row[data-url="/"]');
      await waitFrame(() => location.pathname === '/', 'accueil');
    }
    // Fichier du serveur (7.10) : `.htaccess` montré avec ses balises (conditions sur
    // l'environnement, boucles sur les contenus), modifié comme texte et relu ; texte mal fermé
    // refusé avec sa raison ; annulé.
    {
      const siteF = () => invoke('edit_open').then((o) => o.site);
      const docF = JSON.stringify(await siteF());
      const bodyF = async () => JSON.stringify((await siteF()).pages.find((p) => p.path === '/.htaccess').body);
      await page.click('.dock-tab[data-panel="pages"]');
      await page.hover('.pages .page-row[data-url="/.htaccess"]');
      await page.click('.pages .page-row[data-url="/.htaccess"] .row-action');
      await page.waitForSelector('.page-settings-file textarea');
      const ta = page.locator('.page-settings-file textarea');
      const text0 = await ta.inputValue();
      check('fichier du serveur : texte avec ses balises (conditions, boucles), modifiable',
        await ta.isEnabled() && text0.includes('{% if env.staging %}') && /\{% for \w+ in collections\./.test(text0), `${text0.length} caractères`);
      await ta.fill(text0 + '\n# Essai Revel {{ site.url }}\n');
      await ta.blur();
      let b1 = '';
      for (let i = 0; i < 20 && !b1.includes('Essai Revel'); i++) { await page.waitForTimeout(250); b1 = await bodyF(); }
      check('fichier du serveur : texte relu (valeur, conditions et boucles gardées)',
        b1.includes('Essai Revel') && b1.includes('"expr":"site.url"') && b1.includes('"node":"each"') && b1.includes('"cond":"env.staging"'));
      if (shots) await page.locator('.dock-col--left').screenshot({ path: `${shots}/editeur-fichier-serveur.png` });
      await ta.fill(text0 + '{% for x in %}');
      await ta.blur();
      await page.waitForSelector('.page-settings-file .field-error');
      check('fichier du serveur : texte illisible refusé, raison affichée', (await bodyF()) === b1,
        (await page.textContent('.page-settings-file .field-error')).trim().slice(0, 80));
      await page.click('.page-settings-head .icon-btn');
      for (let i = 0; i < 4 && JSON.stringify(await siteF()) !== docF; i++) {
        await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(400);
      }
      check('fichier du serveur : annulé, document identique', JSON.stringify(await siteF()) === docF);
      await page.click('.pages .page-row[data-url="/"]');
      await waitFrame(() => location.pathname === '/', 'accueil');
    }
    // Valeur reliée d'un attribut (Inspecteur › Attributs) : attribut écrit relié à une
    // information du site, texte autour, « seulement si remplie » ; délié ; annulé.
    {
      const siteB = () => invoke('edit_open').then((o) => o.site);
      const docB = JSON.stringify(await siteB());
      // Titre du modèle des projets : champs du contenu proposés.
      await page.click('.dock-tab[data-panel="pages"]');
      await page.click('.pages .collection-row[data-collection="projets"]');
      await waitFrame(() => location.pathname.startsWith('/project/') && !!document.querySelector('.related .section-title'), 'modèle des projets');
      await page.frameLocator('iframe.canvas-frame').locator('.related .section-title').first().click();
      await page.click('.dock-tab[data-panel="styles"]');
      if (!(await page.$('.advanced-settings[open]'))) await page.click('.advanced-settings > summary');
      const secB = await frameOf().evaluate(() => document.querySelector('.related .section-title').getAttribute('data-rv'));
      const attrB = async () => (nodeById(await siteB(), secB).attrs || []).find((a) => a.name === 'aria-label')?.value;
      await page.fill('.insp-attrs > input.input--mono', 'aria-label=Écrire à');
      await page.press('.insp-attrs > input.input--mono', 'Enter');
      await page.waitForSelector('.attr-row:has(.attr-name:text-is("aria-label")) .attr-actions .icon-btn >> nth=0');
      await page.click('.attr-row:has(.attr-name:text-is("aria-label")) .attr-actions .icon-btn >> nth=0');
      await page.waitForSelector('.bound-attr');
      let v1 = '';
      for (let i = 0; i < 20 && !(v1 || '').includes('{{'); i++) { await page.waitForTimeout(200); v1 = await attrB(); }
      check('attribut relié : texte écrit gardé devant, champ du contenu relié', /^Écrire à\{\{ [\w.]+ \}\}$/.test(v1 || ''), v1);
      await page.click('.bound-attr .view-line .switch');
      let v2 = '';
      for (let i = 0; i < 20 && !(v2 || '').includes('{% if'); i++) { await page.waitForTimeout(200); v2 = await attrB(); }
      check('attribut relié : seulement si la valeur est remplie', /^\{% if [\w.]+ %\}Écrire à\{\{ [\w.]+ \}\}\{% endif %\}$/.test(v2 || ''), v2);
      if (shots) await page.locator('.insp-attrs').screenshot({ path: `${shots}/editeur-attribut-relie.png` });
      await page.click('.attr-row:has(.attr-name:text-is("aria-label")) .attr-actions .icon-btn >> nth=0');
      let v3 = '';
      for (let i = 0; i < 20 && (v3 || '{').includes('{'); i++) { await page.waitForTimeout(200); v3 = await attrB(); }
      check('attribut relié : délié (texte écrit à la place)', v3 === '', String(v3));
      for (let i = 0; i < 8 && JSON.stringify(await siteB()) !== docB; i++) {
        await page.click('.canvas-area', { position: { x: 5, y: 5 } }).catch(() => {});
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(350);
      }
      check('attribut relié : annulé, document identique', JSON.stringify(await siteB()) === docB);
      await page.click('.dock-tab[data-panel="pages"]');
      await page.click('.pages .page-row[data-url="/"]');
      await waitFrame(() => location.pathname === '/', 'accueil');
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
    const crumb = () => page.$eval('.editor-status .node-crumb.is-on', (e) => e.dataset.tech);
    await canvas.locator('.intro-agency__links a').first().click();
    await page.waitForFunction(() => /^a\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech || ''));
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => /button--ghost|a\.button$/.test(document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech || '') && document.querySelectorAll('.editor-status .node-crumb').length > 0);
    const sib = await crumb();
    await page.keyboard.press('Shift+Enter');
    await page.waitForFunction(() => /^p\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech || ''));
    const par = await crumb();
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => /^a\./.test(document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech || ''));
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
    await page.waitForFunction(() => document.querySelector('.editor-status .node-crumb.is-on')?.dataset.tech === 'span');
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
    // Paramètres (D-104) : rubrique ouverte raccordée à la page (ses parties listées) ;
    // informations du site alignées, sans code affiché ; partie suivie au défilement.
    {
      await page.waitForSelector('.settings-sub[data-sub="site-info"] .siteinfo-row');
      const tab = await page.$eval('.settings-nav-group.is-on', (g) => ({
        parts: [...g.querySelectorAll('.settings-nav-part')].filter((x) => x.offsetParent).length,
        bg: getComputedStyle(g).backgroundColor,
        right: Math.round(g.getBoundingClientRect().right),
      }));
      const content = await page.$eval('.settings-content', (c) => ({ bg: getComputedStyle(c).backgroundColor, left: Math.round(c.getBoundingClientRect().left) }));
      const raw = await page.$$eval('.siteinfo-row, .navlist-head', (rs) => rs.some((r) => /on\s*:\s*click|move \||=>/.test(r.textContent)));
      const cols = await page.$$eval('.siteinfo-list .siteinfo-uses', (us) => new Set(us.map((u) => Math.round(u.getBoundingClientRect().right))).size);
      check('paramètres : rubrique raccordée à la page (parties), informations du site alignées, sans code affiché',
        tab.parts === 5 && tab.bg === content.bg && tab.right > content.left && !raw && cols === 1, `${tab.parts} parties, ${cols} colonne(s)`);
      if (shots) await page.screenshot({ path: `${shots}/parametres-general.png` });
      await page.$eval('.settings-content', (c) => c.scrollTo(0, c.scrollHeight));
      await page.waitForFunction(() => document.querySelector('.settings-nav-part[data-part="forms"]')?.classList.contains('is-on'));
      await page.click('.settings-nav-part[data-part="site-info"]');
      await page.waitForFunction(() => {
        const c = document.querySelector('.settings-content').getBoundingClientRect().top;
        const t = document.querySelector('.settings-sub[data-sub="site-info"]').getBoundingClientRect().top;
        return Math.abs(t - c) < 40;
      });
      check('paramètres : partie à l\'écran marquée dans le rail, aller à une partie', true);
      await page.$eval('.settings-content', (c) => c.scrollTo(0, 0));
    }
    // Plan du site (D-103) : pages, collections, menus et liens ; zoom, déplacement, nœud
    // déplacé (gardé dans le site), panneau, destination d'un lien changée, lien ajouté à un
    // menu, page ouverte dans l'éditeur ; tout annulé.
    {
      const docM = await invoke('edit_open');
      const doc0 = JSON.stringify(docM.site);
      await page.click('.view-tab[data-view="view-map"]');
      await page.waitForSelector('.map-node[data-node="page:index"]');
      await page.waitForTimeout(400);
      const keys = await page.$$eval('.map-node', (n) => n.map((x) => x.dataset.node));
      const nEdges = await page.$$eval('.map-edge', (e) => e.length);
      check('plan du site : pages, collections, parties communes et liens',
        keys.includes('page:index') && keys.some((k) => k.startsWith('col:')) && keys.some((k) => k.startsWith('comp:')) && nEdges > 5,
        `${keys.length} nœuds, ${nEdges} traits`);
      if (shots) await page.screenshot({ path: `${shots}/plan-du-site.png` });
      const tr = () => page.$eval('.map-world', (w) => w.style.transform);
      const st = await page.locator('.map-stage').boundingBox();
      const t0 = await tr();
      await page.mouse.move(st.x + st.width / 2, st.y + st.height / 2);
      await page.keyboard.down('Control');
      await page.mouse.wheel(0, -240);
      await page.keyboard.up('Control');
      await page.waitForFunction((t) => document.querySelector('.map-world').style.transform !== t, t0);
      const t1 = await tr();
      // Plan déplacé (bouton du milieu, n'importe où).
      await page.mouse.move(st.x + st.width / 2, st.y + st.height / 2);
      await page.mouse.down({ button: 'middle' });
      await page.mouse.move(st.x + st.width / 2 + 100, st.y + st.height / 2 + 60, { steps: 5 });
      await page.mouse.up({ button: 'middle' });
      const t2 = await tr();
      check('plan du site : zoom (Ctrl+molette), déplacement (glisser)', t0 !== t1 && t1 !== t2, `${t0} → ${t1} → ${t2}`);
      await page.keyboard.press('f');
      // Nœud déplacé : position gardée dans le site.
      const node = page.locator('.map-node[data-node="page:contact"]');
      const nb = await node.boundingBox();
      await page.mouse.move(nb.x + 30, nb.y + 12);
      await page.mouse.down();
      await page.mouse.move(nb.x + 130, nb.y + 92, { steps: 8 });
      await page.mouse.up();
      let saved = null;
      for (let i = 0; i < 40 && !saved; i++) {
        saved = (await invoke('edit_open')).site.library?.map?.['page:contact'] || null;
        if (!saved) await page.waitForTimeout(100);
      }
      await page.waitForSelector('.map-panel .map-panel-title');
      const title = (await page.textContent('.map-panel .map-panel-title')).trim();
      check('plan du site : nœud déplacé (position gardée), sélectionné (panneau)', !!saved && title.length > 0, `${JSON.stringify(saved)} « ${title} »`);
      // Lien écrit dans l'accueil : destination changée par le panneau du trait.
      await page.click('.map-node[data-node="page:index"]', { position: { x: 20, y: 10 } });
      await page.waitForSelector('.map-panel .map-link');
      const row = page.locator('.map-panel .map-link:has(.icon-btn)').first();
      if (await row.count()) {
        const before = JSON.stringify((await invoke('edit_open')).site.pages);
        await row.locator('.map-link-main').click();
        await page.waitForSelector('.map-panel select');
        const target = await page.$eval('.map-panel select', (sel) => [...sel.options].map((o) => o.value).find((v) => v === 'page:mentions-legales') || [...sel.options].map((o) => o.value).filter(Boolean).pop());
        await page.selectOption('.map-panel select', target);
        let changed = false;
        for (let i = 0; i < 40 && !changed; i++) {
          changed = JSON.stringify((await invoke('edit_open')).site.pages) !== before;
          if (!changed) await page.waitForTimeout(100);
        }
        check('plan du site : destination d\'un lien changée (panneau du trait)', changed, target);
      } else {
        check('plan du site : lien écrit dans l\'accueil trouvé', false);
      }
      // Lien ajouté : point d'une page tiré sur une autre (lien écrit à la fin de la page).
      await page.locator('.map-stage').focus();
      await page.keyboard.press('Escape');
      await page.keyboard.press('f');
      await page.waitForTimeout(300);
      const port = page.locator('.map-node[data-node="page:contact"] .map-port');
      const pb = await port.boundingBox();
      const tb = await page.locator('.map-node[data-node="page:agence"]').boundingBox();
      const hitAt = await page.evaluate(([x, y]) => String(document.elementFromPoint(x, y)?.className || ''), [pb.x + pb.width / 2, pb.y + pb.height / 2]);
      await page.mouse.move(pb.x + pb.width / 2, pb.y + pb.height / 2);
      await page.mouse.down();
      await page.mouse.move(tb.x + tb.width / 2, tb.y + tb.height / 2, { steps: 10 });
      await page.mouse.up();
      let added = false;
      for (let i = 0; i < 40 && !added; i++) {
        const body = (await invoke('edit_open')).site.pages.find((p) => p.id === 'contact')?.body || [];
        added = JSON.stringify(body[body.length - 1] || {}).includes('"/agence/"');
        if (!added) await page.waitForTimeout(100);
      }
      const notice = added ? (await page.waitForSelector('.map-notice').then((n) => n.textContent()).catch(() => '')) : `point : ${hitAt}`;
      check('plan du site : lien ajouté (point d\'une page tiré sur une autre)', added, notice.trim());
      if (shots) await page.screenshot({ path: `${shots}/plan-du-site-lien.png` });
      // Tout annulé (Ctrl+Z dans le plan).
      await page.locator('.map-stage').focus();
      for (let i = 0; i < 8; i++) {
        if (JSON.stringify((await invoke('edit_open')).site) === doc0) break;
        await page.keyboard.press('Control+z');
        await page.waitForTimeout(400);
      }
      check('plan du site : Ctrl+Z, document identique', JSON.stringify((await invoke('edit_open')).site) === doc0);
      // Double-clic : page ouverte dans l'éditeur.
      await page.dblclick('.map-node[data-node="page:contact"]', { position: { x: 20, y: 10 } });
      await page.waitForSelector('.view-pane--editor:not(.is-hidden)');
      f = await waitFrame(() => location.pathname === '/contact/', 'page ouverte depuis le plan');
      check('plan du site : double-clic, page ouverte dans l\'éditeur', true);
      // Retour à l'accueil (canevas marqué : la suite vérifie qu'il n'est pas rechargé).
      await page.click('.pages .page-row[data-url="/"]');
      f = await waitFrame(() => location.pathname === '/', 'accueil après le plan');
      await f.evaluate(() => { window.__keep = 1; });
      await page.click('.view-tab[data-view="view-settings"]');
      await page.waitForSelector('.settings-nav');
    }
    // Transition entre les pages (6.4) : Paramètres › Général, choisie puis retirée.
    const transition = async (v) => {
      for (let i = 0; i < 40; i++) {
        if (((await invoke('edit_open')).site.settings.page_transition ?? '') === v) return true;
        await page.waitForTimeout(100);
      }
      return false;
    };
    await page.selectOption('#site-transition', 'slide');
    const slide = await transition('slide');
    await page.selectOption('#site-transition', '');
    check('paramètres : transition entre les pages choisie puis retirée', slide && await transition(''));
    await page.click('.view-tab[data-view="view-editor"]');
    await page.waitForSelector('.view-pane--editor:not(.is-hidden)');
    check('retour à l\'éditeur : canevas gardé (pas rechargé)', await frameOf().evaluate(() => window.__keep === 1));
    // Guide intégré (6.8) : menu « ? », rubriques, fermé par Échap.
    await page.click('.shortcuts > summary');
    await page.click('.help-open');
    await page.waitForSelector('.help-dialog');
    const helpN = await page.locator('.help-nav-item').count();
    await page.locator('.help-nav-item', { hasText: 'Contenus (CMS)' }).click();
    const helpH = await page.locator('.help-body h2').first().textContent();
    if (shots) await page.screenshot({ path: `${shots}/aide.png` });
    await page.keyboard.press('Escape');
    await page.waitForSelector('.help-dialog', { state: 'detached' });
    check('aide : guide intégré (rubriques, rubrique affichée, Échap)', helpN >= 10 && helpH === 'Contenus (CMS)', `${helpN} rubriques`);
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
  await page.click('.layers .layer[data-tech*="h1.hero__title"]');
  await page.click('.dock-tab[data-panel="styles"]');
  await page.waitForFunction(() => document.querySelector('.styles .sel-chip.is-on')?.textContent.trim() === 'hero__title');
  check('styles : sélecteur de la classe de l\'élément', true);
  const styleInput = (prop) => page.locator(`.style-row[data-prop="${prop}"]`).locator('input, select').first();
  f = frameOf();
  await f.evaluate(() => { window.__mark2 = 1; });
  // Saisie refaite si le panneau a été redessiné pendant la frappe (éditeur relu après la
  // lecture du document par l'essai juste avant).
  for (let attempt = 0; ; attempt++) {
    await styleInput('color').fill('rgb(10, 20, 30)');
    await styleInput('color').press('Enter');
    try {
      f = await waitFrame(() => getComputedStyle(document.querySelector('h1.hero__title')).color === 'rgb(10, 20, 30)', 'couleur du titre', undefined, 50);
      break;
    } catch (e) {
      if (attempt === 2) throw e;
    }
  }
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
  // Disposition comme Webflow : Bloc, Flex, Grille, Masqué et la flèche des autres affichages ;
  // flex (direction, matrice X / Y, écarts) ; grille (colonnes, lignes) ; tout annulé.
  {
    const layDecls = async () => JSON.stringify((await titleRule()).decls);
    const lay0 = await layDecls();
    const untilD = async (prop, v) => {
      for (let i = 0; i < 60; i++) {
        if (decl(await titleRule(), prop) === v) return true;
        await page.waitForTimeout(100);
      }
      return false;
    };
    const labels = await page.$$eval('.display-ctl .seg button', (l) => l.map((b) => b.textContent.trim()));
    await page.click('.display-ctl .seg button:has-text("Flex")');
    const flex = await untilD('display', 'flex');
    await page.click('.seg-more');
    await page.click('.display-more .menu-item:has-text("Bloc en ligne")');
    const inline = await untilD('display', 'inline-block');
    await page.click('.display-ctl .seg button:has-text("Flex")');
    await untilD('display', 'flex');
    await page.click('.align-box:not(.align-box--small) .align-cell >> nth=4');
    const centre = await untilD('justify-content', 'center') && await untilD('align-items', 'center');
    check('disposition : Bloc, Flex, Grille, Masqué ; autres affichages par la flèche ; matrice d\'alignement',
      labels.join('|') === 'Bloc|Flex|Grille|Masqué' && flex && inline && centre, labels.join('|'));
    await page.click('.display-ctl .seg button:has-text("Grille")');
    await untilD('display', 'grid');
    await page.locator('.stepper').nth(0).locator('.stepper-arrows button').nth(0).click();
    const cols = await untilD('grid-template-columns', 'repeat(2, minmax(0, 1fr))');
    // Lignes non fixées : nombre de lignes affichées dans la page (implicites comprises), plus une.
    await page.waitForTimeout(800);
    const rowsInput = page.locator('.stepper').nth(1).locator('input');
    const shown = parseInt((await rowsInput.inputValue()) || (await rowsInput.getAttribute('placeholder')) || '1', 10) || 1;
    await page.locator('.stepper').nth(1).locator('.stepper-arrows button').nth(0).click();
    const rows = await untilD('grid-template-rows', `repeat(${shown + 1}, auto)`);
    check('disposition : grille, colonnes et lignes par les flèches', cols && rows,
      `${decl(await titleRule(), 'grid-template-columns')} / ${decl(await titleRule(), 'grid-template-rows')}`);
    if (shots) await page.locator('.dock-col--right').screenshot({ path: `${shots}/styles-disposition.png` });
    for (let i = 0; i < 12 && (await layDecls()) !== lay0; i++) {
      await page.click('.editor-toolbar .btn-undo');
      await page.waitForTimeout(250);
    }
    check('disposition : tout annulé', (await layDecls()) === lay0);
  }
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
  const truncated = () => page.$$eval('.dock-panel[data-panel="styles"] :is(.style-label, .style-label-text, .label, .seg button span, .mini-label, .pair-label), .dock-tab-label', (els) =>
    els.filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => e.textContent.trim()));
  const cut = await truncated();
  check('styles : aucun libellé tronqué (français)', cut.length === 0, cut.join(', '));
  if (shots) await page.locator('.dock-col--right').screenshot({ path: `${shots}/editeur-styles-panneau-fr.png` });
  await page.evaluate(() => window.__revelTest.lang('en'));
  await page.waitForFunction(() => document.querySelector('.dock-tab[data-panel="layers"]')?.textContent.includes('Layers'));
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
  // Mise en forme dans le texte (barre au-dessus de la sélection, comme Webflow) :
  // gras par la barre, italique au clavier, une seule annulation pour le tout.
  const t4 = await slotPoint("s.slot === 'text' && s.parent.owner.kind === 'page'");
  if (t4) {
    const count = (site, tag) => JSON.stringify(site).split(`"tag":"${tag}"`).length - 1;
    const site0 = (await invoke('edit_open')).site;
    await page.mouse.dblclick(t4.px, t4.py);
    f = await waitFrame(() => !!document.querySelector('[data-rv-edit]'), 'mode édition (mise en forme)');
    const word = await f.evaluate(() => {
      const span = document.querySelector('[data-rv-edit]');
      const w = document.createTreeWalker(span, NodeFilter.SHOW_TEXT);
      for (let t; (t = w.nextNode());) {
        const m = /\p{L}{3,}/u.exec(t.data);
        if (!m) continue;
        const r = document.createRange();
        r.setStart(t, m.index);
        r.setEnd(t, m.index + m[0].length);
        const s = getSelection();
        s.removeAllRanges();
        s.addRange(r);
        const b = r.getBoundingClientRect();
        return { text: m[0], x: b.left + b.width / 2, y: b.top };
      }
      return null;
    });
    check('mise en forme : mot sélectionné dans le texte', !!word, word && word.text);
    if (word) {
      await page.waitForTimeout(200);
      const box = await page.locator('iframe.canvas-frame').boundingBox();
      const k = box.width / parseFloat(await page.$eval('iframe.canvas-frame', (e) => e.style.width));
      if (shots) await page.screenshot({ path: `${shots}/mise-en-forme.png`, clip: { x: Math.max(0, box.x + word.x * k - 200), y: Math.max(0, box.y + word.y * k - 80), width: 400, height: 140 } });
      // Premier bouton (gras) : 70 px à gauche du milieu de la sélection, 34 px au-dessus.
      await page.mouse.click(box.x + word.x * k - 70 + 15, Math.max(box.y + 4 + 15, box.y + word.y * k - 34 + 15));
      await page.keyboard.press('Control+i');
      const html = await f.evaluate(() => document.querySelector('[data-rv-edit]').innerHTML);
      check('mise en forme : gras (barre) et italique (clavier) dans le texte', /<(b|strong)\b/.test(html) && /<(i|em)\b/.test(html), html.slice(0, 160));
      await page.keyboard.press('Enter');
      let site1 = site0;
      for (let i = 0; i < 50 && count(site1, 'strong') === count(site0, 'strong'); i++) {
        await page.waitForTimeout(100);
        site1 = (await invoke('edit_open')).site;
      }
      check('mise en forme : strong et em enregistrés dans le document',
        count(site1, 'strong') === count(site0, 'strong') + 1 && count(site1, 'em') === count(site0, 'em') + 1 && JSON.stringify(site1).includes(word.text));
      f = await waitFrame(() => !document.querySelector('[data-rv-edit]') && !!document.querySelector('strong em, em strong'), 'mise en forme affichée');
      await page.click('.editor-toolbar .btn-undo');
      let back = false;
      for (let i = 0; i < 50 && !back; i++) {
        back = JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(site0);
        if (!back) await page.waitForTimeout(100);
      }
      check('mise en forme : une seule annulation rend le texte d\'origine', back);
      f = await waitFrame(() => !document.querySelector('strong em, em strong'), 'annulation de la mise en forme');
    }
  }
  // Texte et éléments de mise en forme modifiés d'un bloc (D-72) : le titre entier, ses spans
  // gardés (mêmes identifiants), une seule annulation.
  {
    const site0 = (await invoke('edit_open')).site;
    f = await waitFrame(() => document.readyState === 'complete', 'canevas prêt (bloc)');
    const pt = await f.evaluate(() => {
      // Textes de la page (hors composants) : élément qui les contient.
      const hosts = new Set();
      for (let n = 0; n < 5000; n++) {
        const x = window.__revelSlot(n);
        if (!x.slot) break;
        if (x.slot.slot !== 'text' || x.slot.parent.owner.kind === 'component' || !x.range) continue;
        let c = x.range.commonAncestorContainer;
        if (c.nodeType !== 1) c = c.parentElement;
        for (let e = c && c.closest('[data-rv]'); e; e = e.parentElement && e.parentElement.closest('[data-rv]')) hosts.add(e);
      }
      for (const el of hosts) {
        const kids = [...el.childNodes];
        if (!kids.some((k) => k.nodeType === 1 && k.hasAttribute('data-rv'))) continue;
        if (!kids.every((k) => k.nodeType === 3 || (k.nodeType === 1 && /^(SPAN|STRONG|EM|BR|A)$/.test(k.tagName)))) continue;
        const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        for (let t; (t = w.nextNode());) {
          const m = /\p{L}{3,}/u.exec(t.data);
          if (!m) continue;
          const r = document.createRange();
          r.setStart(t, m.index);
          r.setEnd(t, m.index + m[0].length);
          const b = r.getBoundingClientRect();
          if (b.width > 2 && b.top > 0 && b.bottom < innerHeight) return { x: b.left + b.width / 2, y: b.top + b.height / 2, word: m[0] };
        }
      }
      return null;
    });
    check('bloc : texte avec des éléments de mise en forme repéré', !!pt);
    if (pt) {
      const box = await page.locator('iframe.canvas-frame').boundingBox();
      const k = box.width / parseFloat(await page.$eval('iframe.canvas-frame', (e) => e.style.width));
      await page.mouse.dblclick(box.x + pt.x * k, box.y + pt.y * k);
      f = await waitFrame(() => !!document.querySelector('[data-rv-edit][data-rv]'), 'modification d\'un bloc');
      const st = await f.evaluate(() => {
        const e = document.querySelector('[data-rv-edit]');
        return { id: e.getAttribute('data-rv'), kids: [...e.querySelectorAll('[data-rv]')].map((k) => k.getAttribute('data-rv')), sel: getSelection().toString() };
      });
      check('bloc : élément entier modifiable, mot pointé sélectionné', st.kids.length > 0 && st.sel === pt.word, `${st.sel} / ${pt.word}`);
      await page.keyboard.press('Control+End');
      await page.keyboard.type(' bloc');
      await page.keyboard.press('Enter');
      let site1 = site0;
      for (let i = 0; i < 50 && JSON.stringify(site1) === JSON.stringify(site0); i++) {
        await page.waitForTimeout(100);
        site1 = (await invoke('edit_open')).site;
      }
      const s1 = JSON.stringify(site1);
      check('bloc : texte enregistré, éléments de mise en forme gardés', s1.includes(' bloc') && st.kids.every((id) => s1.includes(`"id":"${id}"`)));
      f = await waitFrame(() => !document.querySelector('[data-rv-edit]') && document.body.textContent.includes(' bloc'), 'bloc enregistré');
      await page.click('.editor-toolbar .btn-undo');
      let back = false;
      for (let i = 0; i < 50 && !back; i++) {
        back = JSON.stringify((await invoke('edit_open')).site) === JSON.stringify(site0);
        if (!back) await page.waitForTimeout(100);
      }
      check('bloc : une seule annulation', back);
      f = await waitFrame(() => !document.body.textContent.includes(' bloc'), 'annulation du bloc');
    }
  }
  // Texte lié à l'en-tête d'un contenu (fichier Markdown).
  const t2 =await slotPoint("s.slot === 'expr' && s.binding && s.binding.source.kind === 'entry' && !s.transformed && !s.html");
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
    // Collection relue (en-tête redessiné) avant d'ouvrir le menu « Nouveau champ ».
    await page.waitForFunction(() => !document.querySelector('.schema-row[data-field="couleur_dominante"]'));
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

    // ---------------------------------------------------------------- Réglages du site (3.9)
    {
      const docS = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
      const doc9 = docS();
      await page.click('.view-tab[data-view="view-settings"]');
      await page.click('.settings-nav-item[data-section="settings-general"]');
      await page.waitForSelector('#site-name');
      await page.waitForFunction(() => document.querySelector('#site-name').value !== '');
      const oldName = await page.inputValue('#site-name');
      await page.fill('#site-name', oldName + ' essai');
      await page.press('#site-name', 'Tab');
      await until(() => JSON.parse(docS()).name === oldName + ' essai', 'nom du site');
      await page.waitForFunction((n) => document.querySelector('.site-menu-name')?.textContent === n, oldName + ' essai');
      await page.fill('#site-url', 'pas une adresse');
      await page.press('#site-url', 'Tab');
      await page.waitForSelector('.notice--error');
      const refused = (await page.textContent('.notice--error')).trim();
      check('réglages du site : nom modifié (barre supérieure), adresse invalide refusée', JSON.parse(docS()).base_url !== 'pas une adresse', refused);
      await page.click('.notice--error .icon-btn');
      // Code personnalisé du site.
      await page.click('.settings-nav-item[data-section="settings-code"]');
      await page.waitForSelector('.code-input');
      await page.fill('.code-field >> nth=0 >> textarea', '<meta name="essai-e2e" content="1">');
      await page.locator('.code-field >> nth=0 >> textarea').blur();
      await until(() => JSON.parse(docS()).settings?.code?.head === '<meta name="essai-e2e" content="1">', 'code ajouté');
      check('réglages du site : code personnalisé (en-tête) enregistré', true);
      if (shots) await page.screenshot({ path: `${shots}/code-personnalise.png` });
      // Polices : fichier importé (graisse et style lus dans le nom), puis retiré.
      // Polices : partie de Général.
      await page.click('.settings-nav-item[data-section="settings-general"]');
      await page.click('.settings-nav-part[data-part="fonts"]');
      await page.waitForSelector('.settings-sub[data-sub="fonts"] .font-list');
      const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.click('.fonts-page .page-head .btn:not(.btn--primary)')]);
      await chooser.setFiles({ name: 'EssaiSans-BoldItalic.woff2', mimeType: 'font/woff2', buffer: Buffer.from('wOF2essai-e2e') });
      await page.waitForSelector('.fonts-dialog .fi-row');
      const fam = await page.inputValue('.fonts-dialog .field input');
      const w = await page.$eval('.fonts-dialog .fi-row select', (s) => s.value);
      const it = await page.$eval('.fonts-dialog .fi-row input[type="checkbox"]', (c) => c.checked);
      check('polices : famille, graisse et italique lus dans le nom du fichier', fam === 'Essai Sans' && w === '700' && it, `${fam} ${w} ${it}`);
      if (shots) await page.screenshot({ path: `${shots}/polices-import.png` });
      await page.click('.fonts-dialog button[type="submit"]');
      await page.waitForSelector('.font-row[data-family="Essai Sans"]');
      const fontDir = path.join(siteDir, 'src/media/fonts/essai-sans');
      check('polices : fichier copié dans le site, règle @font-face ajoutée', fs.existsSync(path.join(fontDir, 'essaisans-bolditalic.woff2'))
        && docS().includes('font-family: \\"Essai Sans\\"'), (await page.textContent('.font-row[data-family="Essai Sans"]')).trim());
      if (shots) await page.screenshot({ path: `${shots}/polices.png` });
      await page.click('.font-row[data-family="Essai Sans"] .icon-btn');
      await page.click('.dialog .btn--danger');
      await page.waitForSelector('.font-row[data-family="Essai Sans"]', { state: 'detached' });
      check('polices : retirée (fichier rangé dans .revel)', !fs.existsSync(fontDir));
      // Tout annuler : document et fichiers identiques.
      await page.click('.view-tab[data-view="view-editor"]');
      for (let i = 0; i < 6 && (docS() !== doc9 || fs.existsSync(fontDir)); i++) {
        await page.click('.editor-toolbar .btn-undo');
        await page.waitForTimeout(400);
      }
      await until(() => docS() === doc9 && !fs.existsSync(fontDir), 'réglages annulés');
      check('réglages du site : tout annulé (document, polices)', true);
    }

    // ---------------------------------------------------------------- Versions (3.10)
    {
      const docV = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
      const docv0 = docV();
      await page.click('.view-tab[data-view="view-settings"]');
      await page.click('.settings-nav-item[data-section="settings-versions"]');
      await page.waitForSelector('.versions-empty .btn, .ver-item');
      if (await page.$('.versions-empty .btn')) {
        await page.click('.versions-empty .btn');
        await page.waitForSelector('.ver-item', { timeout: 120000 });
      }
      check('versions : activées (dépôt du site, première version)', fs.existsSync(path.join(siteDir, '.git'))
        && fs.readFileSync(path.join(siteDir, '.gitignore'), 'utf8').includes('.revel/'));
      // Changement : code personnalisé ajouté, comparé à la dernière version.
      await page.click('.settings-nav-item[data-section="settings-code"]');
      await page.waitForSelector('.code-input');
      await page.fill('.code-field >> nth=0 >> textarea', '<meta name="version-e2e">');
      await page.locator('.code-field >> nth=0 >> textarea').blur();
      await until(() => docV().includes('version-e2e'), 'changement enregistré');
      await page.click('.settings-nav-item[data-section="settings-versions"]');
      await page.waitForSelector('.ver-item--current');
      await page.click('.ver-item--current');
      await page.waitForSelector('.diff tr.is-add');
      const added = await page.$$eval('.diff tr.is-add .diff-text', (l) => l.map((x) => x.textContent).join(' '));
      check('versions : changements en cours comparés ligne à ligne', added.includes('version-e2e'), added.slice(0, 80));
      if (shots) await page.screenshot({ path: `${shots}/versions.png` });
      await page.click('.versions-save');
      await page.fill('.dialog input', 'Essai e2e');
      await page.click('.dialog button[type="submit"]');
      await page.waitForFunction(() => document.querySelector('.ver-item:not(.ver-item--current) .ver-msg')?.textContent === 'Essai e2e', null, { timeout: 60000 });
      check('versions : version enregistrée (plus de changement en cours)', !(await page.$('.ver-item--current')));
      // Restauration de la version précédente : document revenu à l'identique.
      await page.click('.ver-item:not(.ver-item--current) >> nth=1');
      await page.waitForSelector('.ver-detail-head .btn--danger-ghost');
      await page.click('.ver-detail-head .btn--danger-ghost');
      await page.click('.dialog .btn--danger');
      await until(() => docV() === docv0, 'version restaurée');
      await page.waitForFunction(() => /Restauration/.test(document.querySelector('.ver-item:not(.ver-item--current) .ver-msg')?.textContent || ''), null, { timeout: 60000 });
      check('versions : version précédente restaurée (document identique, nouvelle version)', true,
        (await page.textContent('.ver-item:not(.ver-item--current) .ver-msg')).trim());

      // Travail à plusieurs (D-89) : dossier partagé (dépôt central créé, premier envoi), équipier
      // qui modifie d'autres endroits (fusion sans rien demander) et un même endroit (choisi).
      {
        const shared = fs.mkdtempSync(path.join(os.tmpdir(), 'revel-partage-'));
        const mate = fs.mkdtempSync(path.join(os.tmpdir(), 'revel-equipier-'));
        const central = path.join(shared, path.basename(siteDir) + '.git');
        const git = (args, cwd) => execFileSync('git', ['-c', 'user.name=Bruno', '-c', 'user.email=bruno@exemple.invalid', ...args], { cwd, encoding: 'utf8' }).trim();
        const head = (dir) => { try { return git(['--git-dir', dir, 'rev-parse', 'HEAD']); } catch (_) { return ''; } };
        const edit = (dir, f) => {
          const p = path.join(dir, 'site.revel.json');
          const d = JSON.parse(fs.readFileSync(p, 'utf8'));
          f(Object.fromEntries(d.pages.map((x) => [x.id, x.props])));
          fs.writeFileSync(p, JSON.stringify(d, null, 2) + '\n');
        };
        await page.click('.versions-mode button[data-mode="shared"]');
        await page.waitForSelector('.team-dialog .team-compare');
        const rows = await page.$$eval('.team-compare tbody tr', (l) => l.length);
        const chosen = (await page.textContent('.team-col.is-on')).trim();
        const disabled = await page.$eval('.team-on', (b) => b.disabled);
        await page.fill('.team-folder', shared);
        if (shots) await page.locator('.team-dialog').screenshot({ path: `${shots}/versions-plusieurs.png` });
        await page.click('.team-on');
        await page.waitForSelector('.versions-mode button[data-mode="shared"].is-on', { timeout: 60000 });
        await until(() => head(central) !== '' && head(central) === head(path.join(siteDir, '.git')), 'premier envoi au dépôt central');
        const shown = (await page.textContent('.versions-remote-value')).replace(/\u200e/g, '').trim();
        check('versions : travail à plusieurs, dossier partagé (comparaison, dépôt central créé, premier envoi)',
          rows === 6 && chosen === 'Dossier partagé' && disabled && shown === central, `${rows} lignes · ${chosen} · ${shown}`);
        // Équipier : copie du dépôt central, autre page et même endroit modifiés, envoi.
        git(['clone', '-q', central, mate]);
        edit(mate, (p) => { p.agence.e2eEquipe = 'Bruno'; p.index.bodyClass = 'page-home bruno'; });
        git(['commit', '-qam', 'Bruno : agence et accueil'], mate);
        git(['push', '-q', 'origin', 'HEAD'], mate);
        // Ici : contact et le même endroit de l'accueil.
        edit(siteDir, (p) => { p.contact.e2eEquipe = 'Anne'; p.index.bodyClass = 'page-home anne'; });
        await page.waitForTimeout(400);
        await page.click('.versions-sync');
        await page.waitForSelector('.versions-item', { timeout: 60000 });
        const place = (await page.textContent('.versions-item .versions-item-area')).trim();
        const ours = (await page.textContent('.versions-item .versions-pick--ours .versions-pick-value')).trim();
        const theirs = (await page.textContent('.versions-item .versions-pick--theirs .versions-pick-value')).trim();
        const waiting = await page.$eval('.versions-apply', (b) => b.disabled);
        if (shots) await page.screenshot({ path: `${shots}/versions-endroits.png` });
        await page.click('.versions-item .versions-pick--theirs');
        await page.click('.versions-apply');
        await page.waitForSelector('.versions-item', { state: 'detached', timeout: 60000 });
        await until(() => head(central) === head(path.join(siteDir, '.git')), 'choix envoyé');
        const props = Object.fromEntries(JSON.parse(docV()).pages.map((x) => [x.id, x.props]));
        check('versions : endroits différents combinés, même endroit choisi un par un puis envoyé',
          place === 'Page /' && /page-home anne/.test(ours) && /page-home bruno/.test(theirs) && waiting
            && props.agence.e2eEquipe === 'Bruno' && props.contact.e2eEquipe === 'Anne' && props.index.bodyClass === 'page-home bruno',
          `${place} · ${ours} / ${theirs} · ${JSON.stringify([props.agence.e2eEquipe, props.contact.e2eEquipe, props.index.bodyClass])}`);
        // Retour au travail seul : dépôt commun retiré, versions gardées.
        await page.click('.versions-mode button[data-mode="solo"]');
        await page.click('.dialog .btn--danger');
        await page.waitForSelector('.versions-mode button[data-mode="solo"].is-on');
        check('versions : retour au travail seul (dépôt commun retiré)', !(await page.$('.versions-remote-value')));
        fs.writeFileSync(path.join(siteDir, 'site.revel.json'), docv0);
        await until(() => docV() === docv0, 'document d\'origine');
        fs.rmSync(shared, { recursive: true, force: true });
        fs.rmSync(mate, { recursive: true, force: true });
      }
    }

    // ---------------------------------------------------------------- Canevas, suite (3.11)
    {
      const docC = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
      const doc11 = docC();
      await page.click('.view-tab[data-view="view-editor"]');
      await page.waitForSelector('iframe.canvas-frame.is-active');
      // Zoom ajusté : toute la largeur de la page visible.
      await page.selectOption('.editor-zoom select', '');
      await page.waitForTimeout(400);
      const scaleOf = () => page.evaluate(() => {
        const t = getComputedStyle(document.querySelector('iframe.canvas-frame.is-active')).transform;
        const m = /matrix\(([^,]+)/.exec(t);
        return m ? Number(m[1]) : 1;
      });
      const toPage = async (x, y) => {
        const box = await page.locator('iframe.canvas-frame.is-active').boundingBox();
        const s = await scaleOf();
        return [box.x + x * s, box.y + y * s];
      };
      const layer = (tech) => page.locator(`.layers .layer[data-tech*="${tech}"]`).first();
      const undoAll = async (target, what) => {
        for (let i = 0; i < 6 && docC() !== target; i++) {
          await page.click('.editor-toolbar .btn-undo');
          await page.waitForTimeout(400);
        }
        await until(() => docC() === target, what);
      };
      // Poignées de taille : largeur écrite dans la classe, au point de rupture affiché.
      await layer('div.intro-agency__inner').click();
      await page.waitForTimeout(600);
      // Élément montré (le canevas a pu être rechargé après la restauration des versions).
      const r = await waitFrame(() => {
        const e = document.querySelector('.intro-agency__inner');
        if (!e) return false;
        const b = e.getBoundingClientRect();
        if (b.top < 0 || b.bottom > innerHeight) e.scrollIntoView({ block: 'center' });
        return true;
      }, 'élément visible');
      await page.waitForTimeout(400);
      // Poignée en place (calque du canevas sous le point) avant d'appuyer.
      const rf = await waitFrame(() => {
        const b = document.querySelector('.intro-agency__inner')?.getBoundingClientRect();
        const el = b && document.elementFromPoint(b.right, b.top + b.height / 2);
        return !!el && el.tagName === 'REVEL-OVERLAY';
      }, 'poignée de taille affichée');
      const rect = await rf.evaluate(() => {
        const b = document.querySelector('.intro-agency__inner').getBoundingClientRect();
        return { x: b.right, y: b.top + b.height / 2, w: b.width };
      });
      const [hx, hy] = await toPage(rect.x, rect.y);
      await page.mouse.move(hx, hy);
      await page.mouse.down();
      await page.mouse.move(hx - 30, hy, { steps: 4 });
      await page.mouse.move(hx - 60, hy, { steps: 4 });
      await page.mouse.up();
      await until(() => {
        const d = JSON.parse(docC());
        return (d.stylesheet.items || []).some((i) => i.selector === '.intro-agency__inner' && !i.media
          && (i.decls || []).some((x) => x.prop === 'width' && /^\d+px$/.test(x.value)));
      }, 'largeur écrite');
      const wNow = await (await waitFrame(() => !document.querySelector('.intro-agency__inner')?.getAttribute('style'), 'style en ligne retiré'))
        .evaluate(() => document.querySelector('.intro-agency__inner').getBoundingClientRect().width);
      check('canevas : poignée de taille (largeur écrite dans la classe, style en ligne retiré)', wNow < rect.w - 20, `${Math.round(rect.w)} → ${Math.round(wNow)} px`);
      if (shots) await page.screenshot({ path: `${shots}/poignees.png` });
      await undoAll(doc11, 'taille annulée');
      check('canevas : taille annulée (document identique)', true);
      // Sélection multiple (Maj+clic dans les calques) : dupliquée en une étape, annulée.
      await layer('section.hero').click();
      await layer('section.intro-agency').click({ modifiers: ['Shift'] });
      await page.waitForSelector('.chip--multi');
      const chip = (await page.textContent('.chip--multi')).trim();
      check('canevas : sélection multiple (Maj+clic), compte affiché', /^2\b/.test(chip), chip);
      const n0 = (docC().match(/"hero"/g) || []).length;
      await page.keyboard.press('Control+d');
      await until(() => docC() !== doc11, 'sélection dupliquée');
      const d1 = JSON.parse(docC());
      const countClass = (nodes, c) => nodes.reduce((n, x) => n + ((x.attrs || []).some((a) => a.name === 'class' && (a.value || '').split(/\s+/).includes(c)) ? 1 : 0)
        + countClass(x.children || [], c) + countClass(x.then || [], c) + countClass(x.otherwise || [], c), 0);
      const pagesBody = (d) => d.pages.flatMap((p) => p.body || []);
      const dupHero = countClass(pagesBody(d1), 'hero') - countClass(pagesBody(JSON.parse(doc11)), 'hero');
      const dupIntro = countClass(pagesBody(d1), 'intro-agency') - countClass(pagesBody(JSON.parse(doc11)), 'intro-agency');
      check('canevas : sélection multiple dupliquée (les deux éléments)', dupHero === 1 && dupIntro === 1, `${dupHero} + ${dupIntro}`);
      await page.click('.editor-toolbar .btn-undo');
      await until(() => docC() === doc11, 'duplication annulée');
      check('canevas : duplication multiple annulée en une étape', true, String(n0));
      await page.keyboard.press('Escape');
      // Palette de commandes (Ctrl+K) : page ouverte par son nom, puis l'accueil.
      await page.keyboard.press('Control+k');
      await page.waitForSelector('.palette-input');
      await page.keyboard.type('agence');
      const first = (await page.textContent('.palette-item.is-cur .palette-label')).trim();
      if (shots) await page.screenshot({ path: `${shots}/palette.png` });
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => /agence/.test(document.querySelector('iframe.canvas-frame.is-active')?.src || ''));
      check('palette : recherche, page ouverte avec Entrée', !(await page.$('.palette')), first);
      await page.keyboard.press('Control+k');
      await page.waitForSelector('.palette-input');
      await page.keyboard.press('Escape');
      await page.waitForSelector('.palette', { state: 'detached' });
      await page.keyboard.press('Control+k');
      await page.waitForSelector('.palette-input');
      await page.keyboard.type('/');
      await page.locator('.palette-item', { has: page.locator('.palette-hint', { hasText: /^\/$/ }) }).first().click();
      await page.waitForFunction(() => !/agence/.test(document.querySelector('iframe.canvas-frame.is-active')?.src || ''));
      check('palette : Échap ferme, retour à l\'accueil', true);
      // Tous les écrans côte à côte : un cadre par écran, clic dans un écran = écran choisi,
      // bouton d'un écran = affiché ou masqué.
      const nbp = JSON.parse(docC()).breakpoints.length;
      await page.click('.side-toggle');
      await page.waitForFunction((n) => document.querySelectorAll('iframe.canvas-frame').length === n, nbp);
      await page.waitForTimeout(2500);
      const second = page.locator('iframe.canvas-frame').nth(1);
      const sb = await second.boundingBox();
      const bp2 = await second.getAttribute('data-bp');
      await page.mouse.click(sb.x + sb.width / 2, sb.y + sb.height / 2);
      await page.waitForFunction((b) => document.querySelector('iframe.canvas-frame.is-active')?.dataset.bp === b, bp2);
      if (shots) await page.screenshot({ path: `${shots}/ecrans-cote-a-cote.png` });
      const cur = await page.$eval('.editor-toolbar .segmented button.is-current', (b) => b.textContent.trim());
      check('écrans côte à côte : un cadre par écran, clic dans un écran = écran choisi', true, `${nbp} cadres, choisi : ${cur}`);
      await page.click('.editor-toolbar .segmented button >> nth=-1');
      await page.waitForFunction((n) => document.querySelectorAll('iframe.canvas-frame').length === n - 1, nbp);
      await page.click('.editor-toolbar .segmented button >> nth=-1');
      await page.waitForFunction((n) => document.querySelectorAll('iframe.canvas-frame').length === n, nbp);
      check('écrans côte à côte : écran masqué puis affiché (bouton de l\'écran)', true);
      await page.click('.side-toggle');
      await page.waitForFunction(() => document.querySelectorAll('iframe.canvas-frame').length === 1);
      await page.click('.editor-toolbar .segmented button >> nth=0');
      // Cadre allongé : plus de page visible, mise en page inchangée (unités vh).
      f = await waitFrame(() => document.readyState === 'complete' && !!document.querySelector('.hero'), 'cadre unique');
      const hero0 = await f.evaluate(() => document.querySelector('.hero').getBoundingClientRect().height);
      const ih0 = await f.evaluate(() => innerHeight);
      const g = await page.locator('.frame-grow').boundingBox();
      await page.mouse.move(g.x + g.width / 2, g.y + 3);
      await page.mouse.down();
      await page.mouse.move(g.x + g.width / 2, g.y + 250, { steps: 5 });
      await page.mouse.up();
      f = await waitFrame((h) => innerHeight > h + 100, 'cadre allongé', ih0);
      await page.waitForTimeout(300);
      const [hero1, ih1] = await f.evaluate(() => [document.querySelector('.hero').getBoundingClientRect().height, innerHeight]);
      check('cadre allongé : plus de page affichée, mise en page inchangée', Math.abs(hero1 - hero0) < 1 && ih1 > ih0 + 100, `${ih0} → ${ih1} px, héro ${Math.round(hero0)} → ${Math.round(hero1)} px`);
      // Poignée sous la zone visible (zone du canevas jamais défilée) : double-clic envoyé.
      await page.locator('.frame-grow').dispatchEvent('dblclick');
      await waitFrame((h) => Math.abs(innerHeight - h) < 2, 'hauteur rétablie', ih0);
      check('cadre allongé : double-clic, hauteur par défaut', docC() === doc11);
    }

    // ---------------------------------------------------------------- Réglages lisibles (3.12)
    {
      const docR = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
      const home = path.join(siteDir, 'src/_data/pages/accueil.yml');
      const siteYml = path.join(siteDir, 'src/_data/site.yml');
      const doc12 = docR();
      const home0 = fs.readFileSync(home, 'utf8');
      const site0 = fs.readFileSync(siteYml, 'utf8');
      const undoUntil = async (pred, what) => {
        await page.click('.view-tab[data-view="view-editor"]');
        for (let i = 0; i < 6 && !pred(); i++) {
          await page.click('.editor-toolbar .btn-undo');
          await page.waitForTimeout(500);
        }
        await until(pred, what);
      };
      // Titre de l'onglet : « Site | Page », filtre du site réécrit, aperçu à jour.
      await page.click('.view-tab[data-view="view-settings"]');
      await page.click('.settings-nav-item[data-section="settings-general"]');
      await page.waitForSelector('.readable-title');
      await page.click('.readable-title button[data-mode="site"]');
      await until(() => JSON.parse(docR()).settings.filters.seo_title.expr.startsWith('(site.title ~'), 'format du titre');
      await page.waitForFunction(() => document.querySelector('.readable-title button[data-mode="site"]')?.classList.contains('is-on'));
      const sample = (await page.textContent('.readable-sample')).trim();
      check('réglages lisibles : titre de l\'onglet « Site | Page » (filtre réécrit, aperçu)', /^Alunites.* \| /.test(sample), sample);
      if (shots) await page.screenshot({ path: `${shots}/referencement-lisible.png` });
      // Image de partage par défaut : donnée du site.
      await page.fill('.readable .field >> nth=1 >> input', '/assets/img/essai.jpg');
      await page.press('.readable .field >> nth=1 >> input', 'Enter');
      await until(() => fs.readFileSync(siteYml, 'utf8').includes('share_image: /assets/img/essai.jpg'), 'image de partage');
      check('réglages lisibles : image de partage par défaut écrite dans les données (site.yml)', true);
      // Menu du site : texte d'un lien changé (affectation réécrite, canevas à jour).
      await page.click('.settings-nav-item[data-section="settings-navigation"]');
      await page.waitForSelector('.navlist-link');
      await page.fill('.navlist-link >> nth=0 >> .navlist-label', 'L’agence');
      await page.press('.navlist-link >> nth=0 >> .navlist-label', 'Tab');
      await until(() => docR().includes('L’agence'), 'lien renommé');
      await page.click('.navlist-link >> nth=1 >> button >> nth=0');
      const navExpr = () => {
        const find = (nodes) => {
          for (const n of nodes || []) {
            if (n.node === 'set' && n.name === 'nav') return n.expr;
            const r = find(n.children) || find(n.then) || find(n.otherwise);
            if (r) return r;
          }
          return null;
        };
        return find(JSON.parse(docR()).components.find((c) => c.id === 'partials/header').body) || '';
      };
      await until(() => /"label": "Projets"[^\]]*"label": "L’agence"/s.test(navExpr()), 'lien monté');
      if (shots) await page.screenshot({ path: `${shots}/navigation.png` });
      check('réglages lisibles : menu du site (lien renommé, ordre changé)', true);
      // Contenus mis en avant (paramètres de l'accueil) : un contenu retiré, un autre ajouté.
      await page.click('.view-tab[data-view="view-editor"]');
      await page.locator('.page-row', { has: page.locator('.list-meta', { hasText: /^\/$/ }) }).first().locator('.row-action').click();
      await page.waitForSelector('.picks .pick-item');
      const n0 = await page.locator('.pick[data-pick="pages.accueil.featured"] .pick-item').count();
      await page.locator('.pick[data-pick="pages.accueil.featured"] .pick-item >> nth=1').hover();
      await page.click('.pick[data-pick="pages.accueil.featured"] .pick-item >> nth=1 >> button >> nth=2');
      await until(() => fs.readFileSync(home, 'utf8') !== home0, 'contenu retiré');
      await page.waitForFunction((n) => document.querySelectorAll('.pick[data-pick="pages.accueil.featured"] .pick-item').length === n - 1, n0);
      const opt = await page.$eval('.pick[data-pick="pages.accueil.featured"] select option:nth-child(2)', (o) => o.value);
      await page.selectOption('.pick[data-pick="pages.accueil.featured"] select', opt);
      await page.waitForFunction((n) => document.querySelectorAll('.pick[data-pick="pages.accueil.featured"] .pick-item').length === n, n0);
      const featured = fs.readFileSync(home, 'utf8');
      check('réglages lisibles : contenus mis en avant (retiré, ajouté, écrits dans les données)', featured.includes(`- ${opt}`), opt);
      if (shots) await page.screenshot({ path: `${shots}/mis-en-avant.png` });
      await page.click('.page-settings-head .icon-btn');
      // Tout annulé : document et données identiques.
      await undoUntil(() => docR() === doc12 && fs.readFileSync(home, 'utf8') === home0 && fs.readFileSync(siteYml, 'utf8') === site0, 'réglages lisibles annulés');
      check('réglages lisibles : tout annulé (document, données)', true);
    }

    // ---------------------------------------------------------------- Publier en un clic (5.1)
    // Bouton « Publier » de la barre supérieure (hébergement simulé) : publication sans quitter
    // l'éditeur, date de la dernière publication mise à jour.
    {
      await page.click('.pub-btn');
      await page.waitForSelector('.pub-pop .pub-dest');
      const meta0 = (await page.textContent('.pub-dest-meta')).trim();
      const single = (await page.$$('.pub-check')).length === 0;
      await page.click('.pub-foot .btn--primary');
      await page.waitForSelector('.pub-dest .chip--ok', { timeout: 15000 });
      const meta1 = (await page.textContent('.pub-dest-meta')).trim();
      const stay = await page.isVisible('.view-tab.is-on[data-view="view-editor"]');
      if (shots) await page.locator('.pub-pop').screenshot({ path: `${shots}/publier.png` });
      await page.keyboard.press('Escape');
      await page.waitForSelector('.pub-pop', { state: 'detached' });
      check('publier en un clic : publié depuis la barre supérieure, sans quitter l\'éditeur',
        /Jamais publié/.test(meta0) && single && /Publié à l’instant/.test(meta1) && stay, `${meta0} -> ${meta1}`);
    }

    // ---------------------------------------------------------------- Mise en ligne guidée (5.2, 5.3)
    // Paramètres › Publication › Ajouter : hébergeur reconnu dans le message collé, accès remplis,
    // connexion d'essai (simulée), dossier proposé, hébergement enregistré.
    {
      await page.click('.pub-btn');
      await page.click('.pub-foot .pub-manage');
      await page.waitForSelector('.settings-page .host');
      const before = (await page.$$('.settings-page .host')).length;
      await page.click('.settings-page .page-head .btn');
      await page.waitForSelector('.setup');
      await page.fill('.setup-paste', 'Votre hébergement est installé.\nServeur FTP et SFTP : ftp.cluster031.hosting.ovh.net\nLogin principal : alunitp\nMot de passe : Xy7-kQ2pLm\n');
      const preset = await page.inputValue('.setup-preset');
      const filled = [await page.inputValue('.setup-server'), await page.inputValue('.setup-user'), (await page.inputValue('.setup-password')).length];
      const cleared = await page.inputValue('.setup-paste');
      await page.click('.setup .btn--primary');
      await page.waitForSelector('.setup-state.status--ok');
      await page.click('.setup-advanced > summary');
      const folder = await page.inputValue('.setup-folder');
      if (shots) await page.locator('.setup').screenshot({ path: `${shots}/mise-en-ligne.png` });
      await page.click('.setup .dialog-actions .btn:not(.btn--primary)');
      await page.waitForSelector('.setup', { state: 'detached' });
      await page.waitForFunction((n) => document.querySelectorAll('.settings-page .host').length === n + 1, before);
      check('mise en ligne guidée : OVHcloud reconnu, accès remplis, message effacé, dossier www/ proposé, hébergement ajouté',
        preset === 'ovh' && filled[0] === 'ftp.cluster031.hosting.ovh.net' && filled[1] === 'alunitp' && filled[2] === 10
          && cleared === '' && folder === 'www/', `${preset} ${filled.join(' ')} ${folder}`);
      await page.click('.view-tab[data-view="view-editor"]');
      await page.waitForSelector('.view-tab.is-on[data-view="view-editor"]');
    }

    // ---------------------------------------------------------------- Assistants IA (4.1)
    // Un assistant (MCP, même cœur) modifie un texte de la page et une donnée du site : l'éditeur
    // les affiche aussitôt ; Ctrl+Z dans l'éditeur les annule (même historique).
    if (mcpUrl && mcpToken) {
      const docM = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
      const homeM = path.join(siteDir, 'src/_data/pages/accueil.yml');
      const doc4 = docM();
      const home4 = fs.readFileSync(homeM, 'utf8');
      let rpc = 0;
      // Protocole 2026-07-28 : appel sans session, en-têtes du protocole.
      const mcp = async (name, args) => {
        const r = await fetch(mcpUrl, {
          method: 'POST',
          headers: {
            'content-type': 'application/json', accept: 'application/json, text/event-stream',
            authorization: `Bearer ${mcpToken}`, 'mcp-protocol-version': '2026-07-28',
            'mcp-method': 'tools/call', 'mcp-name': name,
          },
          body: JSON.stringify({ jsonrpc: '2.0', id: ++rpc, method: 'tools/call', params: {
            name, arguments: args,
            _meta: {
              'io.modelcontextprotocol/protocolVersion': '2026-07-28',
              'io.modelcontextprotocol/clientInfo': { name: 'editor-e2e', version: '1' },
              'io.modelcontextprotocol/clientCapabilities': {},
            },
          } }),
        });
        const j = await r.json();
        if (!j.result || j.result.isError) throw new Error(`MCP ${name} : ${JSON.stringify(j).slice(0, 300)}`);
        return j.result.content[0].text;
      };
      await page.click('.view-tab[data-view="view-editor"]');
      await page.click('.pages .page-row[data-url="/"]');
      await waitFrame(() => location.pathname === '/' && document.body.textContent.includes('Faire rayonner'), 'accueil');
      const outline = await mcp('get_page', { url: '/' });
      const line = outline.split('\n').find((l) => l.includes('« Faire rayonner »')) || '';
      const id = (/#(\S+)/.exec(line) || [])[1];
      // Dernière action relevée par l'application (simulée ici) : signalée dans la barre d'état.
      await page.evaluate((target) => { window.__revelMcpActs = [{ at: Date.now() + 500, client: 'claude-ai', tool: 'set_text', target }]; }, id);
      await mcp('set_text', { id, text: 'Rayonner avec un assistant' });
      await waitFrame(() => document.body.textContent.includes('Rayonner avec un assistant'), 'texte de l\'assistant affiché');
      await page.waitForSelector('.assistant-note');
      const note = (await page.textContent('.assistant-note')).trim();
      if (shots) await page.screenshot({ path: `${shots}/assistant-barre.png` });
      check('assistants (MCP) : texte de la page modifié par un assistant, affiché aussitôt dans l\'éditeur, signalé', !!id && note === 'Claude · Texte modifié', `#${id} — ${note}`);
      await page.evaluate(() => { delete window.__revelMcpActs; });
      await mcp('set_data', { path: 'pages.accueil.news_title', value: 'Les nouvelles vues par un assistant' });
      await until(() => fs.readFileSync(homeM, 'utf8') !== home4, 'donnée écrite');
      await waitFrame(() => document.body.textContent.includes('Les nouvelles vues par un assistant'), 'donnée affichée');
      const home4l = home4.split('\n');
      const changed = fs.readFileSync(homeM, 'utf8').split('\n').filter((l, i) => l !== home4l[i]);
      check('assistants (MCP) : donnée du site modifiée (seule sa ligne change), affichée', changed.length === 1, changed[0]);
      if (shots) await page.screenshot({ path: `${shots}/assistant-mcp.png` });
      for (let i = 0; i < 4 && (docM() !== doc4 || fs.readFileSync(homeM, 'utf8') !== home4); i++) {
        await page.click('.editor-toolbar .btn-undo');
        await page.waitForTimeout(500);
      }
      await until(() => docM() === doc4 && fs.readFileSync(homeM, 'utf8') === home4, 'modifications de l\'assistant annulées');
      await waitFrame(() => document.body.textContent.includes('Faire rayonner'), 'texte d\'origine');
      check('assistants (MCP) : annulé depuis l\'éditeur (document et données identiques)', true);
    }

    // ---------------------------------------------------------------- Langues (3.8)
    const docL = () => fs.readFileSync(path.join(siteDir, 'site.revel.json'), 'utf8');
    const dataEn = path.join(siteDir, 'src/_data/en');
    await page.click('.view-tab[data-view="view-locales"]');
    await page.click('.locales-add > summary');
    await page.click('.locales-menu .menu-item[data-code="en"]');
    await page.waitForSelector('.locales-table tr[data-locale="en"]');
    await until(() => /"locales": \[\s*\{\s*"code": "en"/.test(docL()), 'langue ajoutée');
    check('langues : anglais ajouté (adresse /en/, avancement)', (await page.textContent('.locales-table tr[data-locale="en"]')).includes('/en/'),
      (await page.textContent('.locales-table tr[data-locale="en"] .meter-num')).trim());
    // Textes des pages : traduction saisie dans le tableau.
    await page.fill('.locales-texts input[type="search"]', 'Faire rayonner');
    await page.waitForFunction(() => document.querySelectorAll('.loc-table tbody tr').length === 1);
    await page.fill('.loc-table .loc-input', 'Make it shine');
    await page.press('.loc-table .loc-input', 'Tab');
    await until(() => docL().includes('"Faire rayonner": "Make it shine"'), 'traduction enregistrée');
    check('langues : texte de page traduit (document du site)', true);
    // Données : écrites dans le dossier de la langue (copie du fichier).
    await page.fill('.locales-texts input[type="search"]', '');
    await page.click('.locales-texts .segmented button:nth-child(2)');
    await page.waitForSelector('.loc-table .loc-key');
    const dataKey = (await page.textContent('.loc-table tbody tr:first-child .loc-key')).trim();
    await page.fill('.loc-table tbody tr:first-child .loc-input', 'Data in English');
    await page.press('.loc-table tbody tr:first-child .loc-input', 'Tab');
    const enYaml = () => (fs.existsSync(dataEn) ? fs.readdirSync(dataEn, { recursive: true }) : []).filter((f) => /\.ya?ml$/.test(f));
    await until(() => enYaml().some((f) => fs.readFileSync(path.join(dataEn, f), 'utf8').includes('Data in English')), 'fichier de données de la langue');
    const enFiles = enYaml();
    check('langues : donnée traduite dans src/_data/en (copie du fichier)', enFiles.length === 1
      && fs.readFileSync(path.join(dataEn, enFiles[0]), 'utf8').includes('Data in English'), `${dataKey} → ${enFiles.join(',')}`);
    if (shots) await page.screenshot({ path: `${shots}/localisation.png` });
    // Traduction automatique hors ligne (D-36) : modèles présents (pont lancé avec REVEL_MODELS)
    // ou bouton indisponible, avec sa raison au survol (CI, sans modèles).
    await page.click('.locales-texts .segmented button:nth-child(1)');
    await page.waitForSelector('.locales-auto');
    await page.waitForTimeout(300);
    let autoSteps = 0;
    if (await page.$eval('.locales-auto', (b) => b.disabled)) {
      check('traduction auto : indisponible sans modèle (raison au survol)', /mod[eè]l|traduit/i.test(await page.getAttribute('.locales-auto', 'title')),
        await page.getAttribute('.locales-auto', 'title'));
    } else {
      await page.click('.locales-auto');
      await page.waitForSelector('.auto-dialog');
      const counts = await page.$$eval('.auto-part-count', (l) => l.map((x) => x.textContent.trim()));
      check('traduction auto : fenêtre (parties à traduire, mots gardés)', counts.length === 3 && !!(await page.$('.auto-dialog input[type="text"]')), counts.join(' · '));
      if (shots) await page.screenshot({ path: `${shots}/traduction-auto-fenetre.png` });
      // Pages seulement (données et contenus décochés : essai rapide).
      for (const n of [2, 3]) {
        const box = `.auto-dialog label.auto-part:nth-of-type(${n}) input`;
        if (await page.$eval(box, (i) => i.checked)) await page.click(`.auto-dialog label.auto-part:nth-of-type(${n}) .switch`);
      }
      await page.click('.auto-dialog button[type="submit"]');
      await page.waitForSelector('.auto-run');
      if (shots) await page.screenshot({ path: `${shots}/traduction-auto-en-cours.png` });
      await page.waitForSelector('.auto-run', { state: 'detached', timeout: 300000 });
      await until(() => /"machine": \[/.test(docL()), 'marques à relire');
      const enDoc = JSON.parse(docL()).locales.find((l) => l.code === 'en');
      const marked = enDoc.machine.filter((k) => k.startsWith('text:'));
      check('traduction auto : textes des pages traduits, marqués « à relire », saisie gardée',
        marked.length > 3 && marked.every((k) => enDoc.texts[k.slice(5)]) && enDoc.texts['Faire rayonner'] === 'Make it shine' && !marked.includes('text:Faire rayonner'),
        `${marked.length} textes ; ${marked.slice(0, 2).map((k) => `${k.slice(5)} → ${enDoc.texts[k.slice(5)]}`).join(' ; ')}`);
      await page.waitForSelector('.loc-table tr.is-auto .loc-review');
      if (shots) await page.screenshot({ path: `${shots}/traduction-auto.png` });
      // Relu : marque retirée ; filtre « À relire ».
      await page.click('.loc-table tr.is-auto .loc-review');
      await until(() => JSON.parse(docL()).locales[0].machine.length === enDoc.machine.length - 1, 'marque retirée');
      await page.click('.locales-review');
      await page.waitForTimeout(200);
      const rows = await page.$$eval('.loc-table tbody tr:not(.loc-review-bar)', (l) => l.map((r) => r.classList.contains('is-auto')));
      check('traduction auto : relu (marque retirée), filtre « À relire »', rows.length === enDoc.machine.length - 1 && rows.every(Boolean)
        && !!(await page.$('.loc-review-bar')), `${rows.length} lignes`);
      await page.click('.locales-review');
      autoSteps = 2;
    }
    // Contenus : ouverture dans le CMS, langue choisie.
    await page.click('.locales-texts .segmented button:nth-child(3)');
    await page.waitForSelector('.loc-table tr[data-entry]');
    await page.click('.loc-table tr[data-entry]');
    await page.waitForSelector('.cms-locale.is-other');
    await page.waitForSelector('.cms-item-body');
    check('langues : contenu ouvert dans le CMS en anglais (à traduire)', (await page.$$('.cms-table .loc-dot:not(.is-done)')).length > 0
      && await page.$eval('.cms-new', (b) => b.disabled));
    if (shots) await page.screenshot({ path: `${shots}/cms-langue.png` });
    // Éditeur : page en anglais (adresse, lang, texte traduit).
    await page.click('.view-tab[data-view="view-editor"]');
    f = await waitFrame(() => location.pathname.startsWith('/en/') && document.body.textContent.includes('Make it shine'), 'page en anglais');
    check('langues : canevas en anglais (/en/, <html lang="en">, texte traduit)', await f.evaluate(() => document.documentElement.lang === 'en'), f.url());
    check('langues : langue affichée dans la barre de l\'éditeur', (await page.$eval('.editor-locale select', (s) => s.value)) === 'en'
      && !!(await page.$('.editor-locale.is-other')));
    if (shots) await page.screenshot({ path: `${shots}/editeur-anglais.png` });
    // Texte écrit modifié sur place en anglais : traduction (le texte français ne change pas).
    const tL = await slotPoint("s.slot === 'text' && s.parent.owner.kind === 'page'");
    if (tL) {
      const before = tL.text.trim();
      await page.mouse.dblclick(tL.px, tL.py);
      f = await waitFrame(() => !!document.querySelector('[data-rv-edit]'), 'mode édition (anglais)');
      await page.keyboard.press('Control+a');
      await page.keyboard.type('Typed in English');
      await page.keyboard.press('Enter');
      await until(() => docL().includes(': "Typed in English"'), 'traduction depuis le canevas');
      check('langues : texte modifié dans le canevas → traduction', !JSON.parse(docL()).pages.some((p) => JSON.stringify(p.body).includes('Typed in English')), before);
    }
    // Langue principale, puis tout annuler : fichiers identiques.
    await page.selectOption('.editor-locale select', '');
    f = await waitFrame(() => !location.pathname.startsWith('/en/'), 'retour en français');
    for (let i = 0; i < 6 + autoSteps && (docL() !== doc0 || fs.existsSync(dataEn)); i++) {
      await page.click('.editor-toolbar .btn-undo');
      await page.waitForTimeout(400);
    }
    await until(() => docL() === doc0 && !fs.existsSync(dataEn), 'langue annulée');
    check('langues : tout annulé (document, dossier de la langue retiré)', true);
  }

  // Réglages de Revel ouverts depuis le site : menu du site gardé, retour au projet.
  {
    await page.click('.site-menu-btn');
    await page.click('.site-menu-pop .menu-item:has-text("Réglages de Revel")');
    await page.waitForSelector('.section[data-section="translate"] :is(.packs-more, .packs-none, .pack)');
    const kept = await page.locator('.site-menu-btn').count() === 1;
    // Rail des parties (D-104) : six parties, la partie choisie montrée et marquée.
    const rail = await page.$$eval('.app-settings .settings-nav-item', (b) => b.map((x) => x.dataset.section));
    await page.click('.app-settings .settings-nav-item[data-section="updates"]');
    await page.waitForFunction(() => {
      const c = document.querySelector('.app-settings .settings-content').getBoundingClientRect().top;
      const t = document.querySelector('.settings-sub[data-sub="updates"]').getBoundingClientRect().top;
      return Math.abs(t - c) < 40 || document.querySelector('.app-settings .settings-nav-group.is-on [data-section="updates"]');
    });
    check('réglages de Revel : rail des parties, aller à une partie', rail.length === 6, rail.join(', '));
    if (shots) await page.screenshot({ path: `${shots}/reglages-revel.png` });
    // Retour au site d'un clic (bouton de la barre, hors du menu du site).
    const back = (await page.textContent('.topbar-back')).trim();
    await page.click('.site-menu-btn');
    const inMenu = await page.$$eval('.site-menu-pop .menu-item', (l) => l.some((x) => x.textContent.includes('Revenir')));
    await page.keyboard.press('Escape');
    await page.click('.topbar-back');
    await page.waitForSelector('iframe.canvas-frame');
    check('réglages : menu du site gardé, retour au projet d\'un clic', kept && /^Revenir à /.test(back) && !inMenu, back);
  }

  // ---------------------------------------------------------------- Langues de traduction (D-36)
  await page.click('.topbar button.logo');
  await page.click('.view-tab[data-view="nav-settings"]');
  const TR = '.section[data-section="translate"]';
  await page.waitForSelector(`${TR} :is(.packs-more, .packs-none, .pack)`);
  // Langues installées seulement ; les autres sous « Plus de langues ».
  const shownFirst = await page.$$eval(`${TR} .pack`, (l) => l.every((x) => !!x.querySelector('.pack-remove')));
  // Aucune langue installée : dit sans ouvrir la liste.
  const noneLine = (await page.$(`${TR} .packs-none`)) !== null;
  const nShown = (await page.$$(`${TR} .pack`)).length;
  await page.click(`${TR} .packs-more`);
  await page.waitForSelector(`${TR} .packs .pack .pack-get`);
  check('réglages : langues installées, puis « Plus de langues »', shownFirst && noneLine === (nShown === 0), `${nShown} installée(s)`);
  const packs = await page.$$eval(`${TR} .pack`, (l) => l.map((x) => [x.dataset.pack, x.querySelector('.pack-get') ? 'get' : x.querySelector('.pack-remove') ? 'rm' : '?']));
  const bundled = packs.filter(([, a]) => a === 'rm').map(([c]) => c);
  check('réglages : langues de traduction (catalogue, chacune retirable ou téléchargeable)', packs.length >= 20
    && packs.every(([, a]) => a !== '?'), packs.map(([c, a]) => c + (a === 'rm' ? '✓' : '')).join(' '));
  // Langue livrée retirée puis remise en service (sans téléchargement).
  if (bundled.includes('fr')) {
    await page.click(`${TR} .pack[data-pack="fr"] .pack-remove`);
    await page.click('.dialog .btn--danger');
    await page.waitForSelector(`${TR} .pack[data-pack="fr"] .pack-get`);
    const off = await page.textContent(`${TR} .pack[data-pack="fr"] .pack-get`);
    await page.click(`${TR} .pack[data-pack="fr"] .pack-get`);
    await page.waitForSelector(`${TR} .pack[data-pack="fr"] .pack-remove`);
    check('réglages : langue livrée retirée, puis remise sans téléchargement', true, off.trim());
  }
  // Téléchargement simulé (sans `dl=1`, l'interface ne télécharge pas réellement) : avancement.
  const getBtn = `${TR} .pack:has(.pack-get) .pack-get`;
  const getCode = await page.$eval(getBtn, (b) => b.closest('.pack').dataset.pack);
  await page.click(getBtn);
  await page.waitForSelector(`${TR} .pack[data-pack="${getCode}"] .pack-bar`);
  const others = await page.$$eval(`${TR} .pack-get`, (l) => l.every((b) => b.disabled));
  if (shots) await page.screenshot({ path: `${shots}/reglages-traduction.png` });
  await page.waitForSelector(`${TR} .pack[data-pack="${getCode}"] .pack-bar`, { state: 'detached', timeout: 20000 });
  check('réglages : téléchargement d\'une langue (avancement, un seul à la fois)', others, getCode);

  // Réglages › Comptes : nom inscrit sur les versions (travail à plusieurs, D-89).
  {
    await page.locator('.settings-author').scrollIntoViewIfNeeded();
    const help = await page.getAttribute('.settings-author', 'title');
    await page.fill('.settings-author', 'Anne Martin');
    await page.locator('.settings-author').blur();
    await page.waitForFunction(() => window.__revelSettings?.author === 'Anne Martin');
    await page.fill('.settings-author', '');
    await page.locator('.settings-author').blur();
    await page.waitForFunction(() => window.__revelSettings && !window.__revelSettings.author);
    check('réglages : nom inscrit sur les versions (enregistré puis retiré)', /qui a modifié quoi/.test(help || ''));
  }

  // Profil (D-106) : en haut à droite, photo et nom modifiés sans quitter l'écran ; photo
  // recadrée et gardée dans les réglages, retirée ; mêmes réglages dans Réglages › Comptes.
  {
    const png = fs.readFileSync(path.join(siteDir, 'src/static/apple-touch-icon.png'));
    await page.click('.profile-btn');
    await page.waitForSelector('.profile-pop');
    await page.fill('.profile-pop .settings-author', 'Anne Martin');
    await page.locator('.profile-pop .settings-author').blur();
    await page.waitForFunction(() => window.__revelSettings?.author === 'Anne Martin');
    const shown = await page.getAttribute('.profile-btn', 'title');
    const nameInBar = await page.$$eval('.topbar .profile-btn', (b) => b[0].textContent.replace(/AM/, '').trim());
    const initials = (await page.textContent('.profile-btn .avatar')).trim();
    await page.setInputFiles('.profile-pop .profile-file', { name: 'photo.png', mimeType: 'image/png', buffer: png });
    await page.waitForFunction(() => /^data:image\/(webp|png);base64,/.test(window.__revelSettings?.avatar || ''));
    await page.waitForSelector('.profile-btn .avatar img');
    const inSettings = await page.$$eval('.settings-sub[data-sub="accounts"] .avatar img', (l) => l.length);
    if (shots) await page.screenshot({ path: `${shots}/profil.png` });
    await page.click('.profile-pop .profile-picture-reset');
    await page.waitForFunction(() => !window.__revelSettings?.avatar);
    await page.fill('.profile-pop .settings-author', '');
    await page.locator('.profile-pop .settings-author').blur();
    await page.waitForFunction(() => !window.__revelSettings?.author);
    await page.keyboard.press('Escape');
    check('profil : nom et photo (recadrée, gardée, retirée), mêmes réglages dans Comptes',
      shown === 'Anne Martin' && initials === 'AM' && nameInBar === '' && inSettings === 1, `${shown} ${initials}`);
  }

  // Réglages › Outils de construction (D-88) : Node.js téléchargé pour l'import construit,
  // paquets en cache vidés, outil retiré.
  {
    const card = '.section[data-section="build-tools"]';
    await page.locator(card).scrollIntoViewIfNeeded();
    await page.waitForSelector(`${card} .pack[data-tool="node-24.22.0"]`);
    const words = (sel) => page.$$eval(`${sel} *`, (l) => l.filter((x) => !x.children.length).map((x) => x.textContent.trim()).filter(Boolean).join(' '));
    const row = await words(`${card} .pack[data-tool="node-24.22.0"]`);
    const cache = await words(`${card} .build-cache`);
    if (shots) await page.locator(card).screenshot({ path: `${shots}/reglages-outils.png` });
    await page.click(`${card} .build-cache-clear`);
    await page.waitForSelector(`${card} .build-cache`, { state: 'detached' });
    await page.click(`${card} .build-tool-remove`);
    await page.waitForSelector(`${card} .packs-none`);
    check('réglages : outils de construction (taille, paquets en cache vidés, outil retiré)',
      /^Node\.js 24\.22\.0 \d+ Mo$/.test(row) && /^Paquets en cache \d+ Mo Vider$/.test(cache), `${row} · ${cache}`);
  }

  // Réglages › Mises à jour : préversions demandées puis retirées (D-86), recherche relancée.
  {
    await page.evaluate(() => { window.__revelUpdates = true; });
    const card = '.section[data-section="updates"]';
    const status = (re) => page.waitForFunction(([c, r]) => new RegExp(r).test(document.querySelector(`${c} .update-status`)?.textContent || ''), [card, re]);
    await page.locator(card).scrollIntoViewIfNeeded();
    await page.click(`${card} .update-actions .btn >> nth=0`);
    await status('à jour');
    const help = await page.getAttribute(`${card} .label:has-text("Recevoir les préversions")`, 'title');
    await page.click(`${card} .switch`);
    await status('^Préversion 0\\.7\\.1-beta\\.1 disponible$');
    await page.waitForSelector('.update-pill');
    const pill = (await page.textContent('.update-pill')).trim();
    const saved = await page.evaluate(() => window.__revelSettings?.prereleases === true);
    if (shots) await page.screenshot({ path: `${shots}/reglages-preversions.png` });
    await page.click(`${card} .switch`);
    await status('à jour');
    const pillGone = (await page.$('.update-pill')) === null;
    const off = await page.evaluate(() => window.__revelSettings?.prereleases === false);
    await page.evaluate(() => { window.__revelUpdates = false; });
    check('réglages : préversions demandées puis retirées (enregistrées, recherche relancée, « Préversion » dans la barre)',
      !!help && pill === 'Préversion 0.7.1-beta.1' && saved && pillGone && off, pill);
  }

  // Réglages › Assistants IA (4.2, serveur simulé) : activation, port, clé, configurations.
  {
    const state = () => page.textContent('.mcp-state');
    await page.locator('.mcp').scrollIntoViewIfNeeded();
    const off = (await state()).trim();
    await page.click('.mcp-server .switch');
    await page.waitForFunction(() => /127\.0\.0\.1:47520/.test(document.querySelector('.mcp-state')?.textContent || ''));
    check('assistants : serveur local activé (état et adresse)', /Désactivé/.test(off), (await state()).trim());
    await page.fill('.mcp-port', '47600');
    await page.press('.mcp-port', 'Enter');
    await page.waitForFunction(() => /:47600/.test(document.querySelector('.mcp-state')?.textContent || ''));
    await page.fill('.mcp-port', '80');
    await page.press('.mcp-port', 'Enter');
    await page.waitForTimeout(200);
    check('assistants : port changé (adresses suivies), port réservé refusé', await page.inputValue('.mcp-port') === '47600'
      && (await page.textContent('.mcp-openapi .mcp-key-value')) === 'http://127.0.0.1:47600/openapi.json');
    const snip = () => page.textContent('.snippet-code');
    const desktop = JSON.parse(await snip());
    await page.click('.mcp-clients [data-client="claude-code"]');
    const code = await snip();
    await page.click('.mcp-clients [data-client="codex"]');
    const codex = await snip();
    check('assistants : configurations prêtes (Claude Desktop, Claude Code, Codex)',
      desktop.mcpServers.revel.args[0] === '--mcp' && /revel\.exe$/.test(desktop.mcpServers.revel.command)
      && /^claude mcp add .* revel -- ".*revel\.exe" --mcp$/.test(code) && /\[mcp_servers\.revel\]/.test(codex), code);
    await page.click('.mcp-clients [data-client="open-webui"]');
    const fields = await page.$$eval('.mcp-field-value', (l) => l.map((e) => e.textContent));
    const hidden = fields[4];
    await page.click('.mcp-key-show');
    const key1 = await page.textContent('.mcp-key-value');
    await page.click('.mcp-key-renew');
    await page.click('.dialog .btn--primary');
    await page.waitForFunction((k) => document.querySelector('.mcp-key-value')?.textContent !== k, key1);
    const key2 = await page.textContent('.mcp-key-value');
    check('assistants : Open WebUI (adresse, clé masquée), clé affichée puis renouvelée',
      fields[1] === 'http://127.0.0.1:47600/mcp' && /^•+$/.test(hidden) && /^[0-9a-f]{64}$/.test(key2) && key2 !== key1, fields[1]);
    if (shots) await page.locator('.mcp').screenshot({ path: `${shots}/reglages-assistants.png` });
    await page.click('.mcp-server .switch');
    await page.waitForFunction(() => /Désactivé/.test(document.querySelector('.mcp-state')?.textContent || ''));
  }

  check('aucune erreur dans l\'interface', errors.length === 0, errors.join(' | '));
} catch (e) {
  check('déroulement', false, e.message);
  if (shots && shotPage) await shotPage.screenshot({ path: `${shots}/echec.png` }).catch(() => {});
} finally {
  await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
