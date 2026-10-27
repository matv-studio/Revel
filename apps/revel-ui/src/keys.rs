//! Raccourcis clavier de l'éditeur, modifiables dans les réglages de Revel (D-125).
//!
//! Chaque action a un identifiant (`undo`), un libellé et ses touches par défaut ; les touches
//! choisies par l'utilisateur (`AppSettings::shortcuts`) remplacent celles par défaut (vide :
//! raccourci retiré). Une combinaison s'écrit `Ctrl+Shift+Z`, `Alt+ArrowUp`, `Shift+1` ; les
//! chiffres sont lus par touche physique (claviers AZERTY), `Ctrl` vaut aussi Cmd sous macOS.

use crate::Ui;
use crate::editor::Key;
use leptos::prelude::*;

/// Action : identifiant, libellé (clé de traduction), touches par défaut.
pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub default: &'static [&'static str],
}

const fn a(id: &'static str, label: &'static str, default: &'static [&'static str]) -> Action {
    Action { id, label, default }
}

/// Actions qui ont un raccourci, dans l'ordre des réglages.
pub const ACTIONS: &[Action] = &[
    a("palette", "palette", &["Ctrl+K"]),
    a("undo", "editor-undo-short", &["Ctrl+Z"]),
    a("redo", "editor-redo-short", &["Ctrl+Shift+Z", "Ctrl+Y"]),
    a("duplicate", "layers-duplicate-short", &["Ctrl+D"]),
    a("delete", "layers-delete-short", &["Delete", "Backspace"]),
    a("copy", "editor-copy", &["Ctrl+C"]),
    a("cut", "editor-cut", &["Ctrl+X"]),
    a("paste", "editor-paste", &["Ctrl+V"]),
    a("wrap", "layers-wrap", &["Ctrl+G"]),
    a("unwrap", "editor-unwrap", &["Ctrl+Shift+G"]),
    a("hide", "style-hide", &["Ctrl+Shift+H"]),
    a("deselect", "editor-deselect", &["Escape"]),
    a("child", "editor-select-child", &["Enter"]),
    a("parent", "editor-select-parent", &["Shift+Enter"]),
    a("next", "keys-next", &["Tab"]),
    a("prev", "keys-prev", &["Shift+Tab"]),
    a("up", "keys-up", &["Alt+ArrowUp"]),
    a("down", "keys-down", &["Alt+ArrowDown"]),
    a("zoom-fit", "editor-zoom-fit-short", &["Shift+1", "Ctrl+1"]),
    a("zoom-100", "editor-zoom-100", &["Shift+0", "Ctrl+0"]),
    a("zoom-sel", "editor-zoom-selection", &["Shift+2", "Ctrl+2"]),
    a("zoom-in", "keys-zoom-in", &["Ctrl+="]),
    a("zoom-out", "keys-zoom-out", &["Ctrl+-"]),
    a("rulers", "editor-rulers", &["Ctrl+R"]),
];

/// Combinaison lue : modificateurs et touche (minuscules ; chiffre : `0`…`9`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combo {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: String,
}

