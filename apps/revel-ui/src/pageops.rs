//! Actions sur les pages fixes (3.4) : créer, dupliquer, supprimer, brouillon, adresse et
//! dossiers (chemins d'adresse), avec redirection 301 de l'ancienne adresse d'une page publiée.
//! Chaque action est une commande (ou une suite) annulable en une étape.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_edit::{Command, ShelfKind};
use revel_model::{Redirect, Shelf, Site};

/// Dossier (chemin d'adresse sans `/` de bord) d'une adresse de page : `/a/b/` -> `a`,
/// `/a/b.html` -> `a`, `/` -> ``.
pub fn folder_of(path: &str) -> String {
    let t = path.trim_matches('/');
    match t.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// Dernier segment d'une adresse (`/a/offres/` -> `offres`).
pub fn slug_of(path: &str) -> String {
    path.trim_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".html")
        .to_string()
}

/// Redirections après des changements d'adresse (`(ancienne, nouvelle)`) : une redirection par
/// page publiée déplacée, celles qui menaient à l'ancienne adresse mènent à la nouvelle, celles
/// qui partaient de la nouvelle adresse sont retirées (la page y est).
pub fn redirects_after(site: &Site, moves: &[(String, String)]) -> Vec<Redirect> {
    let mut out: Vec<Redirect> = site.redirects.clone();
    for (old, new) in moves {
        for r in out.iter_mut() {
            if &r.to == old {
                r.to = new.clone();
            }
        }
        out.retain(|r| &r.from != new && r.from != r.to);
        if !out.iter().any(|r| &r.from == old) {
            out.push(Redirect {
                from: old.clone(),
                to: new.clone(),
            });
        }
    }
    out
}

fn doc<T>(ed: Ed, f: impl FnOnce(&Site) -> T) -> Option<T> {
    ed.doc.with_untracked(|d| d.as_ref().map(f))
}

/// Nouvelle page `name` dans le dossier `folder`, ouverte après sa création.
pub fn create(ui: Ui, ed: Ed, name: &str, folder: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }
    // Site sans gabarit : gabarit de base créé avec la page (en-tête du document, feuille de
    // style), la page n'a que son contenu.
    let Some((layout, page)) = doc(ed, |s| {
        if s.layouts.is_empty() {
            let mut with = s.clone();
            let l = revel_edit::make::base_layout(s);
            with.layouts.push(l.clone());
            (Some(l), revel_edit::make::new_page(&with, name, folder))
        } else {
            (None, revel_edit::make::new_page(s, name, folder))
        }
    }) else {
        return false;
    };
    let url = page.path.clone();
    let add = Command::AddPage { page, index: None };
    let cmd = match layout {
        Some(layout) => Command::Batch {
            commands: vec![
                Command::AddLayout {
                    layout,
                    index: None,
                },
                add,
            ],
        },
        None => add,
    };
    ed.apply(ui, cmd, false);
    ui.goto_page.set(Some(url));
    true
}

/// Copie d'une page, ouverte après sa création.
pub fn duplicate(ui: Ui, ed: Ed, id: &str) {
    let word = ui.now("pages-copy-word", &[]);
    if let Some(Ok(page)) = doc(ed, |s| revel_edit::make::duplicate_page(s, id, &word)) {
        let url = page.path.clone();
        ed.apply(ui, Command::AddPage { page, index: None }, false);
        ui.goto_page.set(Some(url));
    }
}

/// Supprime une page (confirmation) ; la page d'accueil reste.
pub fn delete(ui: Ui, ed: Ed, id: &str) {
    let Some(Some((path, name))) = doc(ed, |s| {
        s.pages.iter().find(|p| p.id == id).map(|p| {
            (
                p.path.clone(),
                p.name.clone().unwrap_or_else(|| p.path.clone()),
            )
        })
    }) else {
        return;
    };
    if path == "/" {
        return;
    }
    let id = id.to_string();
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("pages-delete-title", &[("name", name)]),
            Some(ui.now("pages-delete-body", &[])),
            ui.now("pages-delete", &[]),
            move || {
                if ed.page.get_untracked() == path {
                    ui.goto_page.set(Some("/".into()));
                }
                ed.apply(ui, Command::RemovePage { id: id.clone() }, false);
            },
        )
        .danger(),
    ));
}

