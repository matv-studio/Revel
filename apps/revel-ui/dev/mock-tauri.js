// Simulation de l'API Tauri pour revoir l'interface dans un navigateur, sans l'application.
// Utilisée par tools/ui-shots.mjs (captures de contrôle). Jamais incluse dans l'application.
(() => {
  const params = new URLSearchParams(location.hash.slice(1));
  const state = {
    settings: { lang: params.get("lang") || "fr", theme: params.get("theme") || "dark", accent: params.get("accent") ? "#" + params.get("accent") : null },
    // `update=1` : une version plus récente est proposée (Réglages › Mises à jour).
    update: params.get("update") ? { enabled: true, available: { version: "0.5.0", notes: "Animations Lottie, actions, transitions entre les pages." } } : null,
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
  const recette = [{ id: "h1", name: "Recette OVH", environment: "staging", target: { kind: "sftp", host: "ftp.cluster000.hosting.ovh.net", port: 22, user: "monsite-deploy", remote_dir: "recette-v4k8/", known_host_key: null } }, true];
  // `hosts2=1` : site public et version de test ; `nosecret=1` : mot de passe du site public absent.
  const publicHost = [{ id: "h0", name: "OVHcloud", environment: "production", site_url: "https://alunites.com", preset: "ovh", target: { kind: "sftp", host: "ftp.cluster031.hosting.ovh.net", port: 22, user: "alunitp", remote_dir: "www/", known_host_key: "x" } }, !params.get("nosecret")];
  const profiles = params.get("noprofiles") ? [] : params.get("ghprofile") ? [ghProfile] : params.get("hosts2") ? [publicHost, recette] : [recette];
  // Publications (5.1) : site public publié il y a 3 h, modifié depuis ; version de test jamais
  // publiée (`pubnever=1` : aucune publication).
  const published = params.get("pubnever") ? {} : { h0: Date.now() - 3 * 3600_000 - 240_000, h2: Date.now() - 26 * 3600_000 };
  const pendingChanges = params.get("uptodate") ? {} : {
    h0: [{ url: "/", title: "Accueil", kind: "changed" }, { url: "/offres/tournage/", title: "Tournage", kind: "added" },
      { url: "/agence/", title: "Agence", kind: "changed" }, { url: "/projets/ancien-film/", title: "Ancien film", kind: "removed" },
      { url: "", title: "", kind: "files" }],
    h2: [{ url: "", title: "", kind: "files" }],
  };
  // Publications précédentes gardées (5.5) : site public, la veille.
  const previous = params.get("pubnever") ? {} : { h0: Date.now() - 27 * 3600_000 };
  const publishState = () => profiles.map(([p, ready]) => ({
    profile: p.id, name: p.name, environment: p.environment, ready,
    url: p.site_url || (p.target.kind === "github_pages" ? "https://matv-studio.github.io/alunites-revel-lab/" : p.environment === "production" ? site.base_url : null),
    published_at: published[p.id] ?? null, previous_at: previous[p.id] ?? null, changes: published[p.id] ? (pendingChanges[p.id] || []) : [],
  }));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const mcp = { enabled: false, port: 47520, token: "3f9a1c0e7b2d4a6f8e1c3b5d7f9a0c2e4b6d8f1a3c5e7b9d0f2a4c6e8b1d3f5a" };
  const mcpStatus = () => {
    const busy = mcp.enabled && params.get("mcpbusy");
    return { ...mcp, url: `http://127.0.0.1:${mcp.port}/mcp`, running: mcp.enabled && !busy,
      error: busy ? "Only one usage of each socket address is normally permitted. (os error 10048)" : null,
      program: "C:\\Program Files\\Revel\\revel.exe" };
  };
  class Channel { constructor() { this.onmessage = null; } }
  // Pont vers un vrai cœur de Revel (`revel canvas-serve --bridge`) : paramètre `bridge=adresse`.
  // Les commandes de l'éditeur y sont transmises ; le reste reste simulé.
  const bridge = params.get("bridge");
  const bridged = ["readable_data", "versions_status", "versions_init", "versions_list", "versions_save", "versions_changes", "versions_diff", "versions_restore", "versions_sync", "versions_resolve", "versions_set_remote", "fonts", "fonts_google", "fonts_add_google", "fonts_add_files", "fonts_remove", "translate_can", "translate", "translate_packs", "translate_remove", "edit_open", "edit_apply", "edit_seal", "edit_undo", "edit_redo", "edit_pages", "edit_stamp", "canvas_open", "cms_collections", "cms_folders", "preview_url", "edit_media", "edit_import_media", "edit_replace_media", "edit_locale_data"];
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
      // Serveur des assistants (simulé) : `mcpbusy=1` : port occupé ; `mcpacts=1` : actions.
      case "mcp_status": return mcpStatus();
      case "mcp_set": mcp.enabled = args.enabled; mcp.port = args.port || 47520; return mcpStatus();
      case "mcp_new_token": mcp.token = [...crypto.getRandomValues(new Uint8Array(32))].map((b) => b.toString(16).padStart(2, "0")).join(""); return mcpStatus();
      case "mcp_activity": if (window.__revelMcpActs) return window.__revelMcpActs;
        return mcp.enabled && params.get("mcpacts") ? [
        { at: Date.now() - 60_000, client: "claude-ai", tool: "set_text", target: "xg2whxr8" },
        { at: Date.now() - 240_000, client: "claude-ai", tool: "set_styles", target: ".hero__title" },
        { at: Date.now() - 900_000, client: "codex-mcp-client", tool: "set_data", target: "pages.accueil.news_title" },
        { at: Date.now() - 86_400_000 * 2, client: "", tool: "undo" },
      ] : [];
      case "get_settings": return state.settings;
      case "set_settings": state.settings = args.settings; return null;
      case "recent_sites": return state.recents;
      case "open_site": return site;
      case "default_sites_dir": return state.settings.projects_dir || "C:\\Program Files\\Revel\\Projects";
      case "site_folder": return args.parent + "\\" + slug(args.name || "");
      case "create_site": return { ...site, name: args.name, path: args.parent + "\\" + slug(args.name), pages: 0, layouts: 0, components: 0, collections: [], redirects: [] };
      case "cms_collections": return cms;
      case "cms_folders": return [];
      case "app_info": return ["0.4.2", 3];
      case "update_check": return state.update || { enabled: false };
      case "update_install": return null;
      // Sans pont : document minimal (vue CMS, captures) ; modifications non simulées.
      case "edit_open": return { site: { schema_version: 3, name: site.name, default_locale: "fr" }, state: { revision: 0, can_undo: false, can_redo: false } };
      case "has_site": return true;
      case "list_profiles": return profiles;
      case "publish_state": return publishState();
      // Nom de domaine (5.7) : `domainko=1` : HTTPS absent, autre page.
      case "domain_check": {
        await wait(400);
        const gh = profiles.find(([p]) => p.id === args.profileId)?.[0].target.kind === "github_pages";
        const ko = params.get("domainko");
        return { domain: args.domain, addresses: gh ? ["185.199.108.153", "185.199.109.153"] : ["213.186.33.5"],
          expected: gh ? ["185.199.108.153", "185.199.109.153", "185.199.110.153", "185.199.111.153"] : [],
          https: !ko, http: true, serves_site: !ko, https_error: ko ? "certificate not valid for name" : null };
      }
      // Retour à la publication précédente (5.5) : la précédente devient la publiée.
      case "rollback": {
        const ch = args.onEvent;
        ch.onmessage({ event: "connected", server_key: "x" });
        ch.onmessage({ event: "planned", upload: 3, delete: 1, bytes: 120_000, total: 4537 });
        for (let i = 1; i <= 3; i++) { await wait(params.get("pubslow") ? 700 : 60); ch.onmessage({ event: "uploaded", done: i, total: 3 }); }
        const prev = previous[args.profileId];
        previous[args.profileId] = published[args.profileId]; published[args.profileId] = prev;
        pendingChanges[args.profileId] = [{ url: "/", title: "Accueil", kind: "changed" }];
        ch.onmessage({ event: "finished", uploaded: 3, deleted: 1, total_files: 4537, bytes: 120_000, dry_run: false, server_key: "", server_fingerprint: "", key_recorded: false, url: profiles.find(([p]) => p.id === args.profileId)?.[0].site_url });
        return null;
      }
      // Connexion d'essai (5.3) : `probefail=auth|timeout` : échec ; dossiers selon l'hébergeur.
      case "hosting_probe": {
        await wait(params.get("pubslow") ? 2500 : 500);
        const t = args.profile.target;
        const pf = params.get("probefail");
        if (pf) throw pf === "timeout" ? { key: "error-sftp-timeout", args: [["host", `${t.host}:${t.port}`]] } : { key: "error-sftp-auth", args: [] };
        const folders = /o2switch/.test(t.host) ? ["public_html/", "www/"]
          : /^\d/.test(t.host) ? ["domains/alunites.com/public_html/", "public_html/"]
          : /infomaniak/.test(t.host) ? ["sites/alunites.com/"] : ["www/"];
        return { server_key: `${t.host} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOx`, fingerprint: "SHA256:/bPN11w+yMb4D7FWpnTAuPkibmBXY17S0Uw/rDzIJv8", folders, suggested: folders[0] };
      }
      case "save_profile": {
        const p = { ...args.profile, id: args.profile.id || "h" + Date.now() };
        const i = profiles.findIndex(([q]) => q.id === p.id);
        if (i >= 0) profiles[i] = [p, true]; else profiles.push([p, true]);
        return p;
      }
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
        // Contrôles (5.4) : `checks=1` : points à vérifier ; publication arrêtée sauf « quand même ».
        if (params.get("checks") && !args.skipChecks) {
          ch.onmessage({ event: "checks", checks: [
            { kind: "broken_link", page: "/agence/", target: "/equipe/ancienne-page/", size: 0 },
            { kind: "broken_link", page: "/", target: "/offres/tournage-drone/", size: 0 },
            { kind: "heavy_image", page: "/project/wattr/", target: "/media/projets/wattr-cover.png", size: 3_480_000 },
            { kind: "missing_alt", page: "/", target: "/media/accueil/equipe.jpg", size: 0 },
            { kind: "missing_alt", page: "/agence/", target: "/media/agence/bureau.jpg", size: 0 },
            { kind: "missing_alt", page: "/agence/", target: "/media/agence/plateau.jpg", size: 0 },
            { kind: "no_title", page: "/mentions-legales/", target: "", size: 0 },
          ] });
          if (!args.dryRun) { const f = { key: "error-publish-checks", args: [["count", "7"]] }; ch.onmessage({ event: "failed", ...f }); throw f; }
        }
        if (fail && (!params.get("failon") || params.get("failon") === args.profileId)) { const f = { key: "error-sftp-" + fail, args: [["host", "ftp.cluster000.hosting.ovh.net"], ["detail", "connection refused"]] }; ch.onmessage({ event: "failed", ...f }); throw f; }
        ch.onmessage({ event: "connected", server_key: "x" });
        ch.onmessage({ event: "planned", upload: 14, delete: 2, bytes: 3_400_000, total: 4537 });
        if (args.dryRun) { ch.onmessage({ event: "finished", uploaded: 14, deleted: 2, total_files: 4537, bytes: 3_400_000, dry_run: true, server_key: "x", server_fingerprint: "SHA256:/bPN11w+yMb4D7FWpnTAuPkibmBXY17S0Uw/rDzIJv8", key_recorded: true }); return null; }
        if (params.get("ghprofile")) {
          for (let i = 0; i <= 4356; i += 1089) { await wait(30); ch.onmessage({ event: "preparing", done: i, total: 4356 }); }
          published[args.profileId] = Date.now(); delete pendingChanges[args.profileId];
          ch.onmessage({ event: "finished", uploaded: 4356, deleted: 0, total_files: 4358, bytes: 751_000_000, dry_run: false, server_key: "", server_fingerprint: "", key_recorded: false, url: "https://matv-studio.github.io/alunites-revel-lab/", pages_enabled: true });
          return null;
        }
        for (let i = 1; i <= 14; i++) { await wait(params.get("pubslow") ? 700 : 40); ch.onmessage({ event: "uploaded", done: i, total: 14 }); }
        if (!args.dryRun) { if (published[args.profileId]) previous[args.profileId] = published[args.profileId]; published[args.profileId] = Date.now(); delete pendingChanges[args.profileId]; }
        ch.onmessage({ event: "finished", url: profiles.find(([p]) => p.id === args.profileId)?.[0].site_url, uploaded: 14, deleted: 2, total_files: 4537, bytes: 3_400_000, dry_run: !!args.dryRun, server_key: "x ssh-ed25519 AAAA", server_fingerprint: "SHA256:/bPN11w+yMb4D7FWpnTAuPkibmBXY17S0Uw/rDzIJv8", key_recorded: true });
        return null;
      }
      default: return null;
    }
  }
  window.__TAURI__ = { core: { invoke, Channel }, __mock: true };
})();