impl Combo {
    pub fn parse(s: &str) -> Option<Combo> {
        let mut c = Combo {
            ctrl: false,
            shift: false,
            alt: false,
            key: String::new(),
        };
        // `Ctrl++` : la touche est le dernier `+`.
        let s = s.trim();
        let (mods, key) = match s.strip_suffix("++") {
            Some(m) => (m, "+"),
            None => match s.rsplit_once('+') {
                Some((m, k)) => (m, k),
                None => ("", s),
            },
        };
        for m in mods.split('+').filter(|m| !m.is_empty()) {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "meta" => c.ctrl = true,
                "shift" => c.shift = true,
                "alt" | "option" => c.alt = true,
                _ => return None,
            }
        }
        if key.is_empty() {
            return None;
        }
        c.key = key.to_lowercase();
        Some(c)
    }

    /// Touche appuyée (chiffres par touche physique ; `=` vaut aussi `+` du pavé).
    pub fn matches(&self, k: &Key) -> bool {
        if self.ctrl != k.ctrl || self.shift != k.shift || self.alt != k.alt {
            return false;
        }
        let digit = k
            .code
            .strip_prefix("Digit")
            .or_else(|| k.code.strip_prefix("Numpad"))
            .filter(|d| d.len() == 1);
        if self.key.len() == 1 && self.key.chars().all(|c| c.is_ascii_digit()) {
            return digit == Some(self.key.as_str());
        }
        let key = k.key.to_lowercase();
        if key == self.key {
            return true;
        }
        if self.key == "=" && (matches!(k.code.as_str(), "Equal" | "NumpadAdd") || key == "+") {
            return true;
        }
        if self.key == "-" && matches!(k.code.as_str(), "Minus" | "NumpadSubtract") {
            return true;
        }
        // Lettres par touche physique quand un modificateur change le caractère (Alt).
        self.key.len() == 1 && k.code == format!("Key{}", self.key.to_uppercase())
    }

    /// Écriture enregistrée (`Ctrl+Shift+Z`).
    pub fn write(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(&key_name(&self.key));
        s
    }

    /// Combinaison d'une touche appuyée (enregistrement d'un raccourci) ; `None` pour une
    /// touche de modification seule.
    pub fn of(k: &Key) -> Option<Combo> {
        let key = k.key.to_lowercase();
        if matches!(
            key.as_str(),
            "control" | "shift" | "alt" | "meta" | "altgraph" | "os"
        ) {
            return None;
        }
        let digit = k
            .code
            .strip_prefix("Digit")
            .or_else(|| k.code.strip_prefix("Numpad"))
            .filter(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()));
        let letter = k.code.strip_prefix("Key").map(str::to_lowercase);
        Some(Combo {
            ctrl: k.ctrl,
            shift: k.shift,
            alt: k.alt,
            key: digit
                .map(str::to_string)
                .or(if k.alt || k.shift { letter } else { None })
                .unwrap_or(key),
        })
    }
}

/// Nom d'une touche écrit dans les réglages (`ArrowUp`, `Delete`, `K`).
fn key_name(k: &str) -> String {
    match k {
        "arrowup" => "ArrowUp".into(),
        "arrowdown" => "ArrowDown".into(),
        "arrowleft" => "ArrowLeft".into(),
        "arrowright" => "ArrowRight".into(),
        "delete" => "Delete".into(),
        "backspace" => "Backspace".into(),
        "escape" => "Escape".into(),
        "enter" => "Enter".into(),
        "tab" => "Tab".into(),
        " " => "Space".into(),
        k if k.chars().count() == 1 => k.to_uppercase(),
        k => {
            let mut c = k.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        }
    }
}

/// Touches d'une action : celles choisies, sinon celles par défaut.
/// Élément où l'on écrit (texte, nombre, zone de texte, contenu modifiable) : il garde ses
/// raccourcis (annuler la saisie, copier…). Une liste de choix, une case ou un bouton ne les
/// garde pas : annuler y annule la dernière modification du site.
pub fn is_typing(el: &web_sys::Element) -> bool {
    match el.tag_name().as_str() {
        "TEXTAREA" => true,
        "INPUT" => !matches!(
            el.get_attribute("type")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
            "checkbox" | "radio" | "range" | "color" | "button" | "submit" | "reset" | "file"
        ),
        _ => el.has_attribute("contenteditable"),
    }
}

/// Annuler ou rétablir (Ctrl+Z, Ctrl+Maj+Z, Ctrl+Y) pendant que le focus est sur une liste de
/// choix ou une case : le raccourci vaut pour le site.
pub fn undo_on_control(e: &web_sys::KeyboardEvent) -> bool {
    (e.ctrl_key() || e.meta_key())
        && matches!(e.key().to_lowercase().as_str(), "z" | "y")
        && document()
            .active_element()
            .is_some_and(|el| !is_typing(&el))
}

