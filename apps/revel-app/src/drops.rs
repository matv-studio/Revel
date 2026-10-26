//! Dossiers déposés sur la fenêtre (D-122) : ajout d'un site par glisser-déposer.
//!
//! Les fichiers déposés passent par le glisser-déposer HTML de l'interface (le gestionnaire de
//! Tauri est coupé : `dragDropEnabled: false`, nécessaire sous Windows aux glisser-déposer
//! internes). Le navigateur ne donne pas le chemin d'un fichier déposé ; sous Windows, WebView2 le
//! donne pour les objets transmis par `chrome.webview.postMessageWithAdditionalObjects` (objets
//! `ICoreWebView2File`). L'interface y envoie les fichiers déposés, puis demande les chemins reçus
//! (`dropped_paths`). Ailleurs, le dossier est cherché par son nom (`find_dropped`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Chemins des derniers fichiers déposés (Windows), lus une fois.
#[derive(Default)]
pub struct Dropped(pub Mutex<Vec<String>>);

/// Branche la réception des fichiers déposés sur la fenêtre principale (Windows). Appels COM de
/// WebView2 (`unsafe` par nature) : lecture des objets joints au message, rien d'autre.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn watch(app: &tauri::App) {
    use tauri::Manager;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
    };
    use webview2_com::{WebMessageReceivedEventHandler, take_pwstr};
    use windows_core::{Interface, PWSTR};
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let handle = app.handle().clone();
    let _ = win.with_webview(move |pv| unsafe {
        let Ok(core) = pv.controller().CoreWebView2() else {
            return;
        };
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            // Message sans objets joints (appels de l'interface) : rien à faire ici.
            let Ok(args) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else {
                return Ok(());
            };
            let Ok(objs) = args.AdditionalObjects() else {
                return Ok(());
            };
            let mut n = 0u32;
            objs.Count(&mut n)?;
            let mut paths = Vec::new();
            for i in 0..n {
                let Ok(o) = objs.GetValueAtIndex(i) else {
                    continue;
                };
                if let Ok(f) = o.cast::<ICoreWebView2File>() {
                    let mut p = PWSTR::null();
                    if f.Path(&mut p).is_ok() {
                        paths.push(take_pwstr(p));
                    }
                }
            }
            if !paths.is_empty() {
                use tauri::Manager;
                if let Some(d) = handle.try_state::<Dropped>() {
                    *d.0.lock().unwrap_or_else(|e| e.into_inner()) = paths;
                }
            }
            Ok(())
        }));
        let mut token = 0i64;
        let _ = core.add_WebMessageReceived(&handler, &mut token);
    });
}

#[cfg(not(windows))]
pub fn watch(_app: &tauri::App) {}

/// Chemins reçus depuis le dernier dépôt (vidés à la lecture).
#[tauri::command]
pub fn dropped_paths(d: tauri::State<Dropped>) -> Vec<String> {
    std::mem::take(&mut *d.0.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Dossiers nommés `names` dans les emplacements connus : dossiers parents des sites récents, dossier
/// des sites, Documents, Bureau, Téléchargements, dossier personnel. Les premiers trouvés d'abord.
pub fn find_named(names: &[String], places: &[PathBuf]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        if name.is_empty() || name.contains(['/', '\\']) || name == ".." {
            continue;
        }
        for p in places {
            let c = p.join(name);
            if c.is_dir() {
                let s = c.display().to_string();
                if !out.contains(&s) {
                    out.push(s);
                }
                break;
            }
        }
    }
    out
}

/// Emplacements où chercher un dossier déposé, sans doublon.
pub fn places(
    recent: &[String],
    sites_dir: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if !v.contains(&p) {
            v.push(p);
        }
    };
    for r in recent {
        if let Some(parent) = Path::new(r).parent() {
            push(parent.to_path_buf());
        }
    }
    if let Some(d) = sites_dir {
        push(d);
    }
    if let Some(h) = home {
        for sub in [
            "Documents",
            "Desktop",
            "Bureau",
            "Downloads",
            "Téléchargements",
        ] {
            push(h.join(sub));
        }
        push(h);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_depose_retrouve_par_son_nom() {
        let t = std::env::temp_dir().join(format!("revel-drop-{}", std::process::id()));
        let projets = t.join("Projets");
        std::fs::create_dir_all(projets.join("matv")).unwrap();
        std::fs::create_dir_all(projets.join("autre")).unwrap();
        let recent = vec![projets.join("autre").display().to_string()];
        let pl = places(&recent, None, Some(t.join("maison")));
        assert_eq!(pl[0], projets);
        let found = find_named(&["matv".into(), "absent".into(), "../x".into()], &pl);
        assert_eq!(found, vec![projets.join("matv").display().to_string()]);
        let _ = std::fs::remove_dir_all(&t);
    }
}
