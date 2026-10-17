//! Polices du site (3.9, D-24) : familles déclarées par des règles `@font-face` de la feuille de
//! style, fichiers copiés dans le dossier des médias (`fonts/<famille>/`) et servis par le site :
//! aucune requête des visiteurs vers Google.
//!
//! Ajout depuis Google Fonts : feuille de l'API CSS2 lue, fichiers WOFF2 des jeux de caractères
//! utiles téléchargés une fois. Ajout et retrait sont des commandes de l'historique : les fichiers
//! sont déplacés (`MoveFiles`) entre leur place et `.revel/`, donc annulables.

use crate::{Core, CoreError, Result};
use base64::Engine as _;
use revel_edit::{Command, EditState};
use revel_model::{Site, StyleItem};
use revel_protocol::{FontFace, FontFamily, FontUpload, GoogleFont};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Agent du navigateur annoncé à Google Fonts : un navigateur récent reçoit des fichiers WOFF2.
const BROWSER: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const CATALOG: &str = "https://fonts.google.com/metadata/fonts";
const CATALOG_FILE: &str = "google-fonts.json";

/// Règle `@font-face` lue : position dans la feuille, famille, graisse, style, adresses.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Face {
    pub index: usize,
    pub family: String,
    pub weight: String,
    pub style: String,
    pub urls: Vec<String>,
}

/// Déclarations d'un bloc (`;` ignorés dans les parenthèses et les guillemets).
fn decls(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (mut depth, mut quote, mut start) = (0i32, None::<char>, 0);
    let b = body.as_bytes();
    for (i, c) in body.char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, ';') if depth <= 0 => {
                push_decl(&body[start..i], &mut out);
                start = i + 1;
            }
            _ => {}
        }
    }
    let _ = b;
    push_decl(&body[start..], &mut out);
    out
}

fn push_decl(d: &str, out: &mut Vec<(String, String)>) {
    if let Some((k, v)) = d.split_once(':') {
        let k = k.trim().to_ascii_lowercase();
        if !k.is_empty() {
            out.push((k, v.trim().to_string()));
        }
    }
}

/// Première famille d'une valeur `font-family`, sans guillemets.
pub(crate) fn first_family(v: &str) -> String {
    v.split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\''])
        .trim()
        .to_string()
}

/// Adresses `url(...)` d'une valeur `src`.
fn urls(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("url(") {
        let after = &rest[i + 4..];
        let Some(j) = after.find(')') else { break };
        let u = after[..j].trim().trim_matches(['"', '\'']).to_string();
        if !u.is_empty() {
            out.push(u);
        }
        rest = &after[j + 1..];
    }
    out
}

/// Règles `@font-face` de la feuille de style du site.
pub(crate) fn faces(site: &Site) -> Vec<Face> {
    let Some(sheet) = &site.stylesheet else {
        return Vec::new();
    };
    sheet
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let StyleItem::Raw { css } = item else {
                return None;
            };
            let t = css.trim_start();
            if !t
                .get(..10)
                .is_some_and(|h| h.eq_ignore_ascii_case("@font-face"))
            {
                return None;
            }
            let body = t.split_once('{')?.1.rsplit_once('}')?.0;
            let d = decls(body);
            let get = |k: &str| d.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
            let family = first_family(&get("font-family")?);
            Some(Face {
                index,
                family,
                weight: get("font-weight").unwrap_or_else(|| "400".into()),
                style: get("font-style").unwrap_or_else(|| "normal".into()),
                urls: get("src").map(|s| urls(&s)).unwrap_or_default(),
            })
        })
        .collect()
}

