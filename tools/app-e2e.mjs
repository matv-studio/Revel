#!/usr/bin/env node
// Essai de bout en bout de l'application de bureau (étape 2.9) : la vraie application Tauri,
// pilotée par WebDriver (`tauri-driver`, sous Linux avec WebKitWebDriver dans un écran virtuel
// Xvfb). Vérifie ce que les essais dans un navigateur ne couvrent pas : commandes Tauri réelles,
// protocole `revel-canvas` de l'application, cadre du canevas dans la vue web du système.
//
// L'application ouvre directement le site dans l'éditeur : variable REVEL_OPEN dans l'environnement
// de tauri-driver (héritée par l'application), ou option `--open DOSSIER` de ce script, transmise
// comme argument de l'application (Linux seulement : sous Windows, Edge WebDriver remet les
// arguments à WebView2, pas à l'application). Les essais modifient le site puis annulent : utiliser
// une copie, comparée ensuite à l'original.
// Usage : REVEL_OPEN=DOSSIER_DU_SITE tauri-driver &
//         node tools/app-e2e.mjs CHEMIN_DE_L_APPLICATION [--open DOSSIER_DU_SITE] [--shot CAPTURE.png]

import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
const opt = (name) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv.splice(i, 2)[1] : undefined;
};
const site = opt('--open');
const shot = opt('--shot');
const [app] = argv;
if (!app) {
  console.error('usage : app-e2e.mjs APPLICATION [--open DOSSIER_DU_SITE] [--shot CAPTURE.png]');
  process.exit(2);
}
const W = 'http://127.0.0.1:4444';
const results = [];
const check = (name, ok, detail = '') => {
  results.push(ok);
  console.log(`${ok ? 'ok ' : 'ÉCHEC'} ${name}${detail ? ' — ' + detail : ''}`);
};
const ELEMENT = 'element-6066-11e4-a52e-4f735466cecf';

