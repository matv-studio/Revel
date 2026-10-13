// Écran d'accueil de l'étape 0 : vérifie la liaison interface <-> Rust.
const $ = (id) => document.getElementById(id);

async function load(lang) {
  const tauri = window.__TAURI__;
  if (!tauri) {
    $("tagline").textContent = "(aperçu hors application)";
    return;
  }
  const info = await tauri.core.invoke("app_info", { lang });
  document.documentElement.lang = info.lang;
  $("title").textContent = info.title;
  $("tagline").textContent = info.tagline;
  $("version").textContent = info.version;
  $("stage").textContent = info.stage;
  $("schema").textContent = info.schema_version;
  $("hosts").textContent = info.hosts.join(", ");
  $("lang-label").textContent = info.lang === "fr" ? "Langue" : "Language";
  $("lang").value = info.lang;
}

const initial = (() => {
  try {
    return localStorage.getItem("revel.lang") || navigator.language || "fr";
  } catch {
    return navigator.language || "fr";
  }
})();

$("lang").addEventListener("change", (e) => {
  try { localStorage.setItem("revel.lang", e.target.value); } catch {}
  load(e.target.value);
});

load(initial);
