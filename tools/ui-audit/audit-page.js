// Contrôles exécutés dans la page de l'interface : renvoie les écarts aux règles de la charte.
(() => {
  const SIZES = new Set([11, 12, 13, 15, 20]);
  const vis = (e) => {
    if (!e.isConnected) return false;
    const r = e.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return false;
    if (e.checkVisibility && !e.checkVisibility({ contentVisibilityAuto: true, checkOpacity: true, checkVisibilityCSS: true })) return false;
    if (r.bottom < 0 || r.right < 0 || r.top > innerHeight || r.left > innerWidth) return false;
    for (let p = e; p && p !== document.documentElement; p = p.parentElement) {
      const cs = getComputedStyle(p);
      if (cs.display === 'none' || cs.visibility === 'hidden' || +cs.opacity === 0) return false;
    }
    return true;
  };
  const sel = (e) => {
    const one = (x) => x.tagName.toLowerCase() + (x.id ? '#' + x.id : '') + [...x.classList].slice(0, 3).map((c) => '.' + c).join('');
    const out = [one(e)];
    let p = e.parentElement, n = 0;
    while (p && n < 3 && p !== document.body) { if (p.classList.length || p.id) { out.unshift(one(p)); n++; } p = p.parentElement; }
    return out.join(' > ');
  };
  const own = (e) => [...e.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent).join('').replace(/\s+/g, ' ').trim();
  const parse = (c) => { const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const canvas = document.createElement('canvas'); canvas.width = canvas.height = 1; const ctx = canvas.getContext('2d', { willReadFrequently: true });
  const rgba = (c) => { ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = '#000'; ctx.fillStyle = c; ctx.fillRect(0, 0, 1, 1); const d = ctx.getImageData(0, 0, 1, 1).data; return { r: d[0], g: d[1], b: d[2], a: d[3] / 255 }; };
  const over = (top, bot) => ({ r: top.r * top.a + bot.r * (1 - top.a), g: top.g * top.a + bot.g * (1 - top.a), b: top.b * top.a + bot.b * (1 - top.a), a: 1 });
  const bgOf = (e) => {
    const stack = [];
    for (let p = e; p; p = p.parentElement) {
      const cs = getComputedStyle(p);
      if (cs.backgroundImage !== 'none' && !/gradient\(.*transparent/.test(cs.backgroundImage) && p !== e) return null; // image ou dégradé : non mesuré
      const c = rgba(cs.backgroundColor);
      if (c.a > 0) { stack.push(c); if (c.a >= 0.999) break; }
    }
    let base = rgba(getComputedStyle(document.documentElement).getPropertyValue('--bg') || '#191919');
    for (let i = stack.length - 1; i >= 0; i--) base = over(stack[i], base);
    return base;
  };
  const lum = (c) => { const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); }; return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b); };
  const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
  const disabled = (e) => { for (let p = e; p; p = p.parentElement) { if (p.disabled || p.getAttribute('aria-disabled') === 'true' || p.classList.contains('is-planned') || p.classList.contains('is-disabled') || +getComputedStyle(p).opacity < 0.6) return true; } return false; };
  const titled = (e) => { for (let p = e, n = 0; p && n < 3; p = p.parentElement, n++) if (p.getAttribute('title')) return true; return false; };
  const F = [];
  const add = (rule, e, detail) => F.push({ rule, sel: sel(e), text: (own(e) || e.getAttribute('aria-label') || e.getAttribute('title') || '').slice(0, 60), detail: detail || '' });
  const all = [...document.body.querySelectorAll('*')].filter((e) => !e.closest('svg') || e.tagName === 'svg').filter((e) => !/^(SCRIPT|STYLE|IFRAME|OPTION|CANVAS)$/.test(e.tagName)).filter(vis);
  const userContent = (e) => !!e.closest('.rt-body, .canvas-frame, [contenteditable], .cms-table tbody, .table tbody, .layer-row, .tree-row, .asset, .serp, .diff, .help-body, .code, textarea, input, .tr-row, .tr-source, .tr-cell, .site-card, .var-value, .map-card, .entry-title, .page-row, .comp-row, .preview');
  for (const e of all) {
    const cs = getComputedStyle(e);
    const tag = e.tagName;
    const t = own(e);
    const clickable = tag === 'BUTTON' || tag === 'A' || e.getAttribute('role') === 'button' || e.getAttribute('role') === 'tab' || e.getAttribute('role') === 'radio' || e.getAttribute('role') === 'menuitem';
    if (clickable) {
      const label = (e.textContent || '').replace(/\s+/g, ' ').trim();
      if (!label) {
        if (!e.getAttribute('aria-label') && !e.getAttribute('aria-labelledby')) add('bouton-icône sans aria-label', e);
        if (!e.getAttribute('title') && !disabled(e)) add('bouton-icône sans info-bulle', e, e.getAttribute('aria-label') || '');
      }
      if (cs.cursor === 'pointer' && tag === 'BUTTON') add('curseur main sur un bouton', e);
      const r = e.getBoundingClientRect();
      if ((r.width < 16 || r.height < 16) && !e.closest('.cp, .range, .switch')) add('cible de clic < 16 px', e, Math.round(r.width) + '×' + Math.round(r.height));
    }
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') {
      const named = e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') || e.getAttribute('title') || e.closest('label') || (e.id && document.querySelector(`label[for="${e.id}"]`)) || e.getAttribute('placeholder');
      if (!named && e.type !== 'hidden') add('champ sans nom accessible', e, e.type || tag);
    }
    if (tag === 'IMG' && !e.hasAttribute('alt') && !userContent(e)) add('image sans alt', e);
    if (/^(BUTTON|INPUT|SELECT)$/.test(tag) && !/^(checkbox|radio|range|color|file|hidden)$/.test(e.type || '')) {
      const h = Math.round(e.getBoundingClientRect().height * 10) / 10;
      if (![16, 20, 22, 24, 28, 32].includes(h) && !e.closest('.cp, .switch, .tree-row, .layer-row') && !e.matches('.insert-item, .anim-tile, .view-tab, .template-card, .template-choice, .list-row, .site-card, .swatch, .asset, .menu-item, .settings-nav-item, .dock-tab') && !(tag === 'INPUT' && e.parentElement && /panel-search|search/.test(e.parentElement.className))) add('hauteur de contrôle hors grille', e, h + ' px');
    }
    if (t) {
      const fs = Math.round(parseFloat(cs.fontSize) * 10) / 10;
      if (!SIZES.has(fs) && !userContent(e)) add('taille de texte hors échelle', e, fs + ' px');
      if (!/^("?Geist"?|ui-monospace)/.test(cs.fontFamily) && !userContent(e)) add('police hors charte', e, cs.fontFamily.slice(0, 40));
      if (!userContent(e)) {
        if (cs.textTransform === 'uppercase') add('capitales', e);
        else if (/[A-ZÀ-Ý]{4,}/.test(t) && !/[a-zà-ÿ]/.test(t) && !/^(HTML|CSS|URL|JSON|SFTP|HTTPS?|RSS|SEO|CMS|PHP|RVB|TSL|TSV|HEX|AAA?|WCAG|GLB|GLTF|MCP|API|SVG|PNG|JPE?G|WEBP|AVIF|PDF|CSV|XML|ZIP|DNS|CNAME|AAAA|UTC|EN|FR)([ ·,/:0-9a-z-]|$)+/i.test(t)) add('libellé en capitales', e);
        const words = t.split(' ').length;
        if ((words >= 8 || (/[.!?…]$/.test(t) && words >= 5)) && !e.closest('.notice--error, .tip, [role=tooltip], .toast, .status--ok')) add('phrase visible', e, words + ' mots');
        if (/^[a-z]+(-[a-z0-9]+){1,5}$/.test(t) && !/-/.test(e.getAttribute('data-plain') || '') && t.length > 8) add('clé de traduction visible ?', e);
      }
      if (e.scrollWidth > e.clientWidth + 1 && cs.overflowX !== 'visible' && !titled(e)) add(userContent(e) ? 'texte tronqué sans info-bulle (contenu)' : 'libellé tronqué sans info-bulle', e, e.scrollWidth + ' > ' + e.clientWidth);
      if (!disabled(e) && cs.color !== 'rgba(0, 0, 0, 0)' && !userContent(e)) {
        const bg = bgOf(e);
        if (bg) {
          let fg = rgba(cs.color); if (fg.a < 1) fg = over(fg, bg);
          const cr = ratio(fg, bg);
          const large = parseFloat(cs.fontSize) >= 24 || (parseFloat(cs.fontSize) >= 18.66 && +cs.fontWeight >= 700);
          if (cr < (large ? 3 : 4.5) - 0.02) add('contraste insuffisant (AA)', e, cr.toFixed(2) + ' : ' + cs.color + ' sur rgb(' + [bg.r, bg.g, bg.b].map(Math.round).join(',') + ')');
        }
      }
    }
    if (/(auto|scroll)/.test(cs.overflowX) && e.scrollWidth > e.clientWidth + 1 && e.clientWidth > 40 && !e.closest('.canvas-wrap, .stage, .map, .diff, .cms-table-wrap, .rt-body, .dock-tabs, textarea, .views, .bp-list')) add('défilement horizontal', e, e.scrollWidth + ' > ' + e.clientWidth);
    if (e.classList.contains('segmented') && e.getAttribute('role') !== 'radiogroup' && e.getAttribute('role') !== 'tablist') add('choix exclusif sans rôle', e);
  }
  // Une seule action principale par zone.
  const zones = new Map();
  for (const b of all.filter((e) => e.classList.contains('btn--primary'))) {
    const z = b.closest('.dialog, .pop, .menu, .section, .subsection, .toolbar, .page-head, .panel-foot, .topbar, .cms-head, .dock-panel, .settings-content, .card') || document.body;
    zones.set(z, (zones.get(z) || []).concat(b));
  }
  for (const [z, bs] of zones) if (bs.length > 1) add('plusieurs actions principales dans une zone', z, bs.map((b) => (b.textContent || '').trim()).join(' | '));
  // Répartitions (pour relecture).
  const dist = { radius: {}, duration: {}, sizes: {} };
  for (const e of all) {
    const cs = getComputedStyle(e);
    const br = cs.borderTopLeftRadius; if (br !== '0px') dist.radius[br] = (dist.radius[br] || 0) + 1;
    for (const d of cs.transitionDuration.split(', ')) if (d !== '0s') dist.duration[d] = (dist.duration[d] || 0) + 1;
  }
  return { findings: F, dist, count: all.length };
})()
