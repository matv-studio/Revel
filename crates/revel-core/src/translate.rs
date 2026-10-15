//! Traduction automatique hors ligne (D-36) : modèles OPUS-MT exécutés sur le processeur, sans
//! service extérieur ni connexion pendant la traduction.
//!
//! Dossiers des modèles, lus dans l'ordre : `REVEL_MODELS` (essais, ligne de commande), modèles
//! livrés avec l'application (ressources de l'installateur : français et anglais), langues
//! téléchargées depuis les réglages (`<réglages>/models`).

use crate::{Core, CoreError, Result};
use revel_protocol::{PackState, TranslateEvent, TranslateItem, TranslatePack};
use revel_translate::{Engine, Format, Item, TranslateError, catalog, install};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn format(f: Option<&str>, text: &str) -> Format {
    match f {
        Some("plain") => Format::Plain,
        Some("markdown") => Format::Markdown,
        Some("html") => Format::Html,
        _ => Format::guess(text),
    }
}

fn error(e: TranslateError) -> CoreError {
    match e {
        TranslateError::NoPair(a, b) => CoreError::NoTranslation(a, b),
        TranslateError::Model(m) => CoreError::Translate(m),
        TranslateError::Download(d) => CoreError::Translate(d),
        TranslateError::Integrity(d) => CoreError::TranslateIntegrity(d),
        TranslateError::Cancelled => CoreError::TranslateCancelled,
    }
}

impl Core {
    /// Dossier des modèles livrés avec l'application.
    pub fn set_models_dir(&self, dir: &Path) {
        *self
            .bundled_models
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(dir.to_path_buf());
        self.reset_engine();
    }

    /// Dossiers des modèles livrés (variable d'environnement, ressources de l'application).
    fn bundled_roots(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = std::env::var_os("REVEL_MODELS")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .into_iter()
            .collect();
        roots.extend(
            self.bundled_models
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        );
        roots
    }

    /// Langues livrées retirées par l'utilisateur.
    fn translate_off(&self) -> Vec<String> {
        self.with_state(|s| s.translate_off.clone())
    }

    fn engine(&self) -> Arc<Engine> {
        let hidden: Vec<String> = self
            .translate_off()
            .iter()
            .filter_map(|c| catalog::pack(c))
            .flat_map(|p| p.models.iter().map(|m| m.name.to_string()))
            .collect();
        let mut t = self.translator.lock().unwrap_or_else(|e| e.into_inner());
        t.get_or_insert_with(|| {
            let mut roots = self.bundled_roots();
            roots.push(self.user_models.clone());
            Arc::new(Engine::with_hidden(roots, hidden))
        })
        .clone()
    }

    /// Langues changées : moteur recréé (modèles chargés libérés).
    fn reset_engine(&self) {
        *self.translator.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Traduction automatique possible d'une langue vers une autre.
    pub fn translate_can(&self, from: &str, to: &str) -> bool {
        self.engine().can(from, to)
    }

    /// Paires de langues des modèles présents (de, vers).
    pub fn translate_pairs(&self) -> Vec<(String, String)> {
        self.engine().pairs()
    }

    /// Traduit des textes ; `keep` : mots gardés tels quels (nom du site, marques). Peut durer
    /// plusieurs secondes : à appeler hors du fil de l'interface, par petits lots.
    pub fn translate(
        &self,
        from: &str,
        to: &str,
        items: &[TranslateItem],
        keep: &[String],
    ) -> Result<Vec<String>> {
        let items: Vec<Item> = items
            .iter()
            .map(|i| Item {
                format: format(i.format.as_deref(), &i.text),
                text: i.text.clone(),
            })
            .collect();
        self.engine()
            .translate(from, to, &items, keep)
            .map_err(error)
    }

    /// Langues du catalogue : livrées, téléchargées ou à télécharger.
    pub fn translate_packs(&self) -> Vec<TranslatePack> {
        let bundled = self.bundled_roots();
        let off = self.translate_off();
        catalog::PACKS
            .iter()
            .map(|p| TranslatePack {
                code: p.code.to_string(),
                state: if bundled.iter().any(|r| install::installed(p, r)) {
                    if off.iter().any(|c| c == p.code) {
                        PackState::BundledOff
                    } else {
                        PackState::Bundled
                    }
                } else if install::installed(p, &self.user_models) {
                    PackState::Installed
                } else {
                    PackState::Available
                },
                download: p.download_size(),
                disk: p.disk_size(),
            })
            .collect()
    }

    /// Télécharge et installe une langue (une à la fois) ; `on` reçoit l'avancement.
    pub fn translate_install(&self, code: &str, on: impl Fn(TranslateEvent)) -> Result<()> {
        let pack = catalog::pack(code)
            .ok_or_else(|| CoreError::NoTranslation(catalog::HUB.into(), code.into()))?;
        // Langue livrée retirée : remise en service, sans téléchargement.
        if self
            .bundled_roots()
            .iter()
            .any(|r| install::installed(pack, r))
        {
            self.with_state(|s| s.translate_off.retain(|c| c != code));
            self.reset_engine();
            return Ok(());
        }
        if self.translate_busy.swap(true, Ordering::SeqCst) {
            return Err(CoreError::TranslateBusy);
        }
        self.translate_cancel.store(false, Ordering::SeqCst);
        // Avancement transmis au plus tous les 0,5 % (pas un message par bloc lu).
        let mut last = 0u64;
        let r = install::install(
            pack,
            &self.user_models,
            &self.translate_cancel,
            &mut |p| match p {
                install::Progress::Download { done, total } => {
                    let step = (total / 200).max(1);
                    if done == total || done >= last + step {
                        last = done;
                        on(TranslateEvent::Progress { done, total });
                    }
                }
                install::Progress::Prepare => on(TranslateEvent::Preparing),
            },
        );
        self.translate_busy.store(false, Ordering::SeqCst);
        self.reset_engine();
        r.map_err(error)
    }

    /// Arrête le téléchargement en cours.
    pub fn translate_install_cancel(&self) {
        self.translate_cancel.store(true, Ordering::SeqCst);
    }

    /// Retire une langue : fichiers téléchargés supprimés ; langue livrée avec l'application mise
    /// hors service (ses fichiers font partie de l'installation, en lecture seule).
    pub fn translate_remove(&self, code: &str) -> Result<()> {
        let pack = catalog::pack(code)
            .ok_or_else(|| CoreError::NoTranslation(catalog::HUB.into(), code.into()))?;
        if self
            .bundled_roots()
            .iter()
            .any(|r| install::installed(pack, r))
        {
            self.with_state(|s| {
                if !s.translate_off.iter().any(|c| c == code) {
                    s.translate_off.push(code.to_string());
                }
            });
        }
        self.reset_engine();
        install::remove(pack, &self.user_models).map_err(error)
    }
}