/// Familles du site, avec leurs graisses et le nombre de règles qui les utilisent.
pub(crate) fn families(site: &Site, size_of: impl Fn(&str) -> u64) -> Vec<FontFamily> {
    let mut out: Vec<FontFamily> = Vec::new();
    for f in faces(site) {
        let fam = match out
            .iter_mut()
            .find(|x| x.family.eq_ignore_ascii_case(&f.family))
        {
            Some(x) => x,
            None => {
                out.push(FontFamily {
                    family: f.family.clone(),
                    faces: Vec::new(),
                    uses: 0,
                    size: 0,
                });
                out.last_mut().expect("ajoutée")
            }
        };
        fam.size += f.urls.iter().map(|u| size_of(u)).sum::<u64>();
        match fam
            .faces
            .iter_mut()
            .find(|x| x.weight == f.weight && x.style == f.style)
        {
            Some(x) => x.files.extend(f.urls),
            None => fam.faces.push(FontFace {
                weight: f.weight,
                style: f.style,
                files: f.urls,
            }),
        }
    }
    // Utilisations : `font-family`, `font` et variables CSS (`--police: "Inter", sans-serif`)
    // qui nomment la famille.
    if let Some(sheet) = &site.stylesheet {
        for item in &sheet.items {
            if let StyleItem::Rule(r) = item {
                for d in r.decls.iter().filter(|d| {
                    d.prop == "font-family" || d.prop == "font" || d.prop.starts_with("--")
                }) {
                    for f in out.iter_mut() {
                        if names_family(&d.value, &f.family) {
                            f.uses += 1;
                        }
                    }
                }
            }
        }
    }
    for f in &mut out {
        f.faces.sort_by(|a, b| {
            (weight_num(&a.weight), a.style != "normal")
                .cmp(&(weight_num(&b.weight), b.style != "normal"))
        });
    }
    out
}

/// Valeur qui nomme une famille (`"Inter", sans-serif`, `700 1rem/1.2 Inter`).
fn names_family(value: &str, family: &str) -> bool {
    let name = family.to_ascii_lowercase();
    value.to_ascii_lowercase().split(',').any(|part| {
        let p = part.trim();
        p.trim_matches(['"', '\'']).trim() == name
            || p.ends_with(&format!(" {name}"))
            || p.contains(&format!("\"{name}\""))
            || p.contains(&format!("'{name}'"))
    })
}

fn weight_num(w: &str) -> u32 {
    w.split_whitespace()
        .next()
        .and_then(|x| x.parse().ok())
        .unwrap_or(match w {
            "bold" => 700,
            _ => 400,
        })
}

/// Position d'insertion des nouvelles règles : après les `@import` et `@charset` du début (sinon
/// ils ne seraient plus pris en compte).
fn insert_at(site: &Site) -> usize {
    site.stylesheet
        .as_ref()
        .map(|s| {
            s.items
                .iter()
                .take_while(|i| match i {
                    StyleItem::Raw { css } => {
                        let t = css.trim_start().to_ascii_lowercase();
                        t.starts_with("@import") || t.starts_with("@charset")
                    }
                    StyleItem::Comment { .. } => true,
                    StyleItem::Rule(_) => false,
                })
                .count()
        })
        .unwrap_or(0)
}

/// Règle `@font-face` d'un fichier du site.
fn face_css(
    family: &str,
    weight: &str,
    italic: bool,
    url: &str,
    format: &str,
    range: Option<&str>,
) -> String {
    let range = range
        .map(|r| format!("\n  unicode-range: {r};"))
        .unwrap_or_default();
    format!(
        "@font-face {{\n  font-family: \"{}\";\n  font-style: {};\n  font-weight: {weight};\n  font-display: swap;\n  src: url(\"{url}\") format(\"{format}\");{range}\n}}",
        family.replace('"', ""),
        if italic { "italic" } else { "normal" },
    )
}

