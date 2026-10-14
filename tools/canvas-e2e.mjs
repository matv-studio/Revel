#!/usr/bin/env node
// Essais de bout en bout du canevas de l'éditeur (étape 2.2), dans Chromium (Playwright).
//
// Le canevas est servi par `revel canvas-serve` (même réponses que le protocole de l'application) ;
// il est chargé dans un cadre d'une autre origine, comme dans l'application, et les messages
// échangés avec la page parente sont vérifiés.
//
// Usage : node tools/canvas-e2e.mjs http://127.0.0.1:8160 [/agence/]
// Code de sortie 1 si un essai échoue.

import { createServer } from 'node:http';
import { createRequire } from 'node:module';

const { chromium } = createRequire(import.meta.url)('playwright');
const [base, page = '/agence/'] = process.argv.slice(2);
if (!base) {
  console.error('usage : canvas-e2e.mjs ADRESSE_DU_CANEVAS [PAGE]');
  process.exit(2);
}
const canvasUrl = base.replace(/\/$/, '') + page;

// Page parente : reçoit les messages du canevas.
const harness = `<!doctype html><meta charset="utf-8"><title>essai</title>
<style>html,body{margin:0}iframe{border:0;width:1280px;height:900px}</style>
<iframe id="c" src="${canvasUrl}"></iframe>
<script>
  window.messages = [];
  addEventListener('message', (e) => { if (e.data && e.data.revel === 1) messages.push(e.data); });
  window.send = (m) => document.getElementById('c').contentWindow.postMessage(Object.assign({ revel: 1 }, m), '*');
</script>`;
const server = createServer((_, res) => {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  res.end(harness);
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const harnessUrl = `http://127.0.0.1:${server.address().port}/`;

const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok });
  console.log(`${ok ? 'ok ' : 'ÉCHEC'} ${name}${detail ? ' — ' + detail : ''}`);
};

const browser = await chromium.launch();
try {
  const parent = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await parent.goto(harnessUrl);
  const frame = parent.frameLocator('#c');
  const messages = () => parent.evaluate(() => window.messages);
  const waitFor = async (pred, what) => {
    for (let i = 0; i < 100; i++) {
      const m = (await messages()).find(pred);
      if (m) return m;
      await parent.waitForTimeout(100);
    }
    throw new Error(`message attendu : ${what}`);
  };

  const ready = await waitFor((m) => m.type === 'ready', 'ready');
  check('page prête, repères transmis', ready.slots > 0 && ready.path === page, `${ready.slots} repères`);

  const f = parent.frames().find((x) => x.url().startsWith(base));
  const dom = await f.evaluate(() => {
    const it = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_COMMENT);
    let marks = 0;
    while (it.nextNode()) if (/^\/?rv:\d+$/.test(it.currentNode.data)) marks++;
    return {
      marks,
      slotsScript: !!document.getElementById('rv-slots'),
      ownScript: !!document.querySelector('script[src="/__revel/canvas.js"]'),
      elements: document.querySelectorAll('[data-rv]').length,
      firstText: (() => {
        for (let n = 0; n < 2000; n++) {
          const s = window.__revelSlot(n);
          if (!s.slot) return null;
          if (s.slot.slot === 'text' && s.range) return s.range.toString();
        }
        return null;
      })(),
      ranges: (() => {
        let ok = 0, missing = 0;
        for (let n = 0; ; n++) {
          const s = window.__revelSlot(n);
          if (!s.slot) break;
          if (s.range) ok++; else missing++;
        }
        return { ok, missing };
      })(),
    };
  });
  check('repères retirés de la page (commentaires, liste, script)', dom.marks === 0 && !dom.slotsScript && !dom.ownScript);
  check('plages des repères retrouvées', dom.ranges.missing === 0 && dom.ranges.ok === ready.slots,
    `${dom.ranges.ok} plages, premier texte « ${(dom.firstText || '').trim().slice(0, 40)} »`);

  // Clic : sélection de l'élément, sans action du site.
  const h1 = frame.locator('h1').first();
  const h1Id = await h1.getAttribute('data-rv');
  await h1.click();
  // L'élément sélectionné est le plus proche repéré sous le pointeur : le titre ou un de ses enfants.
  const sel = await waitFor((m) => m.type === 'select' && m.id, 'select titre');
  const inTitle = await f.evaluate(([id, hid]) => {
    const t = document.querySelector('h1[data-rv="' + CSS.escape(hid) + '"]');
    return t.getAttribute('data-rv') === id || !!t.querySelector('[data-rv="' + CSS.escape(id) + '"]');
  }, [sel.id, h1Id]);
  check('clic sur un titre : sélection transmise', inTitle && sel.rect && sel.rect.width > 0, sel.label);

  const link = frame.locator('footer a[href^="/"]').first();
  const linkId = await link.getAttribute('data-rv');
  await link.click();
  const linkSel = await waitFor((m) => m.type === 'select' && m.id && m.id !== sel.id, 'select lien');
  // Un composant utilisé deux fois (logo de l'en-tête et du pied de page) a les mêmes
  // identifiants dans les deux : l'élément cliqué est cherché dans le lien lui-même.
  const inside = await f.evaluate(([id, lid]) => {
    const l = [...document.querySelectorAll('footer [data-rv="' + CSS.escape(lid) + '"]')][0];
    return !!l && (l.getAttribute('data-rv') === id || !!l.querySelector('[data-rv="' + CSS.escape(id) + '"]'));
  }, [linkSel.id, linkId]);
  await parent.waitForTimeout(300);
  check('clic sur un lien : sélection, pas de navigation', inside && f.url().endsWith(page),
    `${linkSel.label} ; ${f.url()}`);

  await frame.locator('main p').first().hover();
  const hov = await waitFor((m) => m.type === 'hover' && m.id, 'hover');
  check('survol transmis', !!hov.id);

  // Sélection depuis l'interface, puis feuille de style rechargée sans recharger la page.
  const target = await frame.locator('footer [data-rv]').last().getAttribute('data-rv');
  await parent.evaluate((id) => window.send({ type: 'select', id, reveal: true }), target);
  await parent.waitForTimeout(300);
  const scrolled = await f.evaluate(() => window.scrollY);
  check("sélection demandée par l'interface : élément montré", scrolled > 0, `défilement ${scrolled}px`);
  const sheet = await f.evaluate(() => {
    const l = document.querySelector('link[rel~="stylesheet"]');
    return l ? new URL(l.href).pathname : null;
  });
  await parent.evaluate((url) => window.send({ type: 'stylesheet', url }), sheet);
  await parent.waitForTimeout(800);
  const reloaded = await f.evaluate((url) => [...document.querySelectorAll('link[rel~="stylesheet"]')]
    .filter((l) => new URL(l.href).pathname === url).map((l) => new URL(l.href).searchParams.has('rv')), sheet);
  check('feuille de style rechargée sur demande', reloaded.length === 1 && reloaded[0] === true);

  // Échap : désélection.
  await frame.locator('main p').first().click();
  await parent.keyboard.press('Escape');
  await waitFor((m) => m.type === 'select' && m.id === null, 'désélection');
  check('Échap : désélection transmise', true);
} catch (e) {
  check('déroulement', false, e.message);
} finally {
  await browser.close();
  server.close();
}
const failed = results.filter((r) => !r.ok).length;
console.log(`${results.length - failed}/${results.length} essais réussis`);
process.exit(failed ? 1 : 0);
