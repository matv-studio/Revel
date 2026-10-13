// Captures de contrôle de l'interface (thèmes, langues, écrans) avec l'API Tauri simulée.
// Usage : node tools/ui-shots.mjs <url du dist servi> <dossier de sortie>
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import fs from "node:fs";
const [base, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const mock = fs.readFileSync(new URL("../apps/revel-ui/dev/mock-tauri.js", import.meta.url), "utf8");
const b = await chromium.launch();
// [nom, paramètres de la simulation, clics successifs ("wait" : attendre la fin d'une action)]
const site = "tr.row-link";
const tabBuild = ".nav-item--sub >> nth=0";
const tabPublish = ".nav-item--sub >> nth=1";
const shots = [
  ["home-dark-fr", "theme=dark&lang=fr", []],
  ["home-light-en", "theme=light&lang=en", []],
  ["home-empty-dark-fr", "theme=dark&lang=fr&empty=1", []],
  ["new-site-dialog-light-fr", "theme=light&lang=fr", [".page-head .btn >> nth=2"]],
  ["overview-dark-fr", "theme=dark&lang=fr", [site]],
  ["overview-light-en", "theme=light&lang=en", [site]],
  ["build-dark-fr", "theme=dark&lang=fr", [site, tabBuild, ".page .btn--primary", "wait"]],
  ["build-light-en-accent", "theme=light&lang=en&accent=3D7EFF", [site, tabBuild, ".page .btn--primary", "wait"]],
  ["preview-error-dark-fr", "theme=dark&lang=fr", [site, ".topbar .btn >> nth=0"]],
  ["publish-empty-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish]],
  ["publish-form-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish, ".empty .btn--primary"]],
  ["publish-form-light-en", "theme=light&lang=en&noprofiles=1", [site, tabPublish, ".empty .btn--primary"]],
  ["publish-check-light-fr", "theme=light&lang=fr&warn=1", [site, tabPublish, ".host .toolbar .btn >> nth=0", "wait"]],
  ["publish-done-dark-en", "theme=dark&lang=en", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["publish-edit-dark-fr", "theme=dark&lang=fr", [site, tabPublish, ".host .icon-btn >> nth=0"]],
  ["publish-delete-dark-fr", "theme=dark&lang=fr", [site, tabPublish, ".host .icon-btn >> nth=1"]],
  ["publish-fail-light-fr", "theme=light&lang=fr&fail=auth", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["publish-github-form-dark-fr", "theme=dark&lang=fr&noprofiles=1", [site, tabPublish, ".empty .btn--primary", ".segmented button >> nth=1"]],
  ["publish-github-code-light-fr", "theme=light&lang=fr&noprofiles=1&ghwait=1", [site, tabPublish, ".empty .btn--primary", ".segmented button >> nth=1", ".subsection .btn"]],
  ["publish-github-connected-light-en", "theme=light&lang=en&noprofiles=1", [site, tabPublish, ".empty .btn--primary", ".segmented button >> nth=1", ".subsection .btn", "wait", "wait"]],
  ["publish-github-done-dark-fr", "theme=dark&lang=fr&ghprofile=1&github=matv-studio&warn=1", [site, tabPublish, ".host .btn--primary", "wait"]],
  ["settings-github-error-light-fr", "theme=light&lang=fr&ghfail=1", [".sidebar-bottom .nav-item", ".account .btn", "wait"]],
  ["settings-accounts-dark-fr", "theme=dark&lang=fr&github=matv-studio", [".sidebar-bottom .nav-item"]],
  ["settings-dark-fr-accent", "theme=dark&lang=fr&accent=3D7EFF", [".sidebar-bottom .nav-item"]],
  ["settings-light-en", "theme=light&lang=en", [".sidebar-bottom .nav-item"]],
];
for (const [name, hash, clicks] of shots) {
  const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
  p.on("pageerror", (e) => console.log(name, "pageerror:", e.message));
  await p.addInitScript(mock);
  await p.goto(`${base}#${hash}`, { waitUntil: "networkidle" });
  await p.waitForTimeout(300);
  for (const c of clicks) {
    if (c === "wait") { await p.waitForTimeout(900); continue; }
    await p.locator(c).first().click();
    await p.waitForTimeout(250);
  }
  await p.screenshot({ path: `${out}/${name}.png` });
  await p.close();
}
await b.close();