/// Brouillon (non publié) ou publié.
pub fn set_draft(ui: Ui, ed: Ed, id: &str, draft: bool) {
    ed.apply(
        ui,
        Command::SetPageDraft {
            id: id.to_string(),
            draft,
        },
        false,
    );
}

/// Nouvelle adresse d'une page (`redirect` : redirection 301 de l'ancienne si la page est
/// publiée). Renvoie `false` si l'adresse est refusée (invalide ou prise).
pub fn set_path(ui: Ui, ed: Ed, id: &str, path: &str, redirect: bool) -> bool {
    let Some(Some((old, draft))) = doc(ed, |s| {
        s.pages
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.path.clone(), p.draft))
    }) else {
        return false;
    };
    if old == path || old == "/" {
        return old == path;
    }
    let ok = doc(ed, |s| {
        revel_model::valid_page_path(path) && !s.pages.iter().any(|p| p.path == path)
    })
    .unwrap_or(false);
    if !ok {
        return false;
    }
    let mut commands = vec![Command::SetPagePath {
        id: id.to_string(),
        path: path.to_string(),
    }];
    if redirect && !draft {
        let redirects = doc(ed, |s| {
            redirects_after(s, &[(old.clone(), path.to_string())])
        })
        .unwrap_or_default();
        commands.push(Command::SetRedirects { redirects });
    }
    let current = ed.page.get_untracked() == old;
    ed.apply(ui, Command::Batch { commands }, false);
    if current {
        ui.goto_page.set(Some(path.to_string()));
    }
    true
}

/// Range une page dans un dossier (`""` : racine), même dernier segment d'adresse.
pub fn move_to(ui: Ui, ed: Ed, id: &str, folder: &str) {
    let Some(Some(path)) = doc(ed, |s| {
        let p = s.pages.iter().find(|p| p.id == id)?;
        let slug = slug_of(&p.path);
        Some(revel_edit::make::free_page_path(s, folder, &slug))
    }) else {
        return;
    };
    set_path(ui, ed, id, &path, true);
}

/// Dossiers de pages : ceux du rangement (créés vides) et ceux des adresses.
pub fn folders(site: &Site) -> Vec<String> {
    let mut out: Vec<String> = site.library.pages.folders.clone();
    for p in &site.pages {
        let mut f = folder_of(&p.path);
        while !f.is_empty() {
            if !out.contains(&f) {
                out.push(f.clone());
            }
            f = match f.rsplit_once('/') {
                Some((a, _)) => a.to_string(),
                None => String::new(),
            };
        }
    }
    out.sort();
    out
}

fn set_shelf(ui: Ui, ed: Ed, shelf: Shelf) {
    ed.apply(
        ui,
        Command::SetShelf {
            kind: ShelfKind::Pages,
            shelf,
        },
        false,
    );
}

/// Nouveau dossier `name` dans `parent` ; `false` : nom refusé ou déjà pris.
pub fn create_folder(ui: Ui, ed: Ed, parent: &str, name: &str) -> bool {
    let slug = revel_model::slugify(name);
    if slug.is_empty() {
        return false;
    }
    let path = if parent.is_empty() {
        slug
    } else {
        format!("{parent}/{slug}")
    };
    let Some((exists, mut shelf)) = doc(ed, |s| {
        (
            folders(s).contains(&path) || s.pages.iter().any(|p| p.path == format!("/{path}/")),
            s.library.pages.clone(),
        )
    }) else {
        return false;
    };
    if exists {
        return false;
    }
    shelf.folders.push(path);
    set_shelf(ui, ed, shelf);
    true
}