async function wd(method, path, body) {
  const r = await fetch(W + path, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(`${method} ${path} : ${r.status} ${JSON.stringify(j.value || j).slice(0, 300)}`);
  return j.value;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const KEY = { Delete: '\uE017', Control: '\uE009', Escape: '\uE00C' };
const OWNER = /(Page|Gabarit|Composant|Layout|Component) /;
const SAVED = /(Enregistré à|Saved at) \d\d:\d\d/;

let sid = null;
try {
  const s = await wd('POST', '/session', {
    capabilities: {
      alwaysMatch: {
        'tauri:options': { application: path.resolve(app), ...(site ? { args: ['--open', path.resolve(site)] } : {}) },
        browserName: 'wry',
      },
    },
  });
  sid = s.sessionId;
  const S = `/session/${sid}`;
  const find = (css) => wd('POST', `${S}/elements`, { using: 'css selector', value: css });
  const exec = (script, args = []) => wd('POST', `${S}/execute/sync`, { script, args });
  // Touches (actions W3C) envoyées à l'élément qui a le focus dans le contexte courant.
  const keys = (...combo) => wd('POST', `${S}/actions`, {
    actions: [{
      type: 'key',
      id: 'clavier',
      actions: [...combo.map((k) => ({ type: 'keyDown', value: KEY[k] || k })), ...[...combo].reverse().map((k) => ({ type: 'keyUp', value: KEY[k] || k }))],
    }],
  });
  // Script exécuté dans le canevas (cadre retrouvé à chaque fois : la page peut être rechargée).
  const inCanvas = async (script) => {
    await wd('POST', `${S}/frame`, { id: null });
    const l = await find('iframe.canvas-frame');
    await wd('POST', `${S}/frame`, { id: l[0] });
    return exec(script);
  };
  const top = () => wd('POST', `${S}/frame`, { id: null });
  const clickInCanvas = async (css) => {
    await inCanvas('return 1');
    const el = await find(css);
    await wd('POST', `${S}/element/${el[0][ELEMENT]}/click`, {});
  };
  const waitFor = async (what, fn, tries = 120) => {
    for (let i = 0; i < tries; i++) {
      try {
        const v = await fn();
        if (v) return v;
      } catch { /* pas encore prêt */ }
      await sleep(250);
    }
    throw new Error('attendu : ' + what);
  };

  const frames = await waitFor('cadre du canevas', async () => {
    const l = await find('iframe.canvas-frame');
    const src = l.length ? await exec('return document.querySelector("iframe.canvas-frame").src') : '';
    return l.length && src ? l : null;
  });
  const src = await exec('return document.querySelector("iframe.canvas-frame").src');
  check('éditeur ouvert au démarrage, canevas par le protocole de l\'application', /^(revel-canvas:\/\/localhost\/|https?:\/\/revel-canvas\.localhost\/)/.test(src), src);

  // Dans le cadre du canevas.
  await wd('POST', `${S}/frame`, { id: frames[0] });
  const inside = await waitFor('page du canevas', async () => {
    const r = await exec('return [document.readyState, document.querySelectorAll("[data-rv]").length, typeof window.__revelSlot, document.title]');
    return r[0] === 'complete' && r[1] > 0 && r[2] === 'function' ? r : null;
  });
  check('page du site rendue avec ses repères', inside[1] > 50, `${inside[1]} éléments repérés, « ${inside[3]} »`);
  const css = await exec('const l = document.querySelector("link[rel~=stylesheet]"); return l && l.sheet ? l.sheet.cssRules.length : -1');
  check('feuille de style générée chargée par le protocole', css > 100, `${css} règles`);
  // Zoom 100 % : Edge WebDriver calcule mal le point de clic dans un cadre réduit par `transform`
  // (« element click intercepted »).
  await top();
  await exec('const s = document.querySelector(".editor-zoom select"); s.value = "1"; s.dispatchEvent(new Event("change", { bubbles: true }));');
  await waitFor('zoom 100 %', async () => /^(|none|scale\(1\))$/.test(await exec('return document.querySelector("iframe.canvas-frame").style.transform')));
  await clickInCanvas('h1');
  await top();
  const status = await waitFor('sélection dans la barre d\'état', async () => {
    const t = await exec('return document.querySelector(".editor-status") && document.querySelector(".editor-status").textContent');
    return OWNER.test(t || '') ? t : null;
  });
  check('clic dans le canevas : sélection transmise à l\'interface', true, status.trim().replace(/\s+/g, ' '));
  const layers = await exec('return document.querySelectorAll(".layers .layer").length');
  check('calques affichés', layers > 20, `${layers} lignes`);

  // Raccourcis dans le canevas : commandes réelles de l'application, fichiers écrits puis rétablis.
  // Le clic atteint l'élément le plus profond (souvent un texte dans le titre) : on compte les
  // éléments repérés.
  const rvCount = 'return document.readyState === "complete" && window.__revelSlot ? document.querySelectorAll("[data-rv]").length : -1';
  const count = await inCanvas(rvCount);
  await clickInCanvas('h1');
  await keys('Delete');
  const fewer = await waitFor('Suppr dans le canevas', async () => {
    const n = await inCanvas(rvCount);
    return n >= 0 && n < count ? n : null;
  });
  check('Suppr dans le canevas : élément retiré', true, `${count} → ${fewer} éléments repérés`);
  await top();
  const saved = await waitFor('état d\'enregistrement', async () => {
    const t = await exec('return document.querySelector(".save-state") && document.querySelector(".save-state").textContent');
    return SAVED.test(t || '') ? t : null;
  });
  check('état d\'enregistrement affiché', true, saved.trim());
  await clickInCanvas('main');
  await keys('Control', 'z');
  await waitFor('Ctrl+Z dans le canevas', async () => (await inCanvas(rvCount)) === count);
  check('Ctrl+Z dans le canevas : élément rétabli', true);
  await top();
  if (shot) {
    await top();
    const png = await wd('GET', `${S}/screenshot`);
    fs.writeFileSync(shot, Buffer.from(png, 'base64'));
  }
} catch (e) {
  check('déroulement', false, e.message);
  if (sid) {
    // Diagnostic : état de l'interface au moment de l'échec.
    try {
      await wd('POST', `/session/${sid}/frame`, { id: null });
      const info = await wd('POST', `/session/${sid}/execute/sync`, {
        script: 'return [location.href, document.querySelector(".notice") && document.querySelector(".notice").textContent, document.body.innerText.slice(0, 400)]',
        args: [],
      });
      console.log('interface :', JSON.stringify(info));
      if (shot) fs.writeFileSync(shot, Buffer.from(await wd('GET', `/session/${sid}/screenshot`), 'base64'));
    } catch (d) {
      console.log('diagnostic impossible :', d.message);
    }
  }
} finally {
  if (sid) await wd('DELETE', `/session/${sid}`).catch(() => {});
}
const failed = results.filter((r) => !r).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
