//! Recette de construction d'un projet : outil qui produit le site, moteur (Node.js, Hugo, Ruby),
//! gestionnaire de paquets, construction, dossiers produits, serveur d'aperçu.

use std::path::Path;

use serde_json::Value;

/// Moteur nécessaire à la construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    /// Node.js (téléchargé par Revel).
    Node,
    /// Hugo, édition « extended » (téléchargé par Revel).
    Hugo,
    /// Ruby et Bundler de l'ordinateur (Jekyll) : pas de version portable officielle, et des
    /// extensions à compiler.
    Ruby,
}

/// Gestionnaire de paquets d'un projet Node.js.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Manager {
    Npm,
    /// pnpm, lancé par Corepack (version du champ `packageManager` s'il y en a une).
    Pnpm,
    /// Yarn, lancé par Corepack ; `release` : Yarn livré dans le dépôt (`.yarn/releases/…`).
    Yarn {
        release: Option<String>,
    },
}

impl Manager {
    pub fn name(&self) -> &'static str {
        match self {
            Manager::Npm => "npm",
            Manager::Pnpm => "pnpm",
            Manager::Yarn { .. } => "Yarn",
        }
    }
}

/// Serveur qui produit les pages quand la construction n'écrit pas de pages HTML (rendu côté
/// serveur) : ses pages sont lues comme celles d'un site en ligne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Serve {
    /// Script de `package.json` (`preview`, `start`), avec ses arguments (`{port}` remplacé).
    Script { name: String, args: Vec<String> },
    /// Fichier lancé par Node (`.output/server/index.mjs`) ; port et adresse dans `PORT`, `HOST`.
    Entry(String),
}

/// Ce qui empêche la construction sur cet ordinateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Jekyll : Ruby et Bundler absents.
    Ruby,
    /// Hugo avec des modules : Go absent.
    Go,
    /// `package.json` sans script `build`.
    BuildScript,
}

/// Remarque sur la construction (bilan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Fichier de verrouillage d'un autre gestionnaire ignoré (`bun.lock`) : versions des
    /// dépendances résolues par npm.
    LockIgnored(String),
}

/// Recette de construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    /// Outil qui produit le site (« SvelteKit », « Hugo »…).
    pub tool: String,
    pub runtime: Runtime,
    /// Projet Node.js : gestionnaire de paquets (aussi pour un site Hugo qui a un `package.json`).
    pub manager: Option<Manager>,
    /// Node.js : nom du script ; Hugo, Jekyll : arguments de l'outil.
    pub build: Vec<String>,
    /// Dossiers où chercher les pages produites, dans l'ordre (`*` : un niveau quelconque).
    pub outputs: Vec<String>,
    /// Serveur d'aperçu quand aucun dossier produit n'a de page.
    pub serve: Option<Serve>,
    /// Fichiers du navigateur produits par la construction (dossier, préfixe d'adresse), copiés
    /// en entier après la lecture des pages servies.
    pub client_dirs: Vec<(String, String)>,
    pub missing: Option<Missing>,
    pub notes: Vec<Note>,
}

/// Comment servir le site construit.
#[derive(Clone, Copy)]
enum ServeKind {
    None,
    /// Script `preview` (Vite, Astro) avec ces arguments.
    Preview(&'static [&'static str]),
    /// Script `start` avec ces arguments (port dans `PORT` aussi).
    Start(&'static [&'static str]),
    /// Serveur Nitro (Nuxt, SolidStart).
    Entry(&'static str),
}

struct Framework {
    dep: &'static str,
    name: &'static str,
    outputs: &'static [&'static str],
    serve: ServeKind,
    client: &'static [(&'static str, &'static str)],
}

const VITE_PREVIEW: &[&str] = &["--port", "{port}", "--strictPort", "--host", "127.0.0.1"];
const GENERIC_OUT: &[&str] = &["dist", "build", "out", "public"];