fn agent(timeout: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(BROWSER)
        .timeout_global(Some(Duration::from_secs(timeout)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn net(e: ureq::Error) -> CoreError {
    CoreError::Font(e.to_string())
}

/// Jeu de caractères d'un fichier découpé à la manière de Google Fonts et Fontsource
/// (`montserrat-latin-ext-wght-normal.woff2`) : plage de caractères qu'il couvre, pour que le
/// navigateur ne charge que les fichiers utiles et que les jeux ne se masquent pas entre eux.
fn subset_range(name: &str) -> Option<&'static str> {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let words: Vec<String> = stem
        .split(['-', '_', '.', ' '])
        .map(str::to_ascii_lowercase)
        .collect();
    let has = |a: &str, b: Option<&str>| {
        words
            .windows(2)
            .any(|w| w[0] == a && Some(w[1].as_str()) == b)
            || (b.is_none() && words.iter().any(|w| w == a))
    };
    // Plages de Fontsource (identiques à celles de Google Fonts).
    let table: [(&str, Option<&str>, &str); 7] = [
        (
            "latin",
            Some("ext"),
            "U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C0, U+2113, U+2C60-2C7F, U+A720-A7FF",
        ),
        (
            "cyrillic",
            Some("ext"),
            "U+0460-052F, U+1C80-1C8A, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F",
        ),
        ("greek", Some("ext"), "U+1F00-1FFF"),
        (
            "latin",
            None,
            "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD",
        ),
        (
            "cyrillic",
            None,
            "U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116",
        ),
        (
            "greek",
            None,
            "U+0370-0377, U+037A-037F, U+0384-038A, U+038C, U+038E-03A1, U+03A3-03FF",
        ),
        (
            "vietnamese",
            None,
            "U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+0300-0301, U+0303-0304, U+0308-0309, U+0323, U+0329, U+1EA0-1EF9, U+20AB",
        ),
    ];
    table
        .iter()
        .find(|(a, b, _)| has(a, *b))
        .map(|(_, _, r)| *r)
}

/// Jeux de caractères utiles aux langues du site.
fn subsets(site: &Site) -> Vec<&'static str> {
    let mut out = vec!["latin", "latin-ext"];
    let codes = std::iter::once(site.default_locale.as_str())
        .chain(site.locales.iter().map(|l| l.code.as_str()));
    for c in codes {
        let base = c.split(['-', '_']).next().unwrap_or(c).to_ascii_lowercase();
        let extra: &[&'static str] = match base.as_str() {
            "ru" | "uk" | "bg" | "sr" | "mk" | "be" | "kk" => &["cyrillic", "cyrillic-ext"],
            "el" => &["greek", "greek-ext"],
            "vi" => &["vietnamese"],
            _ => &[],
        };
        for e in extra {
            if !out.contains(e) {
                out.push(e);
            }
        }
    }
    out
}

/// Blocs de la feuille de Google Fonts : (jeu de caractères, style, graisse, adresse, plage).
fn google_blocks(css: &str) -> Vec<(String, bool, String, String, Option<String>)> {
    let mut out = Vec::new();
    let mut label = String::new();
    let mut rest = css;
    loop {
        let c = rest.find("/*");
        let f = rest.find("@font-face");
        match (c, f) {
            (Some(ci), Some(fi)) if ci < fi => {
                let end = rest[ci..]
                    .find("*/")
                    .map(|e| ci + e + 2)
                    .unwrap_or(rest.len());
                label = rest[ci + 2..end.saturating_sub(2)].trim().to_string();
                rest = &rest[end..];
            }
            (_, Some(fi)) => {
                let Some(open) = rest[fi..].find('{') else {
                    break;
                };
                let Some(close) = rest[fi..].find('}') else {
                    break;
                };
                let body = &rest[fi + open + 1..fi + close];
                let d = decls(body);
                let get = |k: &str| d.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
                if let Some(u) = get("src").and_then(|s| urls(&s).into_iter().next()) {
                    out.push((
                        label.clone(),
                        get("font-style").as_deref() == Some("italic"),
                        get("font-weight").unwrap_or_else(|| "400".into()),
                        u,
                        get("unicode-range"),
                    ));
                }
                rest = &rest[fi + close + 1..];
            }
            _ => break,
        }
    }
    out
}

/// Adresse de l'API CSS2 pour une famille et des variantes (`400`, `700i`).
fn css2_url(family: &str, variants: &[String]) -> String {
    let mut v: Vec<(u8, u32)> = variants
        .iter()
        .map(|x| {
            let italic = x.ends_with('i');
            (
                u8::from(italic),
                x.trim_end_matches('i').parse().unwrap_or(400),
            )
        })
        .collect();
    v.sort();
    v.dedup();
    let fam = family.trim().replace(' ', "+");
    let spec = if v.iter().any(|(i, _)| *i == 1) {
        let list: Vec<String> = v.iter().map(|(i, w)| format!("{i},{w}")).collect();
        format!(":ital,wght@{}", list.join(";"))
    } else {
        let list: Vec<String> = v.iter().map(|(_, w)| w.to_string()).collect();
        format!(":wght@{}", list.join(";"))
    };
    format!("https://fonts.googleapis.com/css2?family={fam}{spec}&display=swap")
}

/// Chemin relatif au dossier du site.
fn rel(dir: &Path, p: &Path) -> Option<String> {
    p.strip_prefix(dir)
        .ok()
        .map(|r| r.to_string_lossy().replace('\\', "/"))
}

/// Format CSS d'un fichier de police (contenu vérifié).
fn font_format(name: &str, data: &[u8]) -> Option<&'static str> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
    let magic = data.get(..4)?;
    match ext.as_str() {
        "woff2" if magic == b"wOF2" => Some("woff2"),
        "woff" if magic == b"wOFF" => Some("woff"),
        "ttf" if magic == [0, 1, 0, 0] || magic == b"true" => Some("truetype"),
        "otf" if magic == b"OTTO" => Some("opentype"),
        _ => None,
    }
}

