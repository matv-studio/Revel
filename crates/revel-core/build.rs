//! Modèles de site (D-128) : les fichiers de `templates/` sont inclus dans le programme (texte
//! seul, quelques centaines de Kio), listés dans `$OUT_DIR/templates.rs`.

use std::path::{Path, PathBuf};

fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, base, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            out.push((rel, p.clone()));
        }
    }
}

fn main() {
    let base = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("templates");
    println!("cargo:rerun-if-changed=templates");
    let mut files = Vec::new();
    walk(&base, &base, &mut files);
    let mut code = String::from("/// Fichiers des modèles : (chemin dans `templates/`, texte).\n");
    code.push_str("pub static FILES: &[(&str, &str)] = &[\n");
    for (rel, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        code.push_str(&format!(
            "    ({rel:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("templates.rs");
    std::fs::write(out, code).unwrap();
}
