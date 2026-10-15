// Simulation de l'API Tauri pour revoir l'interface dans un navigateur, sans l'application.
// Utilisée par tools/ui-shots.mjs (captures de contrôle). Jamais incluse dans l'application.
(() => {
  const params = new URLSearchParams(location.hash.slice(1));
  const state = {
    settings: { lang: params.get("lang") || "fr", theme: params.get("theme") || "dark", accent: params.get("accent") ? "#" + params.get("accent") : null },
    recents: params.get("empty") ? [] : [
      { path: "C:\\Sites\\alunites-revel-lab", name: "Alunites", opened_at: "2026-10-13T14:02:11+00:00" },
      { path: "C:\\Sites\\atelier-lumen", name: "Atelier Lumen", opened_at: "2026-10-11T09:40:00+00:00" },
      { path: "C:\\Sites\\maison-oka", name: "Maison Oka", opened_at: "2026-10-02T17:12:00+00:00" },
    ],
  };
  const site = {
    path: "C:\\Sites\\alunites-revel-lab", name: "Alunites", base_url: "https://alunites.com",
    pages: 11, layouts: 4, components: 14,
    collections: [
      { id: "projets", folder: "src/content/projets", entries: 40 },
      { id: "actualites", folder: "src/content/actualites", entries: 22 },
      { id: "expertises", folder: "src/content/expertises", entries: 4 },
      { id: "equipe", folder: "src/content/equipe", entries: 18 },
      { id: "offres", folder: "src/content/offres", entries: 10 },
      { id: "sitemap", folder: "", entries: 0 },
    ],
    environments: ["production", "staging"],
    locale: "fr",
    redirects: [["/projets/ancien-film/", "/project/wattr/"], ["/equipe/", "/agence/"]],
  };
  const fail = params.get("fail");
  const installedPacks = new Set(), offPacks = new Set();
  let lastPacks = [];
  let github = params.get("github") || null;
  const ghProfile = [{ id: "h2", name: "GitHub Pages", environment: "production", target: { kind: "github_pages", repo: "matv-studio/alunites-revel-lab", branch: "gh-pages", custom_domain: null } }, true];
  const profiles = params.get("noprofiles") ? [] : params.get("ghprofile") ? [ghProfile] : [[{ id: "h1", name: "Recette OVH", environment: "staging", target: { kind: "sftp", host: "ftp.cluster000.hosting.ovh.net", port: 22, user: "monsite-deploy", remote_dir: "recette-v4k8/", known_host_key: null } }, true]];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  class Channel { constructor() { this.onmessage = null; } }
  // Pont vers un vrai cœur de Revel (`revel canvas-serve --bridge`) : paramètre `bridge=adresse`.
  // Les commandes de l'éditeur y sont transmises ; le reste reste simulé.
  const bridge = params.get("bridge");
  const bridged = ["translate_can", "translate", "translate_packs", "translate_remove", "edit_open", "edit_apply", "edit_seal", "edit_undo", "edit_redo", "edit_pages", "edit_stamp", "canvas_open", "cms_collections", "preview_url", "edit_media", "edit_import_media", "edit_replace_media", "edit_locale_data"];
  // Nom de dossier d'un nouveau site (comme revel_core::site_folder, sans les suffixes).
  const slug = (n) => n.trim().toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "site";
  // Collections de démonstration (sans pont).
  const cms = [
    { id: "projets", label: "Projets", schema: "projets", folder: "src/content/projets", filename: "{fields.slug}.md",
      view: { primary: "title", fields: ["title", "client", "date", "expertises", "published"], sort: "date", descending: true },
      fields: [
        { name: "title", label: "Titre", kind: "string", required: true },
        { name: "slug", label: "Adresse de la page (slug)", kind: "string", required: true, pattern: "^[a-z0-9]+(?:-[a-z0-9]+)*$",
          description: "La page sera publiée sur /project/<slug>/. Ne plus modifier après publication (référencement)." },
        { name: "published", label: "Publié", kind: "boolean", default: true },
        { name: "date", label: "Date", kind: "date", options: { time: true, format: "yyyy-MM-dd'T'HH:mm:ss" } },
        { name: "client", label: "Client", kind: "string" },
        { name: "expertises", label: "Expertises", kind: "reference", options: { collection: "expertises", multiple: true, value: "{fields.slug}", label: "{fields.title}" } },
        { name: "format", label: "Format", kind: "select", options: { values: [{ name: "film", label: "Film" }, { name: "motion", label: "Motion design" }, { name: "photo", label: "Photographie" }] } },
        { name: "cover", label: "Image principale", kind: "image", required: true },
        { name: "hook", label: "Accroche (une ligne par entrée)", kind: "string", list: true },
        { name: "intro", label: "Texte d'introduction", kind: "rich_text", options: { format: "html" } },
        { name: "seo", label: "Référencement (SEO)", kind: "object", fields: [
          { name: "title", label: "Titre SEO", kind: "string", description: "Affiché dans Google." },
          { name: "description", label: "Description SEO", kind: "text" } ] },
        { name: "wp_id", label: "Identifiant WordPress", kind: "number", hidden: true },
      ],
      entries: [
        { file: "wattr", data: { title: "Wattr", slug: "wattr", published: true, date: "2025-03-12T10:00:00", client: "Wattr", expertises: ["film", "ia"], format: "film", cover: "/media/projets/wattr.jpg", hook: ["Un film", "pour Wattr"], intro: "<p>Un film pour <strong>Wattr</strong>.</p>", seo: { title: "Wattr, le film" }, wp_id: 812 }, body: "", url: "/project/wattr/" },
        { file: "lumen", data: { title: "Atelier Lumen", slug: "lumen", published: true, date: "2024-11-02T09:30:00", client: "Lumen", expertises: ["motion"], format: "motion", cover: "/media/projets/lumen.jpg" }, body: "", url: "/project/lumen/" },
        { file: "oka", data: { title: "Maison Oka", slug: "oka", published: false, date: "2024-06-20T14:00:00", client: "Oka", expertises: [], cover: "/media/projets/oka.jpg" }, body: "", url: null },
      ] },
    { id: "expertises", label: "Expertises", schema: "expertises", folder: "src/content/expertises",
      fields: [{ name: "title", label: "Titre", kind: "string", required: true }, { name: "slug", kind: "string" }],
      entries: [
        { file: "film", data: { title: "Film et vidéo", slug: "film" }, body: "", url: "/expertises/film/" },
        { file: "motion", data: { title: "Motion design", slug: "motion" }, body: "", url: "/expertises/motion/" },
        { file: "ia", data: { title: "Création par IA", slug: "ia" }, body: "", url: "/expertises/ia/" },
      ] },
    { id: "actualites", folder: "src/content/actualites", fields: [], entries: [
        { file: "salon-2026", data: { title: "Salon 2026", date: "2026-02-01", tags: ["événement"] }, body: "Le salon.\n", url: "/actualites/salon-2026/" } ] },
  ];
  async function invoke(cmd, args) {
    // Langue de traduction par le pont : remise en service d'une langue livrée (sans
    // téléchargement) ; téléchargement réel seulement avec `dl=1` (600 Mo), sinon simulé.
    const realInstall = cmd.startsWith("translate_install")
      && (params.get("dl") || lastPacks.some((p) => p.code === args?.code && p.state === "bundled_off"));
    if (bridge && (bridged.includes(cmd) || realInstall)) {
      // Corps en texte brut : requête simple, sans demande préalable (CORS).
      const r = await fetch(bridge.replace(/\/$/, "") + "/__revel/invoke/" + cmd, {
        method: "POST", headers: { "content-type": "text/plain" }, body: JSON.stringify(args || {}),
      });
      const v = await r.json();
      if (!r.ok) throw v;
      if (cmd === "translate_packs") lastPacks = v;
      return v;
    }
    switch (cmd) {
      case "get_settings": return state.settings;
      case "set_settings": state.settings = args.settings; return null;
      case "recent_sites": return state.recents;
      case "open_site": return site;
      case "default_sites_dir": return "C:\\Users\\vous\\Documents\\Revel";
      case "site_folder": return args.parent + "\\" + slug(args.name || "");
      case "create_site": return { ...site, name: args.name, path: args.parent + "\\" + slug(args.name), pages: 0, layouts: 0, components: 0, collections: [], redirects: [] };
      case "cms_collections": return cms;
      // Sans pont : document minimal (vue CMS, captures) ; modifications non simulées.
      case "edit_open": return { site: { schema_version: 3, name: site.name, default_locale: "fr" }, state: { revision: 0, can_undo: false, can_redo: false } };
      case "has_site": return true;
      case "list_profiles": return profiles;
      case "build_site": {
        const ch = args.onEvent;
        ch.onmessage({ event: "progress", phase: "pages", done: 0, total: 1 });
        for (let i = 0; i <= 3843; i += 640) { await wait(60); ch.onmessage({ event: "progress", phase: "images", done: i, total: 3843 }); }
        ch.onmessage({ event: "finished", environment: args.environment, out_dir: "", generated: 70, copied: 443, images_produced: 12, images_cached: 3831, images_failed: 0, seconds: 3.4, warnings: [] });
        return null;
      }
      case "open_preview": return "revel-preview://localhost/" + (args.page || "/").replace(/^\//, "");
      // Sans pont : page de démonstration.
      case "preview_url": return "data:text/html;charset=utf-8," + encodeURIComponent('<!doctype html><title>Aperçu</title><body style="margin:0;font:16px system-ui;display:grid;place-items:center;height:100vh;background:#f4f1ea;color:#222"><h1>Alunites</h1></body>') + "#";
      case "reveal_path": return null;
      case "edit_media": return [];
      case "edit_locale_data": return [];
      case "export_site": {
        const ch = args.onEvent;
        for (const [phase, done, total] of [["pages", 1, 1], ["copies", 1, 1], ["images", 2, 4], ["images", 4, 4]]) { await wait(120); ch.onmessage({ event: "progress", phase, done, total }); }
        const out = args.parent + "\\alunites-" + args.environment;
        ch.onmessage({ event: "finished", environment: args.environment, out_dir: out, generated: 71, copied: 442, images_produced: 12, images_cached: 380, images_failed: 0, seconds: 4.2, warnings: [] });
        return out;
      }
      // Langues de traduction (sans pont) : français livré, allemand téléchargé.
      case "translate_packs": return ["fr", "de", "es", "it", "nl", "sv", "zh"].map((code, i) => ({
        code, state: i === 0 ? (offPacks.has(code) ? "bundled_off" : "bundled") : (i === 1 || installedPacks.has(code)) ? "installed" : "available", download: 597_000_000, disk: 298_000_000 }));
      case "translate_install": {
        if (args.code === "fr" && offPacks.delete("fr")) return null;
        const ch = args.onEvent;
        for (let d = 0; d <= 100; d += 10) { await wait(params.get("dlslow") ? 900 : 60); ch.onmessage({ event: "progress", done: d * 5_970_000, total: 597_000_000 }); }
        ch.onmessage({ event: "preparing" }); await wait(300);
        installedPacks.add(args.code);
        return null;
      }
      case "translate_install_cancel": return null;
      case "translate_remove": if (args.code === "fr") offPacks.add("fr"); installedPacks.delete(args.code); return null;
      case "translate_can": return false;
      case "github_account": return github;
      case "github_disconnect": github = null; return null;
      case "github_cancel": return null;
      case "open_external": return null;
      case "github_connect": {
        const ch = args.onEvent;
        if (params.get("ghfail")) { const f = { key: "error-network", args: [["detail", "dns error"]] }; ch.onmessage({ event: "failed", ...f }); throw f; }
        ch.onmessage({ event: "code", user_code: "F014-2AC6", verification_uri: "https://github.com/login/device", expires_in: 899 });
        if (params.get("ghwait")) { await wait(60000); }
        await wait(1200);
        github = "matv-studio";
        ch.onmessage({ event: "connected", login: github });
        return github;
      }
      case "deploy": {
        const ch = args.onEvent;
        ch.onmessage({ event: "building", build: { event: "progress", phase: "pages", done: 1, total: 1 } });
        ch.onmessage({ event: "warnings", warnings: [{ blocking: false, key: "deploy-warn-too-many-files", args: [["count", "24512"], ["max", "20000"]] }].filter(() => params.get("warn")) });
        await wait(80);
        if (fail) { const f = { key: "error-sftp-" + fail, args: [["host", "ftp.cluster000.hosting.ovh.net"], ["detail", "connection refused"]] }; ch.onmessage({ event: "failed", ...f }); throw f; }
        ch.onmessage({ event: "connected", server_key: "x" });
        ch.onmessage({ event: "planned", upload: 14, delete: 2, bytes: 3_400_000, total: 4537 });
        if (args.dryRun) { ch.onmessage({ event: "finished", uploaded: 14, deleted: 2, total_files: 4537, bytes: 3_400_000, dry_run: true, server_key: "x", server_fingerprint: "SHA256:/bPN11w+yMb4D7FWpnTAuPkibmBXY17S0Uw/rDzIJv8", key_recorded: true }); return null; }
        if (params.get("ghprofile")) {
          for (let i = 0; i <= 4356; i += 1089) { await wait(30); ch.onmessage({ event: "preparing", done: i, total: 4356 }); }
          ch.onmessage({ event: "finished", uploaded: 4356, deleted: 0, total_files: 4358, bytes: 751_000_000, dry_run: false, server_key: "", server_fingerprint: "", key_recorded: false, url: "https://matv-studio.github.io/alunites-revel-lab/", pages_enabled: true });
          return null;
        }
        for (let i = 1; i <= 14; i++) { await wait(40); ch.onmessage({ event: "uploaded", done: i, total: 14 }); }
        ch.onmessage({ event: "finished", uploaded: 14, deleted: 2, total_files: 4537, bytes: 3_400_000, dry_run: !!args.dryRun, server_key: "x ssh-ed25519 AAAA", server_fingerprint: "SHA256:/bPN11w+yMb4D7FWpnTAuPkibmBXY17S0Uw/rDzIJv8", key_recorded: true });
        return null;
      }
      default: return null;
    }
  }
  window.__TAURI__ = { core: { invoke, Channel }, __mock: true };
})();