impl Core {
    /// Polices du site ouvert.
    pub fn fonts(&self, path: &Path) -> Result<Vec<FontFamily>> {
        self.with_project(path, |p| {
            Ok(families(&p.site, |u| {
                p.source_of_url(u)
                    .and_then(|f| std::fs::metadata(f).ok())
                    .map(|m| m.len())
                    .unwrap_or(0)
            }))
        })
    }

    /// Catalogue de Google Fonts (familles les plus utilisées d'abord) ; gardé dans les réglages
    /// pour la consultation sans connexion.
    pub fn fonts_google(&self) -> Result<Vec<GoogleFont>> {
        let cache = self.config_dir().join(CATALOG_FILE);
        let fresh = std::fs::metadata(&cache)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(30 * 24 * 3600));
        if !fresh {
            let fetched = agent(60).get(CATALOG).call().map_err(net).and_then(|r| {
                r.into_body()
                    .with_config()
                    .limit(64 * 1024 * 1024)
                    .read_to_string()
                    .map_err(net)
            });
            match fetched {
                Ok(text) => {
                    let list = parse_catalog(&text)?;
                    if let Some(d) = cache.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let _ = std::fs::write(&cache, serde_json::to_vec(&list).unwrap_or_default());
                    return Ok(list);
                }
                Err(e) if !cache.is_file() => return Err(e),
                Err(_) => {}
            }
        }
        let text = std::fs::read(&cache)?;
        serde_json::from_slice(&text).map_err(|e| CoreError::Font(e.to_string()))
    }

    /// Ajoute une famille de Google Fonts : fichiers WOFF2 des variantes choisies et des jeux de
    /// caractères utiles aux langues du site téléchargés dans le site, règles `@font-face`
    /// ajoutées (une étape d'annulation).
    pub fn fonts_add_google(
        &self,
        path: &Path,
        family: &str,
        variants: &[String],
    ) -> Result<EditState> {
        if variants.is_empty() {
            return Err(CoreError::Font("aucune graisse choisie".into()));
        }
        let site = self.edit_site(path)?;
        if faces(&site)
            .iter()
            .any(|f| f.family.eq_ignore_ascii_case(family))
        {
            return Err(CoreError::Font(format!("{family} est déjà dans le site")));
        }
        let wanted = subsets(&site);
        let a = agent(120);
        let css = a
            .get(&css2_url(family, variants))
            .call()
            .map_err(net)?
            .into_body()
            .read_to_string()
            .map_err(net)?;
        let blocks: Vec<_> = google_blocks(&css)
            .into_iter()
            .filter(|b| wanted.contains(&b.0.as_str()))
            .collect();
        if blocks.is_empty() {
            return Err(CoreError::Font(format!("{family} : aucun fichier trouvé")));
        }
        let (media, prefix) = self.media_dir(path)?;
        let slug = revel_model::slugify(family);
        let stage = path
            .join(revel_project::CACHE_DIR)
            .join("tmp")
            .join("fonts")
            .join(&slug);
        let _ = std::fs::remove_dir_all(&stage);
        std::fs::create_dir_all(&stage)?;
        let dest = media.join("fonts").join(&slug);
        let mut moves = Vec::new();
        let mut items = Vec::new();
        for (subset, italic, weight, url, range) in &blocks {
            let name = format!(
                "{slug}-{}{}-{subset}.woff2",
                weight.replace(' ', "-"),
                if *italic { "i" } else { "" }
            );
            let mut data = Vec::new();
            use std::io::Read;
            a.get(url)
                .call()
                .map_err(net)?
                .into_body()
                .into_reader()
                .take(20 * 1024 * 1024)
                .read_to_end(&mut data)?;
            if font_format(&name, &data).is_none() {
                return Err(CoreError::Font(format!("fichier inattendu : {url}")));
            }
            let staged = stage.join(&name);
            std::fs::write(&staged, &data)?;
            let target = dest.join(&name);
            if !target.exists() {
                moves.push((
                    rel(path, &staged).unwrap_or_default(),
                    rel(path, &target).unwrap_or_default(),
                ));
            }
            let web = format!("{prefix}/fonts/{slug}/{name}");
            items.push(face_css(
                family,
                weight,
                *italic,
                &web,
                "woff2",
                range.as_deref(),
            ));
        }
        self.apply_fonts(path, &site, moves, items)
    }

    /// Ajoute des fichiers de police importés (graisse et style choisis pour chacun).
    pub fn fonts_add_files(
        &self,
        path: &Path,
        family: &str,
        files: &[FontUpload],
    ) -> Result<EditState> {
        let family = family.trim();
        if family.is_empty() || files.is_empty() {
            return Err(CoreError::Font("famille ou fichiers absents".into()));
        }
        let site = self.edit_site(path)?;
        let (media, prefix) = self.media_dir(path)?;
        let slug = revel_model::slugify(family);
        let stage = path
            .join(revel_project::CACHE_DIR)
            .join("tmp")
            .join("fonts")
            .join(&slug);
        std::fs::create_dir_all(&stage)?;
        let dest = media.join("fonts").join(&slug);
        let mut moves = Vec::new();
        let mut items = Vec::new();
        for f in files {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&f.data)
                .map_err(|e| CoreError::Font(e.to_string()))?;
            let format = font_format(&f.name, &data)
                .ok_or_else(|| CoreError::Font(format!("fichier de police refusé : {}", f.name)))?;
            let (stem, ext) = f.name.rsplit_once('.').unwrap_or((&f.name, "woff2"));
            let base = revel_model::slugify(stem);
            let ext = ext.to_ascii_lowercase();
            let mut name = format!("{base}.{ext}");
            let mut n = 2;
            while dest.join(&name).exists()
                || moves
                    .iter()
                    .any(|(_, t): &(String, String)| t.ends_with(&format!("/{name}")))
            {
                name = format!("{base}-{n}.{ext}");
                n += 1;
            }
            let staged = stage.join(&name);
            std::fs::write(&staged, &data)?;
            moves.push((
                rel(path, &staged).unwrap_or_default(),
                rel(path, &dest.join(&name)).unwrap_or_default(),
            ));
            let weight = if f.weight.trim().is_empty() {
                "400"
            } else {
                f.weight.trim()
            };
            let web = format!("{prefix}/fonts/{slug}/{name}");
            items.push(face_css(
                family,
                weight,
                f.italic,
                &web,
                format,
                subset_range(&f.name),
            ));
        }
        self.apply_fonts(path, &site, moves, items)
    }

    /// Ajout : fichiers mis en place et règles insérées, en une étape d'annulation.
    fn apply_fonts(
        &self,
        path: &Path,
        site: &Site,
        moves: Vec<(String, String)>,
        items: Vec<String>,
    ) -> Result<EditState> {
        let at = insert_at(site);
        let mut commands = Vec::new();
        if !moves.is_empty() {
            commands.push(Command::MoveFiles { moves });
        }
        commands.extend(
            items
                .into_iter()
                .enumerate()
                .map(|(i, css)| Command::InsertStyle {
                    index: at + i,
                    item: StyleItem::Raw { css },
                }),
        );
        self.edit_apply(path, Command::Batch { commands }, false)
    }

    /// Retire une famille : règles `@font-face` retirées, fichiers rangés dans `.revel/corbeille`
    /// (une étape d'annulation : tout revient).
    pub fn fonts_remove(&self, path: &Path, family: &str) -> Result<EditState> {
        let site = self.edit_site(path)?;
        let all = faces(&site);
        let (gone, kept): (Vec<&Face>, Vec<&Face>) = all
            .iter()
            .partition(|f| f.family.eq_ignore_ascii_case(family));
        if gone.is_empty() {
            return Err(CoreError::Font(format!("{family} n'est pas dans le site")));
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let trash: PathBuf = Path::new(revel_project::CACHE_DIR)
            .join("corbeille")
            .join(format!("polices-{stamp}"));
        let moves: Vec<(String, String)> = self.with_project(path, |p| {
            let mut moves = Vec::new();
            for u in gone.iter().flat_map(|f| f.urls.iter()) {
                // Fichier encore utilisé par une autre famille : gardé.
                if kept.iter().any(|k| k.urls.contains(u)) {
                    continue;
                }
                if let Some(file) = p.source_of_url(u)
                    && let Some(r) = rel(&p.dir, &file)
                    && !moves.iter().any(|(f, _): &(String, String)| *f == r)
                {
                    let to = trash.join(&r).to_string_lossy().replace('\\', "/");
                    moves.push((r, to));
                }
            }
            Ok(moves)
        })?;
        let mut commands: Vec<Command> = Vec::new();
        let mut idx: Vec<usize> = gone.iter().map(|f| f.index).collect();
        idx.sort_unstable_by(|a, b| b.cmp(a));
        commands.extend(
            idx.into_iter()
                .map(|index| Command::RemoveStyleAt { index }),
        );
        if !moves.is_empty() {
            commands.push(Command::MoveFiles { moves });
        }
        self.edit_apply(path, Command::Batch { commands }, false)
    }
}