/// Outils reconnus d'après les dépendances de `package.json`, du plus précis au plus général.
const FRAMEWORKS: &[Framework] = &[
    Framework {
        dep: "@sveltejs/kit",
        name: "SvelteKit",
        outputs: &["build", ".vercel/output/static", ".netlify/static"],
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[(".svelte-kit/output/client", "")],
    },
    Framework {
        dep: "next",
        name: "Next.js",
        outputs: &["out"],
        serve: ServeKind::Start(&["-p", "{port}", "-H", "127.0.0.1"]),
        client: &[(".next/static", "_next/static"), ("public", "")],
    },
    Framework {
        dep: "nuxt",
        name: "Nuxt",
        outputs: &[".output/public", "dist"],
        serve: ServeKind::Entry(".output/server/index.mjs"),
        client: &[(".output/public", "")],
    },
    Framework {
        dep: "astro",
        name: "Astro",
        outputs: &["dist"],
        serve: ServeKind::Preview(&["--port", "{port}", "--host", "127.0.0.1"]),
        client: &[("dist/client", "")],
    },
    Framework {
        dep: "gatsby",
        name: "Gatsby",
        outputs: &["public"],
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "@remix-run/react",
        name: "Remix",
        outputs: &["build/client"],
        serve: ServeKind::Start(&[]),
        client: &[("build/client", ""), ("public", "")],
    },
    Framework {
        dep: "@react-router/dev",
        name: "React Router",
        outputs: &["build/client"],
        serve: ServeKind::Start(&[]),
        client: &[("build/client", "")],
    },
    Framework {
        dep: "@angular/core",
        name: "Angular",
        outputs: &["dist/*/browser", "dist/*"],
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "@solidjs/start",
        name: "SolidStart",
        outputs: &[".output/public"],
        serve: ServeKind::Entry(".output/server/index.mjs"),
        client: &[(".output/public", "")],
    },
    Framework {
        dep: "@docusaurus/core",
        name: "Docusaurus",
        outputs: &["build"],
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "vitepress",
        name: "VitePress",
        outputs: &["docs/.vitepress/dist", ".vitepress/dist"],
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "react-scripts",
        name: "Create React App",
        outputs: &["build"],
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "svelte",
        name: "Svelte",
        outputs: GENERIC_OUT,
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[],
    },
    Framework {
        dep: "vue",
        name: "Vue",
        outputs: GENERIC_OUT,
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[],
    },
    Framework {
        dep: "react",
        name: "React",
        outputs: GENERIC_OUT,
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[],
    },
    Framework {
        dep: "solid-js",
        name: "Solid",
        outputs: GENERIC_OUT,
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[],
    },
    Framework {
        dep: "vite",
        name: "Vite",
        outputs: GENERIC_OUT,
        serve: ServeKind::Preview(VITE_PREVIEW),
        client: &[],
    },
    Framework {
        dep: "parcel",
        name: "Parcel",
        outputs: GENERIC_OUT,
        serve: ServeKind::None,
        client: &[],
    },
    Framework {
        dep: "webpack",
        name: "webpack",
        outputs: GENERIC_OUT,
        serve: ServeKind::None,
        client: &[],
    },
];

fn package_json(dir: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn has_dep(pkg: &Value, name: &str) -> bool {
    ["dependencies", "devDependencies"]
        .iter()
        .any(|k| pkg.get(k).and_then(|d| d.get(name)).is_some())
}

fn script<'a>(pkg: &'a Value, name: &str) -> Option<&'a str> {
    pkg.get("scripts")?.get(name)?.as_str()
}

/// Projet Hugo : fichier de configuration propre à Hugo, ou `config.*` avec des gabarits ou des
/// thèmes.
pub fn is_hugo(dir: &Path) -> bool {
    ["hugo.toml", "hugo.yaml", "hugo.yml", "hugo.json"]
        .iter()
        .any(|f| dir.join(f).is_file())
        || dir.join("config/_default").is_dir()
            && ["hugo.toml", "config.toml", "hugo.yaml", "config.yaml"]
                .iter()
                .any(|f| dir.join("config/_default").join(f).is_file())
        || ["config.toml", "config.yaml"]
            .iter()
            .any(|f| dir.join(f).is_file())
            && (dir.join("layouts").is_dir() || dir.join("themes").is_dir())
}

