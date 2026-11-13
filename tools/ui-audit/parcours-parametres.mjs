import { start, openSite, audit, save, scrollAudit, pop, inventory, escapes, openFloats, OUT } from './lib.mjs';
import fs from 'fs';
const { browser, page, errors } = await start();
await openSite(page);
const T = async (name, fn) => { try { await fn(); } catch (e) { console.log(`ÉTAPE EN ÉCHEC (${name}) : ` + e.message.split('\n')[0].slice(0, 140)); } };
await page.locator('.view-tab', { hasText: 'Paramètres' }).click(); await page.waitForTimeout(1200);
const main = await page.evaluate(() => { const n = document.querySelector('.settings-nav'); const m = n?.parentElement?.querySelector(':scope > :not(.settings-nav)'); return m ? m.className : null; });
console.log('zone principale :', main);
const items = await page.$$eval('.settings-nav-item', (b) => b.map((x) => x.textContent.trim()));
const slug = (s) => s.toLowerCase().normalize('NFD').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
const seen = [];
for (const it of items) {
  await T(it, async () => {
    await page.locator('.settings-nav-item', { hasText: it }).first().click(); await page.waitForTimeout(900);
    const parts = await page.$$eval('.settings-nav-part', (b) => b.map((x) => x.textContent.trim()));
    console.log(it, '→', parts.join(' | '));
    for (const p of parts.length ? parts : [null]) {
      const name = 'param-' + slug(it) + (p ? '-' + slug(p) : '');
      if (p) { await page.locator('.settings-nav-part', { hasText: new RegExp('^' + p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '$') }).first().click(); await page.waitForTimeout(900); }
      // Ouvre tous les details de la zone pour tout contrôler.
      await page.evaluate(() => document.querySelectorAll('.settings-main details:not(.menu-wrap):not(.dropdown), .site-settings details:not(.menu-wrap):not(.dropdown)').forEach((d) => {}));
      await scrollAudit(page, '.settings-nav ~ *', name, { max: 8 });
      const inv = (await inventory(page)).filter((x) => !/view-tab|logo|site-menu-btn|split-|profile-btn|settings-nav|Publier»|Annuler \(|Rétablir \(/.test(x));
      console.log('  ' + name + ' : ' + inv.slice(0, 45).join(' ; ').slice(0, 1100));
      // Menus (summary) de la zone.
      const n = await page.locator('.settings-nav ~ * summary').count();
      for (let i = 0; i < Math.min(n, 6); i++) {
        const s = page.locator('.settings-nav ~ * summary').nth(i);
        if (!(await s.isVisible().catch(() => false))) continue;
        await pop(page, `${name}-menu-${i}`, s);
        await page.evaluate(() => document.querySelectorAll('.settings-nav ~ * details[open]').forEach((d) => { if (d.querySelector(':scope > .menu, :scope > .pop')) d.open = false; }));
      }
    }
  });
}
save(OUT + '/audit.json');
fs.writeFileSync(OUT + '/echap-s6.json', JSON.stringify(escapes));
console.log('erreurs :', errors.length, errors.slice(0, 5).join(' || '));
await browser.close();
