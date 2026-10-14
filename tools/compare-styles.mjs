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
// Avec --shots, des captures pleine page (sans préférence de mouvement) sont aussi comparées : le dessin
// des images mises à l'échelle varie légèrement d'une capture à l'autre d'un même site (quelques unités
// sur 255, rarement plus sur quelques pixels isolés). Un écart compte au-delà de --tolerance (8 sur 255
// par canal) sur plus de --max-pixels pixels (50) ; un vrai écart de mise en page ou de couleur touche
// des milliers de pixels.
//
// Usage : node tools/compare-styles.mjs DOSSIER_A DOSSIER_B [--pages /,/agence/] [--widths 1280,375]
//         [--jobs 4] [--shots] [--tolerance 8] [--max-pixels 50]
// Sans --pages : toutes les pages HTML du dossier A. Code de sortie 1 en cas d'écart.

import { createServer } from 'node:http';
import { readFile, readdir, stat } from 'node:fs/promises';
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
const tolerance = Number(opt('--tolerance', '8'));
const maxPixels = Number(opt('--max-pixels', '50'));
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
    await document.fonts.ready;
    for (const v of document.querySelectorAll('video')) { v.pause(); v.currentTime = 0; }
  });
  await page.waitForTimeout(200);
  await page.evaluate(async () => {
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

function computedStyles() {
  const out = [];
  let n = 0;
  for (const el of document.querySelectorAll('*')) {
    n++;
    const pseudos = [null, '::before', '::after'];
    if (el.tagName === 'LI' || el.tagName === 'SUMMARY') pseudos.push('::marker');
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') pseudos.push('::placeholder');
    for (const ps of pseudos) {
      const cs = getComputedStyle(el, ps);
      const v = [];
      for (let i = 0; i < cs.length; i++) v.push(`${cs[i]}: ${cs.getPropertyValue(cs[i])}`);
      const cls = typeof el.className === 'string' && el.className ? '.' + el.className.trim().split(/\s+/).join('.') : '';
      out.push([`${el.tagName.toLowerCase()}${cls}${ps ?? ''} (n° ${n})`, v]);
    }
  }
  return out;
}

const [srvA, urlA] = await serve(dirA);
const [srvB, urlB] = await serve(dirB);
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
let shotsCompared = 0, noiseMax = 0, noisePixels = 0;

async function load(base, { motion, w, p }) {
  const ctx = await browser.newContext({ viewport: { width: w, height: 900 }, reducedMotion: motion });
  try {
    const page = await ctx.newPage();
    await page.goto(base + p, { waitUntil: 'load' });
    await settle(page);
    const styles = await page.evaluate(computedStyles);
    const shot = shots && motion === 'no-preference'
      ? await page.screenshot({ fullPage: true, animations: 'disabled' })
      : null;
    return [styles, shot];
  } finally {
    await ctx.close();
  }
}

async function worker() {
  const cmpCtx = await browser.newContext();
  const cmp = await cmpCtx.newPage();
  for (let job = queue.shift(); job; job = queue.shift()) {
    const [[a, shotA], [b, shotB]] = [await load(urlA, job), await load(urlB, job)];
    if (shotA && shotB) {
      const d = await pixelDiff(cmp, shotA, shotB);
      shotsCompared++;
      if (d.size) diffs.push({ ...job, what: `capture : dimensions ${d.size}` });
      else if (d.over > maxPixels) diffs.push({ ...job, what: `capture : ${d.over} pixels au-delà de la tolérance (écart max ${d.max}, à partir de y = ${d.top})` });
      else { noiseMax = Math.max(noiseMax, d.max); noisePixels = Math.max(noisePixels, d.over); }
    }
    compared += a.length;
    if (a.length !== b.length) {
      diffs.push({ ...job, what: `nombre d'éléments : ${a.length} / ${b.length}` });
      continue;
    }
    for (let i = 0; i < a.length; i++) {
      const [name, va] = a[i];
      const vb = b[i][1];
      const da = va.filter((x) => !vb.includes(x));
      if (da.length) {
        const db = vb.filter((x) => !va.includes(x));
        diffs.push({ ...job, what: `${name} : ${da.slice(0, 3).join(' ; ')} → ${db.slice(0, 3).join(' ; ')}` });
        break;
      }
    }
  }
  await cmpCtx.close();
}

await Promise.all(Array.from({ length: jobsN }, worker));
await browser.close();
srvA.close();
srvB.close();

console.log(`${total} combinaisons (${pages.length} pages × ${widths.length} largeurs × 2 préférences de mouvement), ` +
  `${compared} styles calculés comparés (éléments et pseudo-éléments)` +
  (shots ? `, ${shotsCompared} paires de captures (bruit toléré : écart max ${noiseMax}/255, ` +
    `au plus ${noisePixels} pixel(s) au-delà de ${tolerance})` : '') +
  ` : ${diffs.length} écart(s).`);
for (const d of diffs.slice(0, 30)) console.log(`- ${d.p} à ${d.w} px (${d.motion}) : ${d.what}`);
process.exit(diffs.length ? 1 : 0);
