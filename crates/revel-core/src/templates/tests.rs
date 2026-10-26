use super::*;

#[test]
fn langue_choisie() {
    assert_eq!(localize("a [[Bonjour||Hello]] b", true), "a Bonjour b");
    assert_eq!(localize("a [[Bonjour||Hello]] b", false), "a Hello b");
    assert_eq!(localize("[[x]] {{ a or b }}", true), "[[x]] {{ a or b }}");
}

#[test]
fn couleurs_selon_le_theme() {
    let css = ":root { color-scheme: light; --bg: #fff; --text: #111; }\n@media (prefers-color-scheme: dark) {\n  :root { color-scheme: dark; --bg: #000; --text: #eee; }\n}\n.a { color: var(--text); }\n";
    assert_eq!(apply_scheme(css, Scheme::Auto), css);
    let light = apply_scheme(css, Scheme::Light);
    assert!(!light.contains("prefers-color-scheme") && light.contains("--bg: #fff"));
    let dark = apply_scheme(css, Scheme::Dark);
    assert!(!dark.contains("prefers-color-scheme"), "{dark}");
    assert!(
        dark.contains("--bg: #000") && dark.contains("--text: #eee"),
        "{dark}"
    );
    assert!(dark.contains("color-scheme: dark"), "{dark}");
}

/// Chaque modèle, en français et en anglais : construit sans reste de balisage des modèles,
/// dans la forme de l'interface (relevé vide : tout se règle dans les panneaux).
#[test]
fn tous_les_modeles() {
    let ids = ids();
    assert!(!ids.is_empty());
    let cat = catalogue("fr");
    assert_eq!(cat.len(), ids.len());
    let mut problems = Vec::new();
    for id in &ids {
        for lang in ["fr", "en"] {
            let b = match build(id, "Essai", lang, Scheme::Auto) {
                Ok(b) => b,
                Err(e) => {
                    problems.push(format!("{id} ({lang}) : {e}"));
                    continue;
                }
            };
            let doc = serde_json::to_string(&b.site).unwrap();
            for left in ["[[", "]]", "@@"] {
                if doc.contains(left) {
                    problems.push(format!("{id} ({lang}) : « {left} » reste dans le document"));
                }
            }
            for (f, t) in &b.files {
                if t.contains("[[") || t.contains("@@") {
                    problems.push(format!("{id} ({lang}) : balisage resté dans {f}"));
                }
            }
            for i in revel_edit::editable::audit(&b.site) {
                problems.push(format!("{id} ({lang}) : relevé : {i:?}"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Forme écrite de chaque élément du panneau « Ajouter » (relecture pour écrire les modèles) :
/// `cargo test -p revel-core outils_ecrits -- --ignored --nocapture`.
#[test]
#[ignore]
fn outils_ecrits() {
    use revel_edit::make::Preset;
    fn strip(nodes: &mut [Node]) {
        for n in nodes {
            if let Some(id) = revel_model::ids::node_id_mut(n) {
                *id = None;
            }
            for l in revel_model::ids::child_lists_mut(n) {
                strip(l);
            }
        }
    }
    for p in Preset::ALL {
        let mut s = crate::starter::starter_site("Essai", "fr");
        let main = revel_model::ids::node_id(&s.pages[0].body[0])
            .map(str::to_string)
            .unwrap_or_default();
        let parent = revel_edit::Parent::of(
            &revel_edit::Target {
                owner: revel_edit::Owner::Page("index".into()),
                node: main,
            },
            revel_edit::Branch::Children,
        );
        let _ = revel_edit::apply(
            &mut s,
            &revel_edit::Command::InsertPreset {
                parent,
                index: 0,
                preset: p,
                text: String::new(),
            },
        );
        let Node::Element { children, .. } = &s.pages[0].body[0] else {
            continue;
        };
        let mut nodes = children[..1].to_vec();
        strip(&mut nodes);
        println!("=== {p:?}\n{}", markup::print(&nodes));
    }
}