/// Catalogue de Google Fonts : familles, catégorie, variantes (`400`, `400i`).
fn parse_catalog(text: &str) -> Result<Vec<GoogleFont>> {
    let text = text.trim_start_matches(")]}'").trim_start();
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| CoreError::Font(e.to_string()))?;
    let mut list: Vec<(i64, GoogleFont)> = v
        .get("familyMetadataList")
        .and_then(|l| l.as_array())
        .map(|l| {
            l.iter()
                .filter_map(|f| {
                    let family = f.get("family")?.as_str()?.to_string();
                    let fonts = f.get("fonts")?;
                    let mut variants: Vec<String> = match fonts {
                        serde_json::Value::Object(m) => m.keys().cloned().collect(),
                        serde_json::Value::Array(a) => a
                            .iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect(),
                        _ => Vec::new(),
                    };
                    variants.sort_by_key(|x| {
                        (
                            x.ends_with('i'),
                            x.trim_end_matches('i').parse::<u32>().unwrap_or(400),
                        )
                    });
                    Some((
                        f.get("popularity")
                            .and_then(|p| p.as_i64())
                            .unwrap_or(i64::MAX),
                        GoogleFont {
                            family,
                            category: f
                                .get("category")
                                .and_then(|c| c.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            variants,
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|(p, _)| *p);
    Ok(list.into_iter().map(|(_, f)| f).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regles_font_face() {
        let d = decls(
            " font-family: 'Inter Tight'; src: url(\"data:font/woff2;base64,AA==\") format('woff2'), url(/f.woff); font-weight: 700 ",
        );
        assert_eq!(d.len(), 3);
        assert_eq!(first_family(&d[0].1), "Inter Tight");
        assert_eq!(
            urls(&d[1].1),
            vec!["data:font/woff2;base64,AA==", "/f.woff"]
        );
        assert_eq!(
            css2_url("Open Sans", &["700".into(), "400".into(), "400i".into()]),
            "https://fonts.googleapis.com/css2?family=Open+Sans:ital,wght@0,400;0,700;1,400&display=swap"
        );
        assert_eq!(
            css2_url("Inter", &["400".into()]),
            "https://fonts.googleapis.com/css2?family=Inter:wght@400&display=swap"
        );
        let css = "/* cyrillic */\n@font-face {\n  font-family: 'Inter';\n  font-style: italic;\n  font-weight: 400;\n  src: url(https://x/a.woff2) format('woff2');\n  unicode-range: U+0400-045F;\n}\n/* latin */\n@font-face {\n  font-family: 'Inter';\n  font-style: normal;\n  font-weight: 700;\n  src: url(https://x/b.woff2) format('woff2');\n  unicode-range: U+0000-00FF;\n}\n";
        let b = google_blocks(css);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].0, "cyrillic");
        assert!(b[0].1);
        assert_eq!(
            (b[1].0.as_str(), b[1].2.as_str(), b[1].3.as_str()),
            ("latin", "700", "https://x/b.woff2")
        );
        assert_eq!(b[1].4.as_deref(), Some("U+0000-00FF"));
        assert!(
            subset_range("montserrat-latin-ext-wght-normal.woff2")
                .unwrap()
                .starts_with("U+0100-02BA")
        );
        assert!(
            subset_range("montserrat-latin-wght-italic.woff2")
                .unwrap()
                .starts_with("U+0000-00FF")
        );
        assert_eq!(subset_range("inter_greek-ext.woff2"), Some("U+1F00-1FFF"));
        assert_eq!(subset_range("Inter-Bold.woff2"), None);
        assert_eq!(subset_range("Latinum-Bold.woff2"), None);
        assert_eq!(font_format("a.woff2", b"wOF2xxxx"), Some("woff2"));
        assert_eq!(font_format("a.woff2", b"wOFFxxxx"), None);
        assert_eq!(font_format("a.otf", b"OTTOxxxx"), Some("opentype"));
        assert!(names_family(
            "\"Montserrat Variable\", sans-serif",
            "Montserrat Variable"
        ));
        assert!(names_family("700 1rem/1.2 Inter, sans-serif", "Inter"));
        assert!(!names_family("Interstate, sans-serif", "Inter"));
    }
}
