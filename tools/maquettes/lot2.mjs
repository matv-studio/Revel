// Maquettes du lot 2 (D-182) : interface réelle, panneaux retouchés dans la page avant capture.
import { chromium, open, invoke } from './base.mjs';
const OUT = '/home/claude/sp/shots';
const only = process.argv[3] ? process.argv[3].split(',') : null;
const theme = process.argv[2] || 'dark';
const CSS = `
.mq-set{color:var(--set-text)!important;background:var(--set-bg);border-radius:3px;padding:0 4px;margin-left:-4px}
.mq-line{display:flex;align-items:center;gap:6px}
.mq-thumb{width:28px;height:28px;border-radius:var(--radius-sm);background:var(--surface-hover);border:1px solid var(--line-strong);display:grid;place-items:center;flex:none;color:var(--text-muted)}
.mq-fake{flex:1;min-width:0;height:var(--control-md);display:flex;align-items:center;gap:6px;padding:0 8px;border:1px solid var(--line-strong);border-radius:var(--radius-md);background:var(--surface-raised);font-size:var(--text-md);color:var(--text);white-space:nowrap;overflow:hidden}
.mq-fake svg{flex:none;color:var(--text-muted)} .mq-fake .end{margin-left:auto}
.mq-sw{width:14px;height:14px;border-radius:3px;flex:none;border:1px solid var(--line-strong)}
.mq-list{display:flex;flex-direction:column;gap:4px}
.mq-item{display:flex;align-items:center;gap:6px;height:28px;padding:0 6px;border:1px solid var(--line-strong);border-radius:var(--radius-md);background:var(--surface-raised);font-size:var(--text-md);color:var(--text)}
.mq-item span.t{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.mq-item svg{flex:none;color:var(--text-faint)}
.mq-pop{position:fixed;z-index:9999;background:var(--surface);box-shadow:var(--shadow-dialog);border-radius:var(--radius-lg);padding:10px;width:270px;font-size:var(--text-md);color:var(--text);display:flex;flex-direction:column;gap:8px}
.mq-pop .h{display:flex;align-items:center;gap:6px;font-weight:var(--weight-medium)} .mq-pop .v{color:var(--text-muted);font-size:var(--text-sm)}
.mq-pop .btn{justify-content:flex-start;width:100%}
.mq-outline{position:fixed;z-index:9990;border:1.5px solid var(--c);pointer-events:none}
.mq-tag{position:fixed;z-index:9991;font:600 11px var(--font);padding:2px 6px;border-radius:3px 3px 0 0;color:#fff;background:var(--c);display:flex;gap:6px;align-items:center;white-space:nowrap}
.mq-tag i{font-style:normal;opacity:.6} .mq-tag b{font-weight:600;background:rgb(255 255 255/.25);padding:0 4px;border-radius:2px}
.mq-pill{font-size:11px;padding:1px 7px;border-radius:999px;background:var(--comp-bg);color:var(--comp);white-space:nowrap}
.mq-pill--n{background:var(--surface-hover);color:var(--text-muted)}
.mq-dim{opacity:.45}
.mq-head{display:flex;align-items:center;gap:8px;justify-content:space-between}
.mq-sub{display:flex;align-items:center;justify-content:space-between}
`;
const I = {
  grip: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="currentColor"><circle cx="6" cy="4" r="1"/><circle cx="10" cy="4" r="1"/><circle cx="6" cy="8" r="1"/><circle cx="10" cy="8" r="1"/><circle cx="6" cy="12" r="1"/><circle cx="10" cy="12" r="1"/></svg>',
  x: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>',
  plus: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M8 3.5v9M3.5 8h9"/></svg>',
  db: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5"><ellipse cx="8" cy="4" rx="5" ry="2"/><path d="M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8c0 1.1 2.2 2 5 2s5-.9 5-2"/></svg>',
  page: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><path d="M4 1.5h5l3 3v10H4z"/><path d="M9 1.5v3h3"/></svg>',
  img: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M2.5 11l3.5-3 3 2.5 2-1.5 2.5 2"/><circle cx="10.5" cy="6.5" r="1"/></svg>',
  reset: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 8a4.5 4.5 0 1 1 1.4 3.3M3.5 12V8.8h3.2"/></svg>',
  eye: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M1.5 8s2.4-4.5 6.5-4.5S14.5 8 14.5 8s-2.4 4.5-6.5 4.5S1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/></svg>',
  chev: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 6.5L8 10l3.5-3.5"/></svg>',
  pen: '<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><path d="M10.5 2.5l3 3-8 8H2.5v-3z"/></svg>',
};
const sel = (label, opts, cls = '') => `<label class="field ${cls}"><span class="label">${label}</span><div class="select"><select class="input">${opts.map((o) => `<option>${o}</option>`).join('')}</select></div></label>`;
const seg = (label, opts, on = 0) => `<div class="style-row"><span class="style-label"><span class="style-label-text">${label}</span></span><div class="style-control"><div class="segmented" role="radiogroup">${opts.map((o, i) => `<button class="seg${i === on ? ' is-on' : ''}" type="button">${o}</button>`).join('')}</div></div></div>`;
const segw = (label, opts, on = 0) => `<div class="field"><span class="label">${label}</span><div class="seg" role="radiogroup">${opts.map((o, i) => `<button role="radio" aria-checked="${i === on}" class="${i === on ? 'is-on' : ''}"><span>${o}</span></button>`).join('')}</div></div>`;
const sw = (label, on = true) => `<label class="view-line prop-row"><span class="prop-label">${label}</span><span class="switch"><input type="checkbox" ${on ? 'checked' : ''}><span class="switch-track"></span></span></label>`;

