// Captures de contrôle de l'interface (thèmes, langues, écrans) avec l'API Tauri simulée.
// Usage : node tools/ui-shots.mjs <url du dist servi> <dossier de sortie> [filtre de nom]
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import fs from "node:fs";
const [base, out, only] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const mock = fs.readFileSync(new URL("../apps/revel-ui/dev/mock-tauri.js", import.meta.url), "utf8");
const b = await chromium.launch();
// [nom, paramètres de la simulation, clics successifs ("wait" : attendre la fin d'une action)]
// Les sites s'ouvrent dans l'éditeur (canevas réel : voir editor-e2e.mjs) ; ici, par leurs paramètres.
const site = ".site-card .site-actions .icon-btn >> nth=0";
const view = (v) => `.view-tab[data-view="${v}"]`;
const section = (k) => `.settings-nav-item[data-section="${k}"]`;
const tabBuild = section("settings-export");
const tabPublish = section("settings-publish");
const appSettings = '.view-tab[data-view="nav-settings"]';
const shots = [
  ["home-dark-fr", "theme=dark&lang=fr", []],
  ["home-light-en", "theme=light&lang=en", []],
  ["home-list-dark-fr", "theme=dark&lang=fr", [".home-tools .segmented button >> nth=1"]],
  ["home-empty-dark-fr", "theme=dark&lang=fr&empty=1", []],
  ["new-site-dialog-light-fr", "theme=light&lang=fr", [".page-head .btn--primary"]],
  ["new-site-typed-dark-fr", "theme=dark&lang=fr", [".page-head .btn--primary", "type:Atelier Lumière"]],
  ["site-menu-dark-fr", "theme=dark&lang=fr", [site, ".site-menu-btn"]],
  ["settings-general-dark-fr", "theme=dark&lang=fr", [site]],
  ["settings-general-light-en", "theme=light&lang=en", [site]],
  ["cms-dark-fr", "theme=dark&lang=fr", [site, view("view-cms")]],
  ["cms-entry-light-fr", "theme=light&lang=fr", [site, view("view-cms"), ".cms-table tbody tr >> nth=0"]],
  ["cms-entry-dark-en", "theme=dark&lang=en", [site, view("view-cms"), ".cms-table tbody tr >> nth=2"]],
  ["locales-dark-fr", "theme=dark&lang=fr", [site, view("view-locales")]],
  ["redirects-light-fr", "theme=light&lang=fr", [site, section("settings-redirects")]],
  ["seo-planned-dark-fr", "theme=dark&lang=fr", [site, section("settings-seo")]],
  ["export-dark-fr", "theme=dark&lang=fr", [site, tabBuild]],
  ["export-done-dark-fr", "theme=dark&lang=fr", [site, tabBuild, ".page .btn--primary", "wait"]],
  ["export-done-light-en-accent", "theme=light&lang=en&accent=3D7EFF", [site, tabBuild, ".page .btn--primary", "wait"]],
  ["publish-empty-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish]],
  ["publish-form-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish, ".empty .btn--primary"]],
  ["publish-form-light-en", "theme=light&lang=en&noprofiles=1", [site, tabPublish, ".empty .btn--primary"]],
  ["publish-check-light-fr", "theme=light&lang=fr&warn=1", [site, tabPublish, ".host .toolbar .btn >> nth=0", "wait"]],
  ["publish-done-dark-en", "theme=dark&lang=en", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["publish-edit-dark-fr", "theme=dark&lang=fr", [site, tabPublish, ".host .icon-btn >> nth=0"]],
  ["publish-delete-dark-fr", "theme=dark&lang=fr", [site, tabPublish, ".host .icon-btn >> nth=1"]],
  ["publish-fail-light-fr", "theme=light&lang=fr&fail=auth", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["publish-github-form-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish, ".empty .btn--primary", ".settings-content .segmented button >> nth=1"]],
  ["publish-github-code-light-fr", "theme=light&lang=fr&noprofiles=1&ghwait=1", [site, tabPublish, ".empty .btn--primary", ".settings-content .segmented button >> nth=1", ".subsection .btn"]],
  ["publish-github-connected-light-en", "theme=light&lang=en&noprofiles=1", [site, tabPublish, ".empty .btn--primary", ".settings-content .segmented button >> nth=1", ".subsection .btn", "wait", "wait"]],
  ["publish-github-done-dark-fr", "theme=dark&lang=fr&ghprofile=1&github=matv-studio&warn=1", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["settings-github-error-light-fr", "theme=light&lang=fr&ghfail=1", [appSettings, ".account .btn", "wait"]],
  ["settings-accounts-dark-fr", "theme=dark&lang=fr&github=matv-studio", [appSettings]],
  ["settings-dark-fr-accent", "theme=dark&lang=fr&accent=3D7EFF", [appSettings]],
  ["settings-light-en", "theme=light&lang=en", [appSettings]],
];
for (const [name, hash, clicks] of shots.filter(([n]) => !only || n.includes(only))) {
  const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
  p.on("pageerror", (e) => console.log(name, "pageerror:", e.message));
  await p.addInitScript(mock);
  await p.goto(`${base}#${hash}`, { waitUntil: "networkidle" });
  await p.waitForTimeout(300);
  for (const c of clicks) {
    if (c === "wait") { await p.waitForTimeout(900); continue; }
    if (c.startsWith("type:")) { await p.keyboard.type(c.slice(5)); await p.waitForTimeout(250); continue; }
    await p.locator(c).first().click();
    await p.waitForTimeout(250);
  }
  await p.screenshot({ path: `${out}/${name}.png` });
  await p.close();
}
await b.close();