/// Projet Jekyll : `_config.yml` avec des gabarits, des articles ou un `Gemfile`.
pub fn is_jekyll(dir: &Path) -> bool {
    dir.join("_config.yml").is_file()
        && (dir.join("_layouts").is_dir()
            || dir.join("_posts").is_dir()
            || dir.join("Gemfile").is_file())
}

/// Nom de l'outil qui construit le site du dossier `dir`, s'il y en a un.
pub fn tool_name(dir: &Path) -> Option<String> {
    if let Some(pkg) = package_json(dir)
        && let Some(f) = FRAMEWORKS.iter().find(|f| has_dep(&pkg, f.dep))
        && !(is_hugo(dir) || is_jekyll(dir))
    {
        return Some(f.name.to_string());
    }
    if is_hugo(dir) {
        return Some("Hugo".into());
    }
    if is_jekyll(dir) {
        return Some("Jekyll".into());
    }
    None
}

/// Gestionnaire de paquets d'après `packageManager` puis les fichiers de verrouillage.
fn manager(dir: &Path, pkg: &Value, notes: &mut Vec<Note>) -> Manager {
    let yarn_release = || {
        std::fs::read_dir(dir.join(".yarn/releases"))
            .ok()?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".cjs") || n.ends_with(".js"))
            .max()
            .map(|n| format!(".yarn/releases/{n}"))
    };
    let declared = pkg
        .get("packageManager")
        .and_then(Value::as_str)
        .unwrap_or("");
    if declared.starts_with("pnpm@") {
        return Manager::Pnpm;
    }
    if declared.starts_with("yarn@") {
        return Manager::Yarn {
            release: yarn_release(),
        };
    }
    if declared.starts_with("npm@") {
        return Manager::Npm;
    }
    if dir.join("pnpm-lock.yaml").is_file() {
        return Manager::Pnpm;
    }
    if dir.join("yarn.lock").is_file() {
        return Manager::Yarn {
            release: yarn_release(),
        };
    }
    for lock in ["bun.lock", "bun.lockb"] {
        if dir.join(lock).is_file() && !dir.join("package-lock.json").is_file() {
            notes.push(Note::LockIgnored(lock.into()));
            break;
        }
    }
    Manager::Npm
}

/// Hugo avec des modules (`go.mod`, ou `[module]` avec des imports) : Go nécessaire.
fn hugo_needs_go(dir: &Path) -> bool {
    if dir.join("go.mod").is_file() {
        return true;
    }
    ["hugo.toml", "config.toml", "config/_default/hugo.toml"]
        .iter()
        .filter_map(|f| std::fs::read_to_string(dir.join(f)).ok())
        .any(|t| t.contains("[[module.imports]]") || t.contains("[[imports]]"))
}

