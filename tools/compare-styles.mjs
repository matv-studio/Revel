#!/usr/bin/env node
// Équivalence de deux feuilles de style sur les mêmes pages (étape 2.1, D-18).
//
// Les deux dossiers contiennent le même site, seule la feuille de style diffère (CSS d'origine
// contre CSS généré par Revel). Chaque page est ouverte dans Chromium (Playwright) avec les deux
// versions, à plusieurs largeurs (--widths ; la CI passe chaque point de rupture du site) et avec les
// deux préférences de mouvement ; une fois l'état
// stabilisé (images chargées ou en échec, polices prêtes, vidéos arrêtées, animations terminées),
// les styles calculés de chaque élément et de ses pseudo-éléments sont comparés.
//
// Les styles calculés sont comparés plutôt que des captures : les captures d'un même site
// varient d'une fois à l'autre (images différées, bandeau défilant, apparitions au défilement),
// alors que les styles calculés reflètent exactement la cascade appliquée.
//
// Avec --shots, des captures pleine page (sans préférence de mouvement) sont aussi comparées. Le dessin
// des images mises à l'échelle et du texte tourné varie d'une capture à l'autre d'un même site (jusqu'à
// 30 sur 255 par canal observé, sur des milliers de pixels) : un pixel compte comme différent au-delà de
// --tolerance (48 par canal) et une paire est en écart au-delà de --max-pixels pixels différents (50).
// Une paire en écart est capturée une seconde fois avant d'être signalée. Les écarts de valeurs plus
// fins sont couverts par la comparaison des styles calculés.
//
// Usage : node tools/compare-styles.mjs DOSSIER_A DOSSIER_B [--pages /,/agence/] [--widths 1280,375]
//         [--jobs 4] [--shots] [--tolerance 48] [--max-pixels 50] [--save DOSSIER]
// --save : captures des paires en écart enregistrées dans DOSSIER (a-… et b-…).
// DOSSIER_B peut être une adresse (http://…) : pages déjà servies, par exemple le canevas de
// l'éditeur (`revel canvas-serve`) ; les éléments propres au canevas (calque de sélection, script,
// liste des repères) sont alors ignorés.
// Sans --pages : toutes les pages HTML du dossier A. Code de sortie 1 en cas d'écart.

import { createServer } from 'node:http';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { extname, join, normalize, relative, sep } from 'node:path';

// `require` suit NODE_PATH (Playwright installé globalement ou dans un dossier à part).
const { chromium } = createRequire(import.meta.url)('playwright');

const args = process.argv.slice(2);
const opt = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 ? args.splice(i, 2)[1] : def;
};
const widths = opt('--widths', '1280,860,375').split(',').map(Number);
const jobsN = Number(opt('--jobs', '4'));
const tolerance = Number(opt('--tolerance', '48'));
const maxPixels = Number(opt('--max-pixels', '50'));
const saveDir = opt('--save', null);
const shots = args.includes('--shots');
if (shots) args.splice(args.indexOf('--shots'), 1);
let pages = opt('--pages', null)?.split(',');
const [dirA, dirB] = args;
if (!dirA || !dirB) {
  console.error('usage : compare-styles.mjs DOSSIER_A DOSSIER_B [--pages …] [--widths …] [--jobs N]');
  process.exit(2);
}

const TYPES = {
  '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript',
  '.mjs': 'text/javascript', '.json': 'application/json', '.svg': 'image/svg+xml',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp',
  '.avif': 'image/avif', '.gif': 'image/gif', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
  '.woff': 'font/woff', '.ttf': 'font/ttf', '.mp4': 'video/mp4', '.webm': 'video/webm',
  '.xml': 'application/xml', '.txt': 'text/plain',
};

/** Serveur de fichiers statiques sur un port libre ; renvoie son adresse. */
function serve(root) {
  const server = createServer(async (req, res) => {
    try {
      let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
      if (p.endsWith('/')) p += 'index.html';
      const file = normalize(join(root, p));
      if (relative(root, file).startsWith('..')) throw new Error('hors du dossier');
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': TYPES[extname(file).toLowerCase()] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end();
    }
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok([server, `http://127.0.0.1:${server.address().port}`])));
}

async function htmlPages(root, dir = root, out = []) {
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) await htmlPages(root, p, out);
    else if (e.name.endsWith('.html')) out.push('/' + relative(root, p).split(sep).join('/').replace(/(^|\/)index\.html$/, '$1'));
  }
  return out.sort();
}

