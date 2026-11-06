//! Modèles 3D (D-160) : un élément `data-model="/media/x.glb"` (GLB ou glTF) est affiché par
//! three.js (licence MIT, `vendor/three/LICENSE`), hébergé avec le site : aucun service extérieur.
//! Les pages qui en contiennent reçoivent `/assets/revel-model.js` (lecteur et réglages), qui
//! charge le moteur (`/assets/revel-3d/three.js`) quand un modèle approche de l'écran ; les
//! décodeurs Draco et Basis (même dossier) ne sont téléchargés que si le fichier en a besoin.
//! Un élément `data-mview="/media/x.mview"` (scène de Marmoset Toolbag) reçoit
//! `/assets/revel-mview.js`, qui charge le lecteur officiel de Marmoset depuis son serveur.
//! Le canevas de l'éditeur reçoit les mêmes scripts (le modèle s'y voit).

use crate::OutputFile;

/// Adresse du lecteur de modèles dans le site.
pub const URL: &str = "/assets/revel-model.js";

/// Dossier du moteur et des décodeurs.
pub const LIB: &str = "/assets/revel-3d/";

/// Adresse du lecteur de scènes Marmoset dans le site.
pub const MVIEW_URL: &str = "/assets/revel-mview.js";

/// Version de three.js assemblée par `tools/three/`.
pub const THREE_VERSION: &str = "0.186.1";

const ENGINE: &str = include_str!("../vendor/three/revel-three.min.js");
const LOADER: &str = include_str!("model.js");
const MVIEW: &str = include_str!("mview.js");

/// Fichiers du dossier `LIB` : nom et contenu (moteur, décodeurs, licences).
pub fn lib_files() -> [(&'static str, &'static [u8]); 7] {
    [
        ("three.js", ENGINE.as_bytes()),
        (
            "draco_wasm_wrapper.js",
            include_bytes!("../vendor/three/draco_wasm_wrapper.js"),
        ),
        (
            "draco_decoder.wasm",
            include_bytes!("../vendor/three/draco_decoder.wasm"),
        ),
        (
            "basis_transcoder.js",
            include_bytes!("../vendor/three/basis_transcoder.js"),
        ),
        (
            "basis_transcoder.wasm",
            include_bytes!("../vendor/three/basis_transcoder.wasm"),
        ),
        ("LICENSE", include_bytes!("../vendor/three/LICENSE")),
        ("VERSION", include_bytes!("../vendor/three/VERSION")),
    ]
}

/// Contenu d'un fichier servi sous `/assets/revel-3d/` ou de l'un des deux lecteurs (canevas).
pub fn file(path: &str) -> Option<Vec<u8>> {
    if path == URL {
        return Some(script().into_bytes());
    }
    if path == MVIEW_URL {
        return Some(mview_script().into_bytes());
    }
    let name = path.strip_prefix(LIB)?;
    lib_files()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b.to_vec())
}

/// Mots affichés par le lecteur (vue « Calques », compteur) : (mot du lecteur, clé de l'interface).
const WORDS: [(&str, &str); 8] = [
    ("render", "model-view-render"),
    ("normals", "model-view-normals"),
    ("albedo", "model-view-albedo"),
    ("metalness", "model-view-metalness"),
    ("roughness", "model-view-roughness"),
    ("wireframe", "model-view-wireframe"),
    ("triangles", "model-stats-triangles"),
    ("vertices", "model-stats-vertices"),
];

/// Mots du lecteur dans les langues de l'interface (mêmes noms que les réglages), en JSON.
fn words() -> String {
    let mut all = serde_json::Map::new();
    for (code, _) in revel_i18n::LOCALES {
        let l = revel_i18n::Localizer::new(code);
        let mut one = serde_json::Map::new();
        for (word, key) in WORDS {
            one.insert(word.into(), l.t(key, &[]).into());
        }
        all.insert((*code).into(), one.into());
    }
    serde_json::Value::Object(all).to_string()
}