const browser = await chromium.launch();
const shot = async (page, name, clip) => { await page.waitForTimeout(350); await page.screenshot({ path: `${OUT}/${name}-${theme}.png`, clip }); console.log('ok', name); };
const prep = async () => { const s = await open(browser, theme); await s.page.addStyleTag({ content: CSS }); return s; };
// Rectangle d'un élément du canevas, en coordonnées de la fenêtre.
const rectOf = async (page, frameOf, selector, nth = 0) => {
  const r = await frameOf().evaluate(([s, n]) => { const e = document.querySelectorAll(s)[n]; if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.x, y: b.y, w: b.width, h: b.height }; }, [selector, nth]);
  if (!r) throw new Error('introuvable : ' + selector);
  const fr = await page.$eval('iframe.canvas-frame', (e) => { const b = e.getBoundingClientRect(); return { x: b.x, y: b.y, k: b.width / e.offsetWidth }; });
  return { x: fr.x + r.x * fr.k, y: fr.y + r.y * fr.k, w: r.w * fr.k, h: r.h * fr.k };
};
const mark = (page, r, label, color = 'var(--comp)', below = false) => page.evaluate(([r, label, color, below]) => {
  const o = document.createElement('div'); o.className = 'mq-outline'; o.style.cssText = `--c:${color};left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;
  const t = document.createElement('div'); t.className = 'mq-tag'; t.style.cssText = `--c:${color};left:${r.x - 1.5}px;top:${below ? r.y + r.h : r.y - 19}px;${below ? 'border-radius:0 0 3px 3px' : ''}`; t.innerHTML = label;
  document.body.append(o, t);
}, [r, label, color, below]);
const setPanel = (page, name, html) => page.evaluate(([name, html]) => {
  const n = document.querySelector('.style-layerbar-name'); if (name && n) n.textContent = name;
  document.querySelectorAll('.styles > .text-fields').forEach((e) => e.remove());
  let c = document.querySelector('.content-settings');
  if (!c) { c = document.createElement('div'); c.className = 'content-settings'; (document.querySelector('.styles') || document.querySelector('.dock-col--right .panel-pad')).prepend(c); }
  c.innerHTML = html;
}, [name, html]);

// Carte de composant d'essai (une fois).
const site = (await invoke('edit_open')).site;
if (!site.components.some((c) => c.id === 'carte-marque')) {
  const component = { id: 'carte-marque', name: 'Carte de marque',
    props: [{ name: 'titre', label: 'Titre', type: 'text', default: 'Alunites Studio' }, { name: 'texte', label: 'Texte', type: 'text', default: 'Films de marque et contenus pour les réseaux.' },
      { name: 'logo', label: 'Logo', type: 'image', default: '/assets/img/logo.svg' }, { name: 'lien', label: 'Lien', type: 'link', default: '/agence/' }, { name: 'badge', label: 'Badge', type: 'switch', default: 'true' }],
    body: [{ node: 'element', id: 'xcm00000', tag: 'article', attrs: [{ name: 'class', value: 'carte-marque' }, { name: 'style', value: 'margin:150px 24px 48px 48px;padding:28px;width:340px;display:inline-block;vertical-align:top;border:1px solid #3a3a3a;border-radius:14px' }], children: [
      { node: 'element', id: 'xcm00001', tag: 'h3', attrs: [], children: [{ node: 'out', expr: 'titre' }] },
      { node: 'element', id: 'xcm00002', tag: 'p', attrs: [], children: [{ node: 'out', expr: 'texte' }] }] }] };
  await invoke('edit_apply', { command: { cmd: 'add_component', component }, coalesce: false });
  for (const [i, id] of [[0, 'xum00000'], [1, 'xum00001']])
    await invoke('edit_apply', { command: { cmd: 'insert', parent: { owner: { kind: 'page', id: 'index' }, branch: 'children' }, index: i, node: { node: 'use', id, component: 'carte-marque', args: {} } }, coalesce: false });
}
const run = async (name, fn) => { if (only && !only.includes(name)) return; const s = await prep(); try { await fn(s); } catch (e) { console.log('ECHEC', name, e.message.split('\n')[0]); await s.page.screenshot({ path: `${OUT}/echec-${name}.png` }); } await s.page.close(); };

// A. Utilisation d'un composant : propriétés réglées par utilisation.
await run('a-composant', async ({ page, frameOf }) => {
  const card = page.frameLocator('iframe.canvas-frame').locator('.carte-marque').nth(1);
  await card.click(); await page.waitForSelector('.prop-row[data-prop="titre"] input');
  await page.fill('.prop-row[data-prop="titre"] input', 'Alunites Films'); await page.keyboard.press('Tab'); await page.waitForTimeout(1200);
  await page.evaluate(([I]) => {
    const g = document.querySelector('.content-settings .insp-group');
    const row = (label, inner, set) => `<div class="prop-row"><div class="field"><span class="label mq-sub"><span class="${set ? 'mq-set' : ''}">${label}</span>${set ? `<span style="color:var(--text-faint)">${I.reset}</span>` : ''}</span><span class="field-line">${inner}</span></div></div>`;
    g.innerHTML = `<p class="panel-title">Propriétés</p>
      <label class="field"><span class="label">Variante</span><div class="select"><select class="input"><option>Principale</option><option>Survol</option><option>Compacte</option></select></div></label>
      ${row('Titre', '<input class="input" value="Alunites Films">', true)}
      ${row('Texte', '<textarea class="input" rows="2" style="height:46px;resize:none;padding-top:5px" placeholder="Films de marque et contenus pour les réseaux."></textarea>')}
      ${row('Logo', `<span class="mq-thumb">${I.img}</span><span class="mq-fake">logo-films.svg</span>`, true)}
      ${row('Lien', `<span class="mq-fake">${I.page}<span>Agence</span><span class="end">${I.chev}</span></span>`)}
      ${row('Couleur', `<span class="mq-fake"><span class="mq-sw" style="background:#ff5a3c"></span>#FF5A3C</span>`, true)}
      <label class="view-line prop-row"><span class="prop-label">Badge</span><span class="switch"><input type="checkbox" checked><span class="switch-track"></span></span></label>`;
  }, [I]);
  await shot(page, 'a-composant');
});

// B. Double-clic sur un texte relié au CMS : plus de modification muette du contenu.
await run('b-texte-cms', async ({ page, frameOf }) => {
  const s = '[class*="card"] h3, [class*="card"] [class*="title"]';
  await frameOf().evaluate((s) => document.querySelector(s).scrollIntoView({ block: 'center' }), s);
  await page.waitForTimeout(700);
  await page.frameLocator('iframe.canvas-frame').locator(s).first().click(); await page.waitForTimeout(700);
  await page.frameLocator('iframe.canvas-frame').locator(s).first().dblclick(); await page.waitForTimeout(1200);
  await shot(page, 'b0-actuel');
  await page.keyboard.press('Escape'); await page.waitForTimeout(500);
  await page.frameLocator('iframe.canvas-frame').locator(s).first().click(); await page.waitForTimeout(700);
  const r = await rectOf(page, frameOf, s);
  const ro = (l, v) => `<div class="field"><span class="label mq-line">${I.db.replace('width="16" height="16"', 'width="12" height="12"')}${l}</span><span class="mq-fake" style="color:var(--text-muted);background:transparent">${v}</span></div>`;
  await setPanel(page, null, `<div class="insp-group"><p class="panel-title">Contenu</p><span class="mq-fake" style="flex:none">${I.db}<span>Liste · Projets</span></span></div>
    <div class="insp-group"><p class="panel-title">Champs CMS</p>${ro('Client', 'Caisse d’Épargne Rhône-Alpes')}${ro('Titre', '1,8 millions d’impressions pour un rapp…')}${ro('Expertises', 'Identité & stratégie, Digital & activation')}
    <button class="btn btn--small" type="button" style="width:100%;justify-content:center">${I.pen}<span>Modifier ce contenu dans le CMS</span></button></div>`);
  const txt = await frameOf().evaluate((s) => document.querySelector(s).textContent.trim().replace(/\s+/g, ' '), s);
  await mark(page, r, `${I.db.replace('width="16" height="16"', 'width="12" height="12"')} Projets <i>›</i> Titre`, 'var(--text-muted)');
  await page.evaluate(([r, I, txt]) => {
    const p = document.createElement('div'); p.className = 'mq-pop'; p.style.left = r.x + 'px'; p.style.top = (r.y + r.h + 8) + 'px';
    p.innerHTML = `<div class="h">${I.db}<span>Champ CMS · Projets › Titre</span></div><div class="v">« ${txt.slice(0, 60)}… »</div>
      <button class="btn btn--small">${I.pen}<span>Modifier ce contenu dans le CMS</span></button>
      <button class="btn btn--small">${I.pen}<span>Modifier le composant</span></button>`;
    document.body.append(p);
  }, [r, I, txt]);
  await shot(page, 'b-texte-cms');
});

// C. Outil « Champ CMS » : deux menus, aucune expression.
await run('c-champ-cms', async ({ page, frameOf }) => {
  const s = '[class*="card"] h3, [class*="card"] [class*="title"]';
  await frameOf().evaluate((s) => document.querySelector(s).scrollIntoView({ block: 'center' }), s);
  await page.waitForTimeout(700);
  await page.frameLocator('iframe.canvas-frame').locator(s).first().click(); await page.waitForTimeout(700);
  await page.click('[data-action="component-edit"]'); await page.waitForTimeout(3000);
  await page.locator('.layers .layer', { hasText: 'Card title' }).first().click(); await page.waitForTimeout(900);
  await frameOf().evaluate(() => { const h = document.querySelector('h3, [class*="title"]'); h && h.scrollIntoView({ block: 'center' }); }); await page.waitForTimeout(600);
  await page.screenshot({ path: `${OUT}/etat-c.png` });
  await setPanel(page, 'Champ CMS', `<div class="stack"><p class="tool-head"><span class="tool-icon">${I.db}</span><span>Champ CMS</span></p>
    ${sel('Collection', ['Projets', 'Actualités', 'Expertises', 'Équipe', "Offres d'emploi"])}
    ${sel('Champ', ['Titre', 'Client', 'Date', 'Accroche', 'Image principale', 'Adresse de la page'])}
    ${segw('Affiché en', ['Texte', 'Image', 'Lien'], 0)}
    ${sel('Si le champ est vide', ['Masquer l’élément', 'Montrer un texte de remplacement'])}</div>`);
  await page.evaluate(([I]) => { const l = document.querySelector('.layers .layer.is-selected'); if (l) { l.querySelector('.layer-label').textContent = 'Champ CMS'; let p = l.querySelector('.layer-preview'); if (!p) { p = document.createElement('span'); p.className = 'layer-preview'; l.querySelector('.layer-label').after(p); } p.textContent = 'Projets › Titre'; l.querySelector('.layer-icon').innerHTML = I.db; l.querySelector('.layer-marks').innerHTML = ''; }
    for (const o of document.querySelectorAll('.layers .layer')) { const t = o.querySelector('.layer-label'); if (t.textContent.trim() === 'Image') { t.textContent = 'Champ CMS'; const p = document.createElement('span'); p.className = 'layer-preview'; p.textContent = 'Projets › Image principale'; t.after(p); o.querySelector('.layer-icon').innerHTML = I.db; o.querySelector('.layer-marks').innerHTML = ''; } } }, [I]);
  await shot(page, 'c-champ-cms');
});

// D. Liste de collection : automatique ou choix manuel.
await run('d-liste', async ({ page, frameOf }) => {
  const row = page.locator('.layers .layer', { hasText: 'Liste · Projets' }).first();
  await row.scrollIntoViewIfNeeded(); await row.click(); await page.waitForTimeout(900);
  const cs = '[class*="card"] h3, [class*="card"] [class*="title"]';
  await frameOf().evaluate((s) => { let g = document.querySelector(s).closest('[class*="card"]').parentElement; while (g.children.length < 2) g = g.parentElement; g.setAttribute('data-mq', '1'); const y = g.getBoundingClientRect().top + window.scrollY - 150; window.scrollTo(0, y - 60); document.documentElement.scrollTop = y - 60; }, cs);
  await page.waitForTimeout(700);
  const head = `<div class="insp-group collection-list"><p class="panel-title">Liste de collection</p>${sel('Collection', ['Projets', 'Actualités', 'Expertises', 'Équipe', "Offres d'emploi"])}`;
  const card = `<button class="btn btn--small" type="button" style="width:100%;justify-content:center">${I.pen}<span>Modifier la carte</span></button></div>`;
  const item = (t) => `<div class="mq-item">${I.grip}<span class="t">${t}</span>${I.x}</div>`;
  await setPanel(page, null, `${head}${segw('Contenus', ['Automatique', 'Choix manuel'], 0)}
    <div class="field-pair">${sel('Tri', ['Date', 'Titre', 'Ordre de la collection'])}${sel('Ordre', ['Décroissant', 'Croissant'])}</div>
    <label class="field"><span class="label">Nombre de contenus</span><input class="input" type="number" value="6"></label>
    <div class="field"><span class="label">Filtres</span><div class="cond-editor"><div class="cond-row"><div class="select"><select class="input"><option>Expertises</option></select></div><div class="select"><select class="input"><option>contient</option></select></div></div><div class="select"><select class="input"><option>Film &amp; storytelling</option></select></div>
    <button class="btn btn--small cond-add" type="button">${I.plus}<span>Ajouter un filtre</span></button></div></div>${card}`);
  await shot(page, 'd1-liste-auto');
  await setPanel(page, null, `${head}${segw('Contenus', ['Automatique', 'Choix manuel'], 1)}
    <div class="field"><span class="label mq-sub"><span>Contenus choisis</span><span style="color:var(--text-faint)">4</span></span><div class="mq-list">
    ${['WATTR, une histoire électrique.', 'Milord Circus – L’art du trompe-l’œil', 'Force Pro 500', 'Rowenta Freemove'].map(item).join('')}</div>
    <button class="btn btn--small cond-add" type="button" style="margin-top:6px">${I.plus}<span>Ajouter un contenu</span></button></div>${card}`);
  await shot(page, 'd2-liste-manuel');
});

// E. En-tête et pied : une zone commune, plus de gabarits dans l'interface.
await run('e-entete-pied', async ({ page, frameOf }) => {
  await page.locator('.layers .layer', { hasText: 'Header' }).first().click(); await page.waitForTimeout(900);
  await mark(page, await rectOf(page, frameOf, 'header.site-header'), 'En-tête · tout le site', 'var(--comp)', true);
  await page.evaluate(([I]) => {
    for (const l of document.querySelectorAll('.layers .layer')) { const t = l.querySelector('.layer-label');
      if (t.textContent.trim() === 'Header') { t.textContent = 'En-tête'; l.querySelector('.layer-marks').innerHTML = '<span class="mq-pill">Tout le site</span>'; }
      if (t.textContent.trim() === 'Footer') { t.textContent = 'Pied'; l.querySelector('.layer-marks').innerHTML = '<span class="mq-pill">Tout le site</span>'; } }
    document.querySelectorAll('.layout-row').forEach((e) => e.remove());
    document.querySelectorAll('.list-section-title').forEach((e) => { if (/Gabarits/.test(e.textContent)) e.remove(); });
    const tree = document.querySelector('.page-tree'); if (tree) tree.scrollTop = tree.scrollHeight;
    document.querySelector('.style-layerbar-name').textContent = 'En-tête';
    const a = document.querySelector('.instance-actions'); if (a) a.innerHTML = '<span class="mq-pill">Tout le site</span>';
    document.querySelectorAll('.styles > .text-fields').forEach((e) => e.remove());
    document.querySelector('.content-settings').innerHTML = `<div class="insp-group"><p class="panel-title">En-tête</p>
      <label class="view-line prop-row"><span class="prop-label">Montré sur cette page</span><span class="switch"><input type="checkbox" checked><span class="switch-track"></span></span></label></div>`;
    const st = document.querySelector('.editor-status'); if (st) st.innerHTML = st.innerHTML.replace(/Gabarit [^<]*/, 'En-tête · tout le site').replace(/Composant [^<]*/, 'En-tête · tout le site');
  }, [I]);
  await shot(page, 'e-entete-pied');
});

// F. Conditions : branche montrée indiquée dans le canevas et les calques.
await run('f-conditions', async ({ page, frameOf }) => {
  const cond = page.locator('.layers .layer', { hasText: 'Condition' }).first();
  await cond.scrollIntoViewIfNeeded(); await cond.click(); await page.waitForSelector('.cond-tool');
  await page.locator('.cond-tool .segmented .seg', { hasText: /^Si$/ }).click(); await page.waitForTimeout(1500);
  let r = null; try { r = await rectOf(page, frameOf, '.env-badge'); } catch {}
  if (r) await mark(page, r, 'Condition · Staging <b>Si</b> <i>Sinon</i>', 'var(--warning-text)');
  await page.evaluate(([I]) => {
    const rows = [...document.querySelectorAll('.layers .layer')]; const i = rows.findIndex((l) => /Condition/.test(l.textContent));
    const set = (l, d) => { l.style.setProperty('--depth', d); l.dataset.depth = d; };
    const then = rows[i + 1], other = rows[i + 2], inElse = rows[i + 3];
    const si = other.cloneNode(true); si.querySelector('.layer-label').textContent = 'Si'; set(si, 2); si.querySelector('.layer-marks').innerHTML = `<span class="mq-pill mq-pill--n" style="display:inline-flex;gap:4px;align-items:center">${I.eye.replace('width="16" height="16"', 'width="12" height="12"')}montrée</span>`;
    then.before(si); set(then, 3); set(other, 2); other.classList.add('mq-dim'); if (inElse) { set(inElse, 3); inElse.classList.add('mq-dim'); }
    document.querySelector('.cond-code')?.remove();
    const lab = document.querySelector('[data-row="cond-show"] .style-label-text'); if (lab) lab.textContent = 'Montrer';
    cond_scroll: { const t = document.querySelector('.layers-tree'); if (t) t.scrollTop = t.scrollHeight; }
  }, [I]);
  await shot(page, 'f-conditions');
});
await browser.close();