async function settle(page) {
  await page.evaluate(async () => {
    for (const i of document.images) i.loading = 'eager';
    // decode() attend le chargement et le décodage (sinon une image chargée peut ne pas être dessinée).
    await Promise.all([...document.images].map((i) => Promise.race([
      i.decode().catch(() => null), new Promise((r) => setTimeout(r, 8000)),
    ])));
    // Image pas chargée (échec passager du serveur sous la charge) : redemandée une fois.
    await Promise.all([...document.images].filter((i) => i.currentSrc && !(i.complete && i.naturalWidth)).map((i) => {
      const src = i.currentSrc;
      i.src = src + (src.includes('?') ? '&' : '?') + 'rv-retry=1';
      return Promise.race([i.decode().catch(() => null), new Promise((r) => setTimeout(r, 8000))]);
    }));
    // Toutes les polices déclarées, pas seulement celles déjà demandées (sinon une graisse chargée
    // tardivement change la mise en page d'une capture à l'autre).
    await Promise.all([...document.fonts].map((f) => f.load().catch(() => null)));
    await document.fonts.ready;
    for (const v of document.querySelectorAll('video')) { v.pause(); v.currentTime = 0; }
  });
  await page.waitForTimeout(200);
  await page.evaluate(async () => {
    // Calque de sélection du canevas (survol déclenché par un déplacement simulé du pointeur) :
    // il ne fait pas partie du site.
    document.querySelector('revel-overlay')?.style.setProperty('display', 'none', 'important');
    // Éléments vides montrés par le canevas (icône, hauteur) : rendus comme dans le site.
    for (const e of document.querySelectorAll('[data-rv-empty]')) e.removeAttribute('data-rv-empty');
    for (const a of document.getAnimations()) {
      try { a.finish(); } catch { a.pause(); a.currentTime = 0; } // animation infinie
    }
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  });
}

/** Écart entre deux captures PNG, mesuré dans une page Chromium (canevas). */
async function pixelDiff(cmp, a, b) {
  return cmp.evaluate(async ([a, b, tol]) => {
    const load = async (s) => {
      const img = new Image();
      img.src = 'data:image/png;base64,' + s;
      await img.decode();
      return img;
    };
    const [A, B] = [await load(a), await load(b)];
    if (A.width !== B.width || A.height !== B.height) return { size: `${A.width}×${A.height} / ${B.width}×${B.height}` };
    const pixels = (img) => {
      const c = new OffscreenCanvas(img.width, img.height);
      const g = c.getContext('2d', { willReadFrequently: true });
      g.drawImage(img, 0, 0);
      return g.getImageData(0, 0, img.width, img.height).data;
    };
    const [da, db] = [pixels(A), pixels(B)];
    let over = 0, max = 0, top = -1;
    for (let i = 0; i < da.length; i += 4) {
      const m = Math.max(Math.abs(da[i] - db[i]), Math.abs(da[i + 1] - db[i + 1]), Math.abs(da[i + 2] - db[i + 2]));
      if (m > max) max = m;
      if (m > tol) { over++; if (top < 0) top = Math.floor(i / 4 / A.width); }
    }
    return { over, max, top };
  }, [a.toString('base64'), b.toString('base64'), tolerance]);
}

/**
 * Styles calculés de chaque élément et pseudo-élément, calculés dans la page. Renvoie une empreinte
 * par élément (propriétés triées : l'ordre des propriétés personnalisées varie d'une ouverture à
 * l'autre) ; `detail` : propriétés complètes de l'élément n° `detail` (en cas d'écart).
 */
function computedStyles(detail) {
  const out = [];
  let n = 0;
  // Éléments propres au canevas de l'éditeur : ignorés. En-tête du document (`<head>`) aussi :
  // jamais affiché ; les balises de référencement que la génération ajoute (D-92) n'y changent
  // aucun style.
  const own = (el) => el.tagName === 'REVEL-OVERLAY' || el.id === 'rv-slots'
    || (el.tagName === 'SCRIPT' && el.getAttribute('src') === '/__revel/canvas.js')
    || el.closest('head') !== null;
  for (const el of document.querySelectorAll('*')) {
    if (own(el)) continue;
    const pseudos = [null, '::before', '::after'];
    if (el.tagName === 'LI' || el.tagName === 'SUMMARY') pseudos.push('::marker');
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') pseudos.push('::placeholder');
    for (const ps of pseudos) {
      const k = n++;
      if (detail !== undefined && k !== detail) continue;
      const cs = getComputedStyle(el, ps);
      const v = [];
      for (let i = 0; i < cs.length; i++) v.push(`${cs[i]}: ${cs.getPropertyValue(cs[i])}`);
      v.sort();
      if (detail !== undefined) {
        const cls = typeof el.className === 'string' && el.className ? '.' + el.className.trim().split(/\s+/).join('.') : '';
        return [`${el.tagName.toLowerCase()}${cls}${ps ?? ''}`, v];
      }
      // FNV-1a 32 bits de la liste triée.
      let h = 0x811c9dc5;
      const str = v.join(';');
      for (let i = 0; i < str.length; i++) h = Math.imul(h ^ str.charCodeAt(i), 0x01000193);
      out.push(h >>> 0);
    }
  }
  return out;
}

const [srvA, urlA] = await serve(dirA);
const remoteB = /^https?:\/\//.test(dirB);
const [srvB, urlB] = remoteB ? [null, dirB.replace(/\/$/, '')] : await serve(dirB);
pages ??= await htmlPages(dirA);
for (const p of pages) {
  const f = join(dirA, p.endsWith('/') ? p + 'index.html' : p);
  if (!(await stat(f).catch(() => null))) {
    console.error(`page absente : ${p}`);
    process.exit(2);
  }
}