/// Recette du dossier `dir` ; `has_cmd` : commande présente sur l'ordinateur (`bundle`, `go`).
pub fn detect(dir: &Path, has_cmd: &dyn Fn(&str) -> bool) -> Option<Recipe> {
    let pkg = package_json(dir);
    let mut notes = Vec::new();
    if is_hugo(dir) {
        let manager = pkg
            .as_ref()
            .filter(|p| {
                ["dependencies", "devDependencies"].iter().any(|k| {
                    p.get(k)
                        .and_then(Value::as_object)
                        .is_some_and(|o| !o.is_empty())
                })
            })
            .map(|p| manager(dir, p, &mut notes));
        let missing = (hugo_needs_go(dir) && !has_cmd("go")).then_some(Missing::Go);
        return Some(Recipe {
            tool: "Hugo".into(),
            runtime: Runtime::Hugo,
            manager,
            build: ["--minify", "--baseURL", "/", "--destination", "public"]
                .map(String::from)
                .to_vec(),
            outputs: vec!["public".into()],
            serve: None,
            client_dirs: Vec::new(),
            missing,
            notes,
        });
    }
    if is_jekyll(dir) {
        let ok = if dir.join("Gemfile").is_file() {
            has_cmd("bundle")
        } else {
            has_cmd("jekyll")
        };
        return Some(Recipe {
            tool: "Jekyll".into(),
            runtime: Runtime::Ruby,
            manager: None,
            build: ["build", "--destination", "_site", "--baseurl", ""]
                .map(String::from)
                .to_vec(),
            outputs: vec!["_site".into()],
            serve: None,
            client_dirs: Vec::new(),
            missing: (!ok).then_some(Missing::Ruby),
            notes,
        });
    }
    let pkg = pkg?;
    let fw = FRAMEWORKS.iter().find(|f| has_dep(&pkg, f.dep));
    let tool = fw.map_or("Node.js", |f| f.name).to_string();
    let manager = manager(dir, &pkg, &mut notes);
    let serve = match fw.map_or(ServeKind::Preview(VITE_PREVIEW), |f| f.serve) {
        ServeKind::None => None,
        ServeKind::Preview(args) => script(&pkg, "preview").map(|_| Serve::Script {
            name: "preview".into(),
            args: args.iter().map(|a| a.to_string()).collect(),
        }),
        ServeKind::Start(args) => script(&pkg, "start").map(|_| Serve::Script {
            name: "start".into(),
            args: args.iter().map(|a| a.to_string()).collect(),
        }),
        ServeKind::Entry(e) => Some(Serve::Entry(e.into())),
    };
    Some(Recipe {
        tool,
        runtime: Runtime::Node,
        manager: Some(manager),
        build: vec!["build".into()],
        outputs: fw
            .map_or(GENERIC_OUT, |f| f.outputs)
            .iter()
            .map(|s| s.to_string())
            .collect(),
        serve,
        client_dirs: fw
            .map(|f| {
                f.client
                    .iter()
                    .map(|(d, p)| (d.to_string(), p.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        missing: script(&pkg, "build")
            .is_none()
            .then_some(Missing::BuildScript),
        notes,
    })
}

/// Premier dossier produit qui contient une page d'accueil (`index.html`).
pub fn find_output(dir: &Path, outputs: &[String]) -> Option<std::path::PathBuf> {
    for o in outputs {
        if let Some((before, after)) = o.split_once("/*") {
            let Ok(rd) = std::fs::read_dir(dir.join(before)) else {
                continue;
            };
            let mut subs: Vec<_> = rd.flatten().filter(|e| e.path().is_dir()).collect();
            subs.sort_by_key(|e| e.file_name());
            for e in subs {
                let p = e.path().join(after.trim_start_matches('/'));
                if p.join("index.html").is_file() {
                    return Some(p);
                }
            }
        } else if dir.join(o).join("index.html").is_file() {
            return Some(dir.join(o));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for (p, c) in files {
            let f = d.path().join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, c).unwrap();
        }
        d
    }

    const NONE: &dyn Fn(&str) -> bool = &|_| false;

    #[test]
    fn sveltekit_avec_bun_lock() {
        let d = project(&[
            (
                "package.json",
                r#"{"scripts":{"build":"vite build","preview":"vite preview"},"devDependencies":{"@sveltejs/kit":"^2","vite":"^7"}}"#,
            ),
            ("bun.lock", "{}"),
        ]);
        let r = detect(d.path(), NONE).unwrap();
        assert_eq!(r.tool, "SvelteKit");
        assert_eq!(r.runtime, Runtime::Node);
        assert_eq!(r.manager, Some(Manager::Npm));
        assert_eq!(r.notes, vec![Note::LockIgnored("bun.lock".into())]);
        assert_eq!(r.build, vec!["build"]);
        assert!(
            matches!(&r.serve, Some(Serve::Script { name, args }) if name == "preview" && args.contains(&"{port}".to_string()))
        );
        assert_eq!(r.client_dirs[0].0, ".svelte-kit/output/client");
        assert_eq!(r.missing, None);
        assert_eq!(tool_name(d.path()).as_deref(), Some("SvelteKit"));
    }

    #[test]
    fn gestionnaires_de_paquets() {
        let pnpm = project(&[
            (
                "package.json",
                r#"{"scripts":{"build":"astro build"},"dependencies":{"astro":"5"}}"#,
            ),
            ("pnpm-lock.yaml", ""),
        ]);
        assert_eq!(
            detect(pnpm.path(), NONE).unwrap().manager,
            Some(Manager::Pnpm)
        );
        let berry = project(&[
            (
                "package.json",
                r#"{"packageManager":"yarn@4.5.0","scripts":{"build":"next build","start":"next start"},"dependencies":{"next":"15"}}"#,
            ),
            (".yarn/releases/yarn-4.5.0.cjs", ""),
        ]);
        let r = detect(berry.path(), NONE).unwrap();
        assert_eq!(r.tool, "Next.js");
        assert_eq!(
            r.manager,
            Some(Manager::Yarn {
                release: Some(".yarn/releases/yarn-4.5.0.cjs".into())
            })
        );
        assert!(matches!(&r.serve, Some(Serve::Script { name, .. }) if name == "start"));
        // Sans script de construction : signalé.
        let nobuild = project(&[("package.json", r#"{"dependencies":{"react":"19"}}"#)]);
        assert_eq!(
            detect(nobuild.path(), NONE).unwrap().missing,
            Some(Missing::BuildScript)
        );
    }

    #[test]
    fn hugo_et_jekyll() {
        let hugo = project(&[
            ("hugo.toml", "baseURL = 'https://exemple.fr/'"),
            ("content/_index.md", ""),
        ]);
        let r = detect(hugo.path(), NONE).unwrap();
        assert_eq!((r.tool.as_str(), r.runtime), ("Hugo", Runtime::Hugo));
        assert!(r.build.contains(&"--baseURL".to_string()));
        assert_eq!(r.manager, None);
        let modules = project(&[
            ("hugo.toml", "[[module.imports]]\npath = 'github.com/x/y'"),
            ("go.mod", ""),
        ]);
        assert_eq!(
            detect(modules.path(), NONE).unwrap().missing,
            Some(Missing::Go)
        );
        assert_eq!(
            detect(modules.path(), &|c| c == "go").unwrap().missing,
            None
        );
        // Thème qui compile ses styles avec PostCSS : Node.js aussi.
        let postcss = project(&[
            ("config.toml", ""),
            ("layouts/index.html", ""),
            (
                "package.json",
                r#"{"devDependencies":{"postcss-cli":"11"}}"#,
            ),
        ]);
        assert_eq!(
            detect(postcss.path(), NONE).unwrap().manager,
            Some(Manager::Npm)
        );
        let jekyll = project(&[
            ("_config.yml", "title: x"),
            ("Gemfile", ""),
            ("_posts/a.md", ""),
        ]);
        let r = detect(jekyll.path(), NONE).unwrap();
        assert_eq!(
            (r.tool.as_str(), r.missing),
            ("Jekyll", Some(Missing::Ruby))
        );
        assert_eq!(
            detect(jekyll.path(), &|c| c == "bundle").unwrap().missing,
            None
        );
    }

    #[test]
    fn dossier_produit() {
        let d = project(&[("dist/app/browser/index.html", ""), ("build/x.js", "")]);
        let outs: Vec<String> = ["build", "dist/*/browser"].map(String::from).to_vec();
        assert_eq!(
            find_output(d.path(), &outs),
            Some(d.path().join("dist/app/browser"))
        );
        assert_eq!(find_output(d.path(), &["build".into()]), None);
    }
}
