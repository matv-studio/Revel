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
  };
  const fail = params.get("fail");
  let github = params.get("github") || null;
  const ghProfile = [{ id: "h2", name: "GitHub Pages", environment: "production", target: { kind: "github_pages", repo: "matv-studio/alunites-revel-lab", branch: "gh-pages", custom_domain: null } }, true];
  const profiles = params.get("noprofiles") ? [] : params.get("ghprofile") ? [ghProfile] : [[{ id: "h1", name: "Recette OVH", environment: "staging", target: { kind: "sftp", host: "ftp.cluster000.hosting.ovh.net", port: 22, user: "monsite-deploy", remote_dir: "recette-v4k8/", known_host_key: null } }, true]];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  class Channel { constructor() { this.onmessage = null; } }
  // Pont vers un vrai cœur de Revel (`revel canvas-serve --bridge`) : paramètre `bridge=adresse`.
  // Les commandes de l'éditeur y sont transmises ; le reste reste simulé.
  const bridge = params.get("bridge");
  const bridged = ["edit_open", "edit_apply", "edit_seal", "edit_undo", "edit_redo", "edit_pages", "edit_stamp", "canvas_open"];
  async function invoke(cmd, args) {
    if (bridge && bridged.includes(cmd)) {
      // Corps en texte brut : requête simple, sans demande préalable (CORS).
      const r = await fetch(bridge.replace(/\/$/, "") + "/__revel/invoke/" + cmd, {
        method: "POST", headers: { "content-type": "text/plain" }, body: JSON.stringify(args || {}),
      });
      const v = await r.json();
      if (!r.ok) throw v;
      return v;
    }
    switch (cmd) {
      case "get_settings": return state.settings;
      case "set_settings": state.settings = args.settings; return null;
      case "recent_sites": return state.recents;
      case "open_site": return site;
      case "has_site": return true;
      case "list_profiles": return profiles;
      case "build_site": {
        const ch = args.onEvent;
        ch.onmessage({ event: "progress", phase: "pages", done: 0, total: 1 });
        for (let i = 0; i <= 3843; i += 640) { await wait(60); ch.onmessage({ event: "progress", phase: "images", done: i, total: 3843 }); }
        ch.onmessage({ event: "finished", environment: args.environment, out_dir: "", generated: 70, copied: 443, images_produced: 12, images_cached: 3831, images_failed: 0, seconds: 3.4, warnings: [] });
        return null;
      }
      case "open_preview": throw { key: "error-not-built", args: [] };
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
