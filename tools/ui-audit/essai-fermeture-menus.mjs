import { start, openSite, frameOf, openFloats } from './lib.mjs';
const { browser, page, errors } = await start();
await openSite(page);
const f = frameOf(page);
const state = async () => (await openFloats(page)).join(',') || '—';
const cases = [
  ['options de l’aperçu', async () => page.click('.split-more')],
  ['menu du site', async () => page.click('.site-menu-btn')],
  ['profil', async () => page.click('.profile-btn')],
  ['options du panneau', async () => page.locator('.dock-head .icon-btn').first().click()],
  ['menu contextuel du canevas', async () => f.click('section.hero h1', { button: 'right', position: { x: 5, y: 5 } })],
  ['menu contextuel d’un calque', async () => page.locator('.layer').nth(3).click({ button: 'right' })],
  ['menu contextuel d’une page', async () => page.locator('.page-row').nth(1).click({ button: 'right' })],
  ['menu contextuel d’un onglet', async () => page.click('.dock-tab[data-panel="layers"]', { button: 'right' })],
  ['affichage (barre d’outils)', async () => page.locator('.editor-toolbar summary').nth(1).click()],
  ['panneaux (barre d’outils)', async () => page.locator('.editor-toolbar summary').nth(2).click()],
  ['points de rupture', async () => page.locator('.editor-toolbar summary').nth(0).click()],
];
for (const [name, open] of cases) {
  const res = [];
  for (const how of ['Échap', 'clic sur le canevas', 'clic dans un panneau']) {
    try { await open(); } catch (e) { res.push(how + ' : ouverture impossible'); continue; }
    await page.waitForTimeout(350);
    const before = await state();
    if (how === 'Échap') await page.keyboard.press('Escape');
    else if (how === 'clic sur le canevas') await page.mouse.click(700, 600);
    else await page.mouse.click(140, 860);
    await page.waitForTimeout(350);
    const after = await state();
    const detailsOpen = await page.evaluate(() => [...document.querySelectorAll('.editor-toolbar details[open], .topbar details[open]')].length);
    res.push(`${how} : ${before} → ${after}${detailsOpen ? ' (details ouvert : ' + detailsOpen + ')' : ''}`);
    await page.keyboard.press('Escape'); await page.mouse.click(140, 860); await page.waitForTimeout(200);
    await page.evaluate(() => document.querySelectorAll('.editor-toolbar details[open]').forEach((d) => { d.open = false; }));
  }
  console.log(name + '\n   ' + res.join('\n   '));
}
await browser.close();
