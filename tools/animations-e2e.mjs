// Essais du script d'animation du site (6.1), sans serveur : page de toutes les animations servie en
// mémoire avec le script et l'en-tête produits par Revel (`crates/revel-render/src/animations.*`).
// Mouvement accepté, mouvement réduit, sans JavaScript, script absent.
// Usage : node tools/animations-e2e.mjs
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(here + '/')('playwright');
const dir = path.join(here, '../crates/revel-render/src');
const script = fs.readFileSync(path.join(dir, 'animations.js'), 'utf8');
// En-tête ajouté aux pages animées : chaînes de `HEAD` dans animations.rs.
const rs = fs.readFileSync(path.join(dir, 'animations.rs'), 'utf8');
const headSrc = rs.slice(rs.indexOf('const HEAD: &str = concat!('), rs.indexOf('\n);', rs.indexOf('const HEAD')));
const head = [...headSrc.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => m[1].replace(/\\"/g, '"')).join('')
  + '<script src="/assets/revel-animations.js" defer></script>';
const FAMILIES = {
  appear: ['fade', 'up', 'down', 'left', 'right', 'zoom-in', 'zoom-out', 'blur', 'pop', 'flip', 'rotate', 'reveal'],
  text: ['fade', 'up', 'blur', 'pop', 'flip', 'typewriter'],
  hover: ['grow', 'shrink', 'lift', 'shadow', 'dim', 'tilt', 'brighten', 'underline'],
  press: ['press', 'squish', 'push'],
  loop: ['spin', 'pulse', 'float', 'swing', 'shake', 'blink', 'bounce', 'heartbeat', 'ticker'],
  scroll: ['parallax', 'fade-in', 'zoom-in', 'rotate', 'slide-x', 'fade-out'],
};
const body = [];
for (const [f, ps] of Object.entries(FAMILIES)) {
  for (const p of ps) {
    let extra = f === 'text' && p === 'up' ? ' split=lines' : '';
    if (f === 'appear' && p === 'fade') extra += ' stagger=0.1';
    if (f === 'loop' && p === 'ticker') body.push(`<div id="${f}-${p}" data-revel-${f}="${p} pause" style="display:flex;gap:20px"><span>Un</span><span>Deux</span><span>Trois</span></div>`);
    else if (f === 'text') body.push(`<h2 id="${f}-${p}" data-revel-${f}="${p}${extra}">Texte animé ${p} avec <a href="#">un lien</a> et plusieurs mots pour faire des lignes</h2>`);
    else body.push(`<div class="b" id="${f}-${p}" data-revel-${f}="${p}${extra}"><span>${f} ${p}</span><span>b</span></div>`);
  }
}
const html = `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Animations</title>${head}<style>.b{width:200px;height:60px;margin:40px;background:#3d7eff;color:#fff}</style></head><body>${body.join('\n')}</body></html>`;
const URL = 'http://revel.test/';
const serve = async (page, missing = false) => {
  await page.route('http://revel.test/**', (r) => {
    if (r.request().url().endsWith('/assets/revel-animations.js')) {
      return missing ? r.fulfill({ status: 404, body: '' }) : r.fulfill({ contentType: 'text/javascript', body: script });
    }
    return r.fulfill({ contentType: 'text/html; charset=utf-8', body: html });
  });
};
const browser = await chromium.launch();
const results = [];
const check = (name, ok, info = '') => { results.push(ok); console.log(`${ok ? 'ok ' : 'ÉCHEC'} ${name}${info ? ' — ' + info : ''}`); };

// Mouvement accepté.
{
  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
  await serve(page);
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(URL);
  await page.waitForFunction(() => !!window.RevelAnimations);
  check('classe rv-anim posée', await page.evaluate(() => document.documentElement.classList.contains('rv-anim')));
  // Éléments en haut : apparition jouée.
  await page.waitForTimeout(100);
  const first = await page.evaluate(() => {
    const el = document.getElementById('appear-up');
    return { in: el.classList.contains('rv-in'), anims: el.getAnimations().length };
  });
  check('apparition à l\'écran : jouée', first.in && first.anims > 0, JSON.stringify(first));
  const stag = await page.evaluate(() => [...document.getElementById('appear-fade').children].map((c) => c.getAnimations()[0]?.effect.getTiming().delay));
  check('décalage entre enfants', stag.length === 2 && stag[1] - stag[0] === 100, JSON.stringify(stag));
  const below = await page.evaluate(() => getComputedStyle(document.getElementById('text-flip')).opacity);
  check('texte hors de l\'écran : masqué en attendant', below === '0', below);
  // Tout parcourir : chaque élément animé, aucune erreur.
  const ids = await page.$$eval('[id]', (l) => l.map((e) => e.id));
  for (const id of ids) {
    await page.evaluate((i) => document.getElementById(i).scrollIntoView({ block: 'center' }), id);
    await page.waitForTimeout(60);
  }
  await page.waitForTimeout(1500);
  const state = await page.evaluate(() => [...document.querySelectorAll('[id]')].map((el) => {
    const f = el.id.split('-')[0];
    const visible = getComputedStyle(el).opacity !== '0';
    let n = el.getAnimations({ subtree: true }).length;
    return [el.id, f, visible, n, el.classList.contains('rv-in')];
  }));
  const hidden = state.filter(([, , v]) => !v).map(([i]) => i);
  check('tous les éléments visibles après leur passage à l\'écran', hidden.length === 0, hidden.join(' '));
  const loops = state.filter(([, f, , n]) => f === 'loop' && n === 0).map(([i]) => i);
  check('boucles en cours', loops.length === 0, loops.join(' '));
  const scrolls = state.filter(([, f, , n]) => f === 'scroll' && n === 0).map(([i]) => i);
  check('animations de défilement en place', scrolls.length === 0, scrolls.join(' '));
  const pieces = await page.evaluate(() => [...document.querySelectorAll('[data-revel-text]')].map((e) => [e.id, e.querySelectorAll('.rv-w').length, e.querySelectorAll('.rv-c').length, !!e.querySelector('.rv-sr'), !!e.querySelector('a')]));
  check('texte découpé (mots ou lettres), texte lu gardé, lien gardé', pieces.every(([, w, , sr, a]) => w > 5 && sr && a)
    && pieces.find(([i]) => i === 'text-typewriter')[2] > 20, JSON.stringify(pieces));
  // Bandeau : copies masquées aux lecteurs d'écran.
  const tick = await page.evaluate(() => {
    const el = document.getElementById('loop-ticker');
    return { track: !!el.querySelector('.rv-track'), hidden: el.querySelectorAll('[aria-hidden="true"]').length, n: el.querySelector('.rv-track').children.length };
  });
  check('bandeau : contenu recopié', tick.track && tick.hidden >= 3 && tick.n >= 6, JSON.stringify(tick));
  // Survol : animation vers l'état actif puis retour.
  await page.evaluate(() => document.getElementById('hover-grow').scrollIntoView({ block: 'center' }));
  const box = await page.locator('#hover-grow').boundingBox();
  await page.mouse.move(box.x + 20, box.y + 20);
  await page.waitForTimeout(400);
  const m = await page.evaluate(() => getComputedStyle(document.getElementById('hover-grow')).transform);
  check('survol : agrandi', /matrix\(1\.05/.test(m), m);
  await page.mouse.move(5, 5);
  await page.waitForTimeout(400);
  const m2 = await page.evaluate(() => getComputedStyle(document.getElementById('hover-grow')).transform);
  check('survol quitté : revenu', m2 === 'none' || /matrix\(1, 0, 0, 1, 0, 0\)/.test(m2), m2);
  // Défilement : parallaxe suit la position.
  const par = async () => page.evaluate(() => getComputedStyle(document.getElementById('scroll-parallax')).transform);
  await page.evaluate(() => { const e = document.getElementById('scroll-parallax'); scrollTo(0, e.offsetTop - innerHeight + 10); });
  await page.waitForTimeout(150);
  const p1 = await par();
  await page.evaluate(() => { const e = document.getElementById('scroll-parallax'); scrollTo(0, e.offsetTop - 10); });
  await page.waitForTimeout(150);
  const p2 = await par();
  check('parallaxe : déplacement selon le défilement', p1 !== p2, `${p1} → ${p2}`);
  check('aucune erreur (mouvement accepté)', errors.length === 0, errors.join(' | '));
  await page.close();
}

// Mouvement réduit demandé : rien d'animé, tout visible.
{
  const ctx = await browser.newContext({ reducedMotion: 'reduce' });
  const page = await ctx.newPage();
  await serve(page);
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(URL);
  await page.waitForFunction(() => !!window.RevelAnimations);
  const r = await page.evaluate(() => ({
    cls: document.documentElement.classList.contains('rv-anim'),
    hidden: [...document.querySelectorAll('[data-revel-appear],[data-revel-text]')].filter((e) => getComputedStyle(e).opacity === '0').length,
    anims: document.getAnimations().length,
  }));
  check('mouvement réduit : rien d\'animé, tout visible', !r.cls && r.hidden === 0 && r.anims === 0, JSON.stringify(r));
  check('aucune erreur (mouvement réduit)', errors.length === 0, errors.join(' | '));
  await ctx.close();
}

// Sans JavaScript : tout visible.
{
  const ctx = await browser.newContext({ javaScriptEnabled: false });
  const page = await ctx.newPage();
  await serve(page);
  await page.goto(URL);
  const hidden = await page.$$eval('[data-revel-appear],[data-revel-text]', (l) => l.filter((e) => getComputedStyle(e).opacity === '0').length);
  check('sans JavaScript : tout visible', hidden === 0, String(hidden));
  await ctx.close();
}

// Script absent : contenu montré au bout de 3 s.
{
  const page = await browser.newPage();
  await serve(page, true);
  await page.goto(URL);
  await page.waitForTimeout(3300);
  const hidden = await page.$$eval('[data-revel-appear],[data-revel-text]', (l) => l.filter((e) => getComputedStyle(e).opacity === '0').length);
  check('script absent : contenu montré après 3 s', hidden === 0, String(hidden));
  await page.close();
}
await browser.close();
const failed = results.filter((x) => !x).length;
console.log(`${results.length - failed}/${results.length}`);
process.exit(failed ? 1 : 0);
