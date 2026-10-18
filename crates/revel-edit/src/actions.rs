//! Interactions entre éléments (6.3) : un élément déclenche des actions sur lui-même ou sur
//! d'autres éléments, sans code (comme les interactions de Webflow). Elles sont écrites dans
//! l'attribut `data-revel-on` de l'élément déclencheur, une par point-virgule :
//!
//! `click class:is-open #menu; scroll:80 class:is-solid self`
//!
//! - déclencheur : `click` (à chaque appui, l'action s'inverse), `hover` (au survol, défaite en
//!   sortant), `scroll:<px>` (page défilée au-delà, défaite en revenant), `view` (élément à
//!   l'écran), `load` (au chargement) ;
//! - action : `toggle` (afficher ou masquer), `show`, `hide`, `class:<nom>` (classe ajoutée),
//!   `play` (lire ou mettre en pause une vidéo), `dismiss` (masquer et s'en souvenir : bandeau de
//!   consentement), `random:<ms>` (un enfant de l'élément visé, pris au hasard, affiché seul après
//!   le délai), `spin:<degrés>` (rotation ajoutée à chaque fois, plus un quart de tour au hasard) ;
//! - élément visé : `self`, `parent`, `next` (élément suivant), `body`, `.<classe>` (tous les
//!   éléments de la classe), `#<identifiant>`.
//!
//! Le script du site (`revel-render/src/actions.js`) les exécute.

/// Déclencheur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    Click,
    Hover,
    Scroll(u32),
    View,
    Load,
}

/// Action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    Toggle,
    Show,
    Hide,
    Class(String),
    Play,
    Dismiss,
    /// Un enfant au hasard affiché seul, après un délai (ms).
    Random(u32),
    /// Rotation ajoutée (degrés), plus 0 à 3 quarts de tour au hasard.
    Spin(u32),
}

/// Une interaction : déclencheur, action, élément visé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub trigger: Trigger,
    pub act: Act,
    pub target: String,
}

fn class_ok(c: &str) -> bool {
    !c.is_empty()
        && c.chars()
            .all(|x| x.is_alphanumeric() || x == '-' || x == '_')
}

/// Élément visé valide.
pub fn target_ok(t: &str) -> bool {
    matches!(t, "self" | "parent" | "next" | "body")
        || t.strip_prefix('.').is_some_and(class_ok)
        || t.strip_prefix('#').is_some_and(class_ok)
}

impl Rule {
    pub fn write(&self) -> String {
        let t = match &self.trigger {
            Trigger::Click => "click".to_string(),
            Trigger::Hover => "hover".to_string(),
            Trigger::Scroll(px) => format!("scroll:{px}"),
            Trigger::View => "view".to_string(),
            Trigger::Load => "load".to_string(),
        };
        let a = match &self.act {
            Act::Toggle => "toggle".to_string(),
            Act::Show => "show".to_string(),
            Act::Hide => "hide".to_string(),
            Act::Class(c) => format!("class:{c}"),
            Act::Play => "play".to_string(),
            Act::Dismiss => "dismiss".to_string(),
            Act::Random(ms) => format!("random:{ms}"),
            Act::Spin(deg) => format!("spin:{deg}"),
        };
        format!("{t} {a} {}", self.target)
    }

    pub fn parse(s: &str) -> Option<Rule> {
        let mut it = s.split_whitespace();
        let (t, a, target) = (it.next()?, it.next()?, it.next()?);
        if it.next().is_some() || !target_ok(target) {
            return None;
        }
        let trigger = match t {
            "click" => Trigger::Click,
            "hover" => Trigger::Hover,
            "view" => Trigger::View,
            "load" => Trigger::Load,
            t => Trigger::Scroll(t.strip_prefix("scroll:")?.parse().ok()?),
        };
        let act = match a {
            "toggle" => Act::Toggle,
            "show" => Act::Show,
            "hide" => Act::Hide,
            "play" => Act::Play,
            "dismiss" => Act::Dismiss,
            a if a.starts_with("random:") => Act::Random(a[7..].parse().ok()?),
            a if a.starts_with("spin:") => Act::Spin(a[5..].parse().ok()?),
            a => Act::Class(
                a.strip_prefix("class:")
                    .filter(|c| class_ok(c))?
                    .to_string(),
            ),
        };
        Some(Rule {
            trigger,
            act,
            target: target.to_string(),
        })
    }
}

/// Interactions d'un attribut ; `None` si une partie n'est pas reconnue (modifiable en texte).
pub fn parse(value: &str) -> Option<Vec<Rule>> {
    value
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(Rule::parse)
        .collect()
}

/// Valeur de l'attribut ; vide : attribut retiré.
pub fn write(rules: &[Rule]) -> String {
    rules.iter().map(Rule::write).collect::<Vec<_>>().join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let v = "click class:is-open #menu; scroll:80 class:is-solid self; hover play .video; load dismiss #consentement; click random:900 .mantras; click spin:540 .rosace";
        let r = parse(v).unwrap();
        assert_eq!(r.len(), 6);
        assert_eq!(r[4].act, Act::Random(900));
        assert_eq!(r[0].act, Act::Class("is-open".into()));
        assert_eq!(r[1].trigger, Trigger::Scroll(80));
        assert_eq!(write(&r), v);
        assert_eq!(parse("").unwrap(), vec![]);
        for bad in [
            "click",
            "click explode self",
            "tap toggle self",
            "click toggle a b",
            "click class: self",
            "click toggle #a.b",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }
}