/// Dossier renommé : adresses de ses pages changées (redirections pour les pages publiées),
/// rangement suivi. `false` : nom refusé ou déjà pris.
pub fn rename_folder(ui: Ui, ed: Ed, path: &str, name: &str) -> bool {
    let slug = revel_model::slugify(name);
    if slug.is_empty() {
        return false;
    }
    let parent = match path.rsplit_once('/') {
        Some((a, _)) => a.to_string(),
        None => String::new(),
    };
    let new = if parent.is_empty() {
        slug
    } else {
        format!("{parent}/{slug}")
    };
    if new == path {
        return true;
    }
    let Some(r) = doc(ed, |s| {
        if folders(s).contains(&new) {
            return None;
        }
        let old_prefix = format!("/{path}/");
        let new_prefix = format!("/{new}/");
        let mut commands = Vec::new();
        let mut moves = Vec::new();
        for p in s.pages.iter().filter(|p| p.path.starts_with(&old_prefix)) {
            let np = format!("{new_prefix}{}", &p.path[old_prefix.len()..]);
            if s.pages.iter().any(|x| x.path == np) {
                return None;
            }
            commands.push(Command::SetPagePath {
                id: p.id.clone(),
                path: np.clone(),
            });
            if !p.draft {
                moves.push((p.path.clone(), np));
            }
        }
        let mut shelf = s.library.pages.clone();
        for f in shelf.folders.iter_mut() {
            if *f == path || f.starts_with(&format!("{path}/")) {
                *f = format!("{new}{}", &f[path.len()..]);
            }
        }
        if shelf != s.library.pages {
            commands.push(Command::SetShelf {
                kind: ShelfKind::Pages,
                shelf,
            });
        }
        if !moves.is_empty() {
            commands.push(Command::SetRedirects {
                redirects: redirects_after(s, &moves),
            });
        }
        Some((commands, moves))
    }) else {
        return false;
    };
    let Some((commands, moves)) = r else {
        return false;
    };
    if commands.is_empty() {
        return true;
    }
    let cur = ed.page.get_untracked();
    ed.apply(ui, Command::Batch { commands }, false);
    if let Some((_, n)) = moves.iter().find(|(o, _)| *o == cur) {
        ui.goto_page.set(Some(n.clone()));
    }
    true
}

/// Dossier vide supprimé (un dossier qui contient des pages reste).
pub fn delete_folder(ui: Ui, ed: Ed, path: &str) {
    let Some((empty, mut shelf)) = doc(ed, |s| {
        let prefix = format!("/{path}/");
        (
            !s.pages.iter().any(|p| p.path.starts_with(&prefix)),
            s.library.pages.clone(),
        )
    }) else {
        return;
    };
    if !empty {
        return;
    }
    shelf
        .folders
        .retain(|f| f != path && !f.starts_with(&format!("{path}/")));
    set_shelf(ui, ed, shelf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_et_redirections() {
        assert_eq!(folder_of("/a/b/"), "a");
        assert_eq!(folder_of("/a/b/c.html"), "a/b");
        assert_eq!(folder_of("/a/"), "");
        assert_eq!(folder_of("/"), "");
        assert_eq!(slug_of("/a/offres/"), "offres");
        let mut s = Site::new("x", "fr");
        s.redirects = vec![
            Redirect {
                from: "/vieux/".into(),
                to: "/offres/".into(),
            },
            Redirect {
                from: "/emplois/".into(),
                to: "/ailleurs/".into(),
            },
        ];
        let r = redirects_after(&s, &[("/offres/".into(), "/emplois/".into())]);
        let pairs: Vec<(&str, &str)> = r.iter().map(|r| (r.from.as_str(), r.to.as_str())).collect();
        assert_eq!(
            pairs,
            vec![("/vieux/", "/emplois/"), ("/offres/", "/emplois/")]
        );
    }
}