pub fn bindings(ui: Ui, id: &str) -> Vec<String> {
    let custom = ui.settings.with_untracked(|s| s.shortcuts.get(id).cloned());
    match custom {
        Some(c) if c.trim().is_empty() => Vec::new(),
        Some(c) => vec![c],
        None => ACTIONS
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.default.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
    }
}

/// Action dont une touche correspond.
pub fn action_of(ui: Ui, k: &Key) -> Option<&'static str> {
    ACTIONS.iter().map(|a| a.id).find(|id| {
        bindings(ui, id)
            .iter()
            .filter_map(|b| Combo::parse(b))
            .any(|c| c.matches(k))
    })
}

/// Touches affichées dans la langue de l'interface (`Ctrl+Maj+Z · Ctrl+Y`).
pub fn display(ui: Ui, id: &str) -> String {
    bindings(ui, id)
        .iter()
        .map(|b| display_combo(ui, b))
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn display_combo(ui: Ui, b: &str) -> String {
    let Some(c) = Combo::parse(b) else {
        return b.to_string();
    };
    let mut parts: Vec<String> = Vec::new();
    if c.ctrl {
        parts.push("Ctrl".into());
    }
    if c.shift {
        parts.push(ui.now("keys-shift", &[]));
    }
    if c.alt {
        parts.push("Alt".into());
    }
    parts.push(match c.key.as_str() {
        "arrowup" => "↑".into(),
        "arrowdown" => "↓".into(),
        "arrowleft" => "←".into(),
        "arrowright" => "→".into(),
        "delete" => ui.now("keys-delete", &[]),
        "backspace" => "⌫".into(),
        "escape" => ui.now("keys-escape", &[]),
        "enter" => ui.now("keys-enter", &[]),
        " " | "space" => ui.now("keys-space", &[]),
        k => key_name(k),
    });
    parts.join("+")
}

/// Actions qui partagent une touche (conflit signalé dans les réglages).
pub fn conflicts(ui: Ui, id: &str) -> Vec<&'static str> {
    let mine: Vec<Combo> = bindings(ui, id)
        .iter()
        .filter_map(|b| Combo::parse(b))
        .collect();
    ACTIONS
        .iter()
        .filter(|a| a.id != id)
        .filter(|a| {
            bindings(ui, a.id)
                .iter()
                .filter_map(|b| Combo::parse(b))
                .any(|c| mine.contains(&c))
        })
        .map(|a| a.label)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: &str, code: &str, ctrl: bool, shift: bool, alt: bool) -> Key {
        Key {
            key: key.into(),
            code: code.into(),
            ctrl,
            shift,
            alt,
        }
    }

    #[test]
    fn combinaisons_lues_et_reconnues() {
        let c = Combo::parse("Ctrl+Shift+Z").unwrap();
        assert!(c.matches(&key("Z", "KeyZ", true, true, false)));
        assert!(!c.matches(&key("z", "KeyZ", true, false, false)));
        // Chiffre par touche physique (AZERTY : Maj+1 donne « 1 », sans Maj « & »).
        let z = Combo::parse("Shift+1").unwrap();
        assert!(z.matches(&key("1", "Digit1", false, true, false)));
        assert!(Combo::parse("Ctrl++").unwrap().key == "+");
        assert!(
            Combo::parse("Alt+ArrowUp")
                .unwrap()
                .matches(&key("ArrowUp", "ArrowUp", false, false, true))
        );
        assert_eq!(
            Combo::parse("ctrl+shift+arrowup").unwrap().write(),
            "Ctrl+Shift+ArrowUp"
        );
        assert!(Combo::parse("Hyper+K").is_none());
        // Enregistrement : Alt+lettre lue par touche physique (Alt change le caractère sous macOS).
        let rec = Combo::of(&key("π", "KeyP", false, false, true)).unwrap();
        assert_eq!(rec.write(), "Alt+P");
        assert!(Combo::of(&key("Shift", "ShiftLeft", false, true, false)).is_none());
    }
}