const browser = await chromium.launch();
const queue = [];
for (const motion of ['no-preference', 'reduce']) for (const w of widths) for (const p of pages) queue.push({ motion, w, p });
const total = queue.length;
let compared = 0;
const diffs = [];
let shotsCompared = 0, noiseMax = 0, noisePixels = 0, retakes = 0;

// Propriétés qui découlent des dimensions (changent dès qu'un descendant change de taille).
const SIZE = new Set(['height', 'block-size', 'width', 'inline-size', 'perspective-origin', 'transform-origin',
  'grid-template-rows', 'grid-template-columns']);

async function open(base, { motion, w, p }) {
  const ctx = await browser.newContext({ viewport: { width: w, height: 900 }, reducedMotion: motion });
  const page = await ctx.newPage();
  await page.goto(base + p, { waitUntil: 'load' });
  await settle(page);
  const hashes = await page.evaluate(computedStyles);
  const shot = shots && motion === 'no-preference'
    ? await page.screenshot({ fullPage: true, animations: 'disabled' })
    : null;
  return { ctx, page, hashes, shot };
}

async function worker() {
  const cmpCtx = await browser.newContext();
  const cmp = await cmpCtx.newPage();
  const shotDiff = async (a, b) => {
    if (a.equals(b)) return { over: 0, max: 0 };
    return pixelDiff(cmp, a, b);
  };
  const failed = (d) => d.size || d.over > maxPixels;
  for (let job = queue.shift(); job; job = queue.shift()) {
    const A = await open(urlA, job);
    const B = await open(urlB, job);
    try {
      if (A.shot && B.shot) {
        shotsCompared++;
        let [sa, sb] = [A.shot, B.shot];
        let d = await shotDiff(sa, sb);
        if (failed(d)) {
          // Seconde capture des deux côtés : un écart passager (chargement tardif) n'est pas signalé.
          retakes++;
          const [A2, B2] = [await open(urlA, job), await open(urlB, job)];
          await A2.ctx.close();
          await B2.ctx.close();
          [sa, sb] = [A2.shot, B2.shot];
          d = await shotDiff(sa, sb);
        }
        if (failed(d)) {
          diffs.push({ ...job, what: d.size ? `capture : dimensions ${d.size}` : `capture : ${d.over} pixels au-delà de la tolérance (écart max ${d.max}, à partir de y = ${d.top})` });
          if (saveDir) {
            await mkdir(saveDir, { recursive: true });
            const name = `${job.w}${job.p.replace(/[^a-z0-9]+/gi, '_')}.png`;
            await writeFile(join(saveDir, 'a-' + name), sa);
            await writeFile(join(saveDir, 'b-' + name), sb);
          }
        } else { noiseMax = Math.max(noiseMax, d.max); noisePixels = Math.max(noisePixels, d.over); }
      }
      compared += A.hashes.length;
      if (A.hashes.length !== B.hashes.length) {
        diffs.push({ ...job, what: `nombre d'éléments : ${A.hashes.length} / ${B.hashes.length}` });
        continue;
      }
      const bad = A.hashes.map((h, k) => (h !== B.hashes[k] ? k : -1)).filter((k) => k >= 0);
      if (bad.length) {
        // Élément le plus parlant : le premier dont l'écart ne se limite pas aux dimensions
        // (conséquence d'un écart plus bas dans la page, comme la hauteur de <html>).
        let shown = null;
        for (const i of bad.slice(0, 60)) {
          const [name, va] = await A.page.evaluate(computedStyles, i);
          const [, vb] = await B.page.evaluate(computedStyles, i);
          const [sa, sb] = [new Set(va), new Set(vb)];
          const da = va.filter((x) => !sb.has(x));
          const db = vb.filter((x) => !sa.has(x));
          const d = { i, name, da, db };
          shown ??= d;
          if (da.some((x) => !SIZE.has(x.split(':')[0]))) { shown = d; break; }
        }
        const { i, name, da, db } = shown;
        diffs.push({ ...job, what: `${bad.length} élément(s) ; ${name} (n° ${i}) : ${da.slice(0, 3).join(' ; ')} → ${db.slice(0, 3).join(' ; ')}` });
      }
    } finally {
      await A.ctx.close();
      await B.ctx.close();
    }
  }
  await cmpCtx.close();
}

await Promise.all(Array.from({ length: jobsN }, worker));
await browser.close();
srvA.close();
srvB?.close();

console.log(`${total} combinaisons (${pages.length} pages × ${widths.length} largeurs × 2 préférences de mouvement), ` +
  `${compared} styles calculés comparés (éléments et pseudo-éléments)` +
  (shots ? `, ${shotsCompared} paires de captures (bruit toléré : écart max ${noiseMax}/255, ` +
    `au plus ${noisePixels} pixel(s) au-delà de ${tolerance}, ${retakes} paire(s) reprise(s))` : '') +
  ` : ${diffs.length} écart(s).`);
for (const d of diffs.slice(0, 30)) console.log(`- ${d.p} à ${d.w} px (${d.motion}) : ${d.what}`);
process.exit(diffs.length ? 1 : 0);