/// Lecteur de modèles (sans commentaires de bloc).
pub fn script() -> String {
    let code = LOADER.replacen("/*WORDS*/{}/*/WORDS*/", &words(), 1);
    crate::lottie::strip_comments(&code)
}

/// Lecteur de scènes Marmoset (sans commentaires de bloc).
pub fn mview_script() -> String {
    crate::lottie::strip_comments(MVIEW)
}

/// La page contient un modèle 3D.
pub fn uses(html: &str) -> bool {
    html.contains(" data-model=")
}

/// La page contient une scène Marmoset.
pub fn uses_mview(html: &str) -> bool {
    html.contains(" data-mview=")
}

/// Ce qu'une page utilise : modèle, scène Marmoset.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Used {
    pub model: bool,
    pub mview: bool,
}

/// Page avec un modèle ou une scène : script(s) ajouté(s) avant `</head>`.
pub fn wire(html: String) -> (String, Used) {
    let used = Used {
        model: uses(&html),
        mview: uses_mview(&html),
    };
    let mut code = String::new();
    if used.model {
        code.push_str(&format!("<script src=\"{URL}\" defer></script>"));
    }
    if used.mview {
        code.push_str(&format!("<script src=\"{MVIEW_URL}\" defer></script>"));
    }
    if code.is_empty() {
        return (html, used);
    }
    (crate::insert_before(&html, "</head>", &code, false), used)
}

/// Fichiers ajoutés au site si une page au moins contient un modèle ou une scène.
pub fn script_files(used: Used, files: &mut Vec<OutputFile>) {
    if used.model {
        files.push(OutputFile {
            path: URL.trim_start_matches('/').to_string(),
            bytes: script().into_bytes(),
        });
        for (name, bytes) in lib_files() {
            files.push(OutputFile {
                path: format!("{}{name}", LIB.trim_start_matches('/')),
                bytes: bytes.to_vec(),
            });
        }
    }
    if used.mview {
        files.push(OutputFile {
            path: MVIEW_URL.trim_start_matches('/').to_string(),
            bytes: mview_script().into_bytes(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mots_du_lecteur_traduits() {
        let js = script();
        assert!(!js.contains("WORDS*/"));
        assert!(js.contains("\"wireframe\":\"Filaire\""), "français");
        assert!(js.contains("\"vertices\":\"Vertices\""), "anglais");
    }

    #[test]
    fn page_avec_modele() {
        let html = "<html><head></head><body><div data-model=\"/media/a.glb\"></div></body></html>";
        let (out, used) = wire(html.to_string());
        assert_eq!(
            used,
            Used {
                model: true,
                mview: false
            }
        );
        assert!(out.contains("<script src=\"/assets/revel-model.js\" defer></script>\n</head>"));
        let (_, used) = wire(
            "<html><head></head><body><div data-mview=\"/m.mview\"></div></body></html>".into(),
        );
        assert!(used.mview && !used.model);
        let (same, used) = wire("<html><head></head><body></body></html>".into());
        assert_eq!(used, Used::default());
        assert!(!same.contains("revel-model"));
        let s = script();
        assert!(s.contains("window.RevelModel"));
        assert!(!s.contains("data-model-poster : image"));
        assert!(ENGINE.starts_with(&format!("/*! three.js {THREE_VERSION} | MIT License")));
        assert!(ENGINE.contains("RevelThree"));
        let mut files = Vec::new();
        script_files(
            Used {
                model: true,
                mview: true,
            },
            &mut files,
        );
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        for p in [
            "assets/revel-model.js",
            "assets/revel-3d/three.js",
            "assets/revel-3d/draco_decoder.wasm",
            "assets/revel-3d/basis_transcoder.wasm",
            "assets/revel-mview.js",
        ] {
            assert!(paths.contains(&p), "{p} : {paths:?}");
        }
        assert!(file("/assets/revel-3d/draco_wasm_wrapper.js").is_some());
        assert!(file("/assets/revel-3d/../x").is_none());
        assert!(mview_script().contains("viewer.marmoset.co"));
    }
}
