//! Formulaires du site (5.8) reçus par l'hébergement, sans service extérieur : chaque
//! `<form data-revel-form="nom">` envoie ses champs à `/revel-form.php`, produit à la génération,
//! qui les transmet par courriel (fonction `mail()` de PHP) à l'adresse réglée dans
//! Paramètres › Formulaires. Champ piège contre les robots, limite d'envois par adresse IP ;
//! envoi sans recharger la page quand JavaScript est disponible, page de confirmation sinon.

use crate::OutputFile;
use revel_model::Site;

/// Fichier PHP qui reçoit les formulaires.
pub const HANDLER: &str = "revel-form.php";

/// Envois au plus par adresse IP et par heure.
const MAX_PER_HOUR: u32 = 10;

/// Envoi sans rechargement : réponse JSON du script, message dans `[data-revel-form-status]`.
const SCRIPT: &str = r#"<script>document.addEventListener("submit",function(e){var f=e.target;if(!f.matches||!f.matches("form[data-revel-form]")||!window.fetch)return;e.preventDefault();var s=f.querySelector("[data-revel-form-status]");if(!s){s=document.createElement("p");s.setAttribute("role","status");s.setAttribute("data-revel-form-status","");f.appendChild(s)}var b=f.querySelector("[type=submit]");if(b)b.disabled=true;fetch(f.action,{method:"POST",body:new FormData(f),headers:{Accept:"application/json"}}).then(function(r){return r.json()}).then(function(d){s.textContent=d.message||"";if(d.ok){f.reset();f.classList.add("is-sent")}}).catch(function(){s.textContent=f.getAttribute("data-revel-error")||""}).then(function(){if(b)b.disabled=false})});</script>"#;

/// Messages du script selon la langue du site.
struct Texts {
    ok: &'static str,
    invalid: &'static str,
    limit: &'static str,
    fail: &'static str,
    subject: &'static str,
    network: &'static str,
}

fn texts(locale: &str) -> Texts {
    if locale.starts_with("fr") {
        Texts {
            ok: "Merci, votre message a bien été envoyé.",
            invalid: "Merci de remplir le formulaire.",
            limit: "Trop de messages envoyés depuis votre connexion. Réessayez dans une heure.",
            fail: "Le message n’a pas pu être envoyé. Réessayez plus tard.",
            subject: "Nouveau message depuis",
            network: "Le message n’a pas pu être envoyé. Vérifiez votre connexion.",
        }
    } else {
        Texts {
            ok: "Thank you, your message has been sent.",
            invalid: "Please fill in the form.",
            limit: "Too many messages sent from your connection. Please try again in an hour.",
            fail: "The message could not be sent. Please try again later.",
            subject: "New message from",
            network: "The message could not be sent. Please check your connection.",
        }
    }
}

/// Valeur d'un attribut dans une balise (`None` : absent).
fn attr_value(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before = lower[..at].chars().last();
        let after = lower[at + name.len()..].chars().next();
        if !before.is_some_and(|c| c.is_whitespace())
            || !after.is_none_or(|c| c == '=' || c.is_whitespace() || c == '>' || c == '/')
        {
            continue;
        }
        let rest = tag[at + name.len()..].trim_start();
        let Some(v) = rest.strip_prefix('=') else {
            return Some(String::new());
        };
        let v = v.trim_start();
        return Some(match v.chars().next() {
            Some(q @ ('"' | '\'')) => v[1..].split(q).next().unwrap_or_default().to_string(),
            _ => v
                .split(|c: char| c.is_whitespace() || c == '>')
                .next()
                .unwrap_or_default()
                .to_string(),
        });
    }
    None
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

/// Formulaires de la page reliés au script (`action`, `method`, champ piège, nom), et script
/// d'envoi ajouté avant `</body>`. `scripts` faux : canevas de l'éditeur (rien n'est ajouté).
/// Renvoie la page et le nombre de formulaires trouvés.
pub(crate) fn wire(site: &Site, html: String, scripts: bool) -> (String, usize) {
    let lower = html.to_ascii_lowercase();
    if !lower.contains("data-revel-form") {
        return (html, 0);
    }
    let error = texts(&site.default_locale).network;
    let mut out = String::with_capacity(html.len() + 512);
    let mut rest = 0;
    let mut count = 0;
    let mut search = 0;
    while let Some(i) = lower[search..].find("<form") {
        let start = search + i;
        let Some(end) = lower[start..].find('>').map(|e| start + e) else {
            break;
        };
        search = end + 1;
        let tag = &html[start..end];
        let Some(name) = attr_value(tag, "data-revel-form") else {
            continue;
        };
        count += 1;
        let mut open = tag.to_string();
        let action = attr_value(tag, "action");
        if action
            .as_deref()
            .is_none_or(|a| a.trim().is_empty() || a.trim() == "#")
        {
            // `action=""` ou `action="#"` : remplacé.
            if action.is_some() {
                open = remove_attr(&open, "action");
            }
            open.push_str(&format!(" action=\"/{HANDLER}\""));
        }
        if attr_value(tag, "method").is_none() {
            open.push_str(" method=\"post\"");
        }
        if attr_value(tag, "data-revel-error").is_none() {
            open.push_str(&format!(" data-revel-error=\"{}\"", escape_attr(error)));
        }
        out.push_str(&html[rest..start]);
        out.push_str(&open);
        out.push('>');
        out.push_str(&format!(
            "<input type=\"text\" name=\"_revel_hp\" value=\"\" tabindex=\"-1\" autocomplete=\"off\" aria-hidden=\"true\" style=\"position:absolute;left:-10000px;width:1px;height:1px;overflow:hidden\"><input type=\"hidden\" name=\"_revel_form\" value=\"{}\">",
            escape_attr(&name)
        ));
        rest = end + 1;
    }
    out.push_str(&html[rest..]);
    if count > 0 && scripts {
        out = crate::insert_before(&out, "</body>", SCRIPT, true);
    }
    (out, count)
}

/// Retire un attribut d'une balise ouvrante (sans le `>`).
fn remove_attr(tag: &str, name: &str) -> String {
    let lower = tag.to_ascii_lowercase();
    let Some(i) = lower.find(&format!(" {name}")) else {
        return tag.to_string();
    };
    let after = &tag[i + 1 + name.len()..];
    let trimmed = after.trim_start();
    let Some(v) = trimmed.strip_prefix('=') else {
        return format!("{}{}", &tag[..i], after);
    };
    let v = v.trim_start();
    let consumed = match v.chars().next() {
        Some(q @ ('"' | '\'')) => v[1..].find(q).map(|e| e + 2).unwrap_or(v.len()),
        _ => v.find(char::is_whitespace).unwrap_or(v.len()),
    };
    let tail = &v[consumed..];
    format!("{}{}", &tag[..i], tail)
}

/// Chaîne PHP entre apostrophes.
fn php_str(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Domaine du site (`https://www.exemple.fr` -> `exemple.fr`).
fn site_domain(site: &Site) -> Option<String> {
    let u = site.base_url.as_deref()?;
    let u = u.split_once("://").map(|(_, r)| r).unwrap_or(u);
    let h = u.split(['/', ':']).next()?.to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h).to_string();
    h.contains('.').then_some(h)
}

/// Script PHP qui reçoit les formulaires (adresse, objet, messages du site).
pub fn handler(site: &Site) -> String {
    let f = &site.settings.forms;
    let t = texts(&site.default_locale);
    // Expéditeur : adresse du domaine du site (exigée par OVHcloud et IONOS), sinon le destinataire.
    let from = site_domain(site)
        .map(|d| format!("site@{d}"))
        .unwrap_or_else(|| f.recipient.trim().to_string());
    let subject = f
        .subject
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("{} {}", t.subject, site.name));
    let ok = f
        .success
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| t.ok.to_string());
    format!(
        r#"<?php
// Formulaires du site, produit par Revel (Paramètres › Formulaires) : messages transmis par
// courriel, sans service extérieur. PHP 7.0 ou plus récent.
declare(strict_types=1);

const RECIPIENT = {recipient};
const FROM = {from};
const SITE = {site};
const SUBJECT = {subject};
const MSG_OK = {ok};
const MSG_INVALID = {invalid};
const MSG_LIMIT = {limit};
const MSG_FAIL = {fail};
const MAX_PER_HOUR = {max};

header('Cache-Control: no-store');
$json = stripos((string)($_SERVER['HTTP_ACCEPT'] ?? ''), 'application/json') !== false;

function reply(int $code, bool $ok, string $message, bool $json)
{{
    http_response_code($code);
    if ($json) {{
        header('Content-Type: application/json; charset=utf-8');
        echo json_encode(['ok' => $ok, 'message' => $message], JSON_UNESCAPED_UNICODE);
        exit;
    }}
    header('Content-Type: text/html; charset=utf-8');
    $back = '/';
    $ref = (string)($_SERVER['HTTP_REFERER'] ?? '');
    $host = (string)($_SERVER['HTTP_HOST'] ?? '');
    if ($ref !== '' && $host !== '' && parse_url($ref, PHP_URL_HOST) === parse_url('http://' . $host, PHP_URL_HOST)) {{
        $back = $ref;
    }}
    $e = function (string $s): string {{ return htmlspecialchars($s, ENT_QUOTES, 'UTF-8'); }};
    echo '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">'
        . '<title>' . $e(SITE) . '</title><body style="font:16px/1.5 system-ui,sans-serif;max-width:560px;margin:15vh auto;padding:0 24px">'
        . '<p>' . $e($message) . '</p><p><a href="' . $e($back) . '">&larr; ' . $e(SITE) . '</a></p>';
    exit;
}}

if (($_SERVER['REQUEST_METHOD'] ?? '') !== 'POST') {{
    reply(405, false, MSG_INVALID, $json);
}}
// Champ piège : seuls les robots le remplissent.
if (trim((string)($_POST['_revel_hp'] ?? '')) !== '') {{
    reply(200, true, MSG_OK, $json);
}}

$cut = function (string $s, int $n): string {{
    return function_exists('mb_substr') ? mb_substr($s, 0, $n, 'UTF-8') : substr($s, 0, $n);
}};
$fields = [];
$email = '';
foreach ($_POST as $k => $v) {{
    $k = (string)$k;
    if (strpos($k, '_revel') === 0 || count($fields) >= 40) {{
        continue;
    }}
    $v = is_array($v) ? implode(', ', array_map('strval', $v)) : (string)$v;
    $v = trim($cut($v, 5000));
    $k = $cut(str_replace(["\r", "\n"], ' ', $k), 60);
    if ($v === '') {{
        continue;
    }}
    $fields[$k] = $v;
    if ($email === '' && stripos($k, 'mail') !== false && filter_var($v, FILTER_VALIDATE_EMAIL)) {{
        $email = $v;
    }}
}}
if (!$fields) {{
    reply(422, false, MSG_INVALID, $json);
}}

// Limite d'envois par adresse IP (fichier temporaire du serveur).
$bucket = sys_get_temp_dir() . '/revel-form-' . hash('sha256', (string)($_SERVER['REMOTE_ADDR'] ?? '') . FROM);
$now = time();
$hits = [];
if (is_file($bucket)) {{
    foreach (explode(',', (string)@file_get_contents($bucket)) as $t) {{
        if ((int)$t > $now - 3600) {{
            $hits[] = (int)$t;
        }}
    }}
}}
if (count($hits) >= MAX_PER_HOUR) {{
    reply(429, false, MSG_LIMIT, $json);
}}
$hits[] = $now;
@file_put_contents($bucket, implode(',', $hits), LOCK_EX);

$clean = function (string $v): string {{ return str_replace(["\r", "\n"], ' ', $v); }};
$form = $clean($cut((string)($_POST['_revel_form'] ?? ''), 60));
$body = SUBJECT . ($form !== '' ? ' (' . $form . ')' : '') . "\n\n";
foreach ($fields as $k => $v) {{
    $body .= $k . " :\n" . $v . "\n\n";
}}
$headers = [
    'From: =?UTF-8?B?' . base64_encode($clean(SITE)) . '?= <' . FROM . '>',
    'MIME-Version: 1.0',
    'Content-Type: text/plain; charset=UTF-8',
    'Content-Transfer-Encoding: 8bit',
];
if ($email !== '') {{
    $headers[] = 'Reply-To: ' . $clean($email);
}}
$subject = '=?UTF-8?B?' . base64_encode(SUBJECT) . '?=';
$sent = @mail(RECIPIENT, $subject, $body, implode("\r\n", $headers), '-f' . FROM);
if (!$sent) {{
    reply(500, false, MSG_FAIL, $json);
}}
reply(200, true, MSG_OK, $json);
"#,
        recipient = php_str(f.recipient.trim()),
        from = php_str(&from),
        site = php_str(&site.name),
        subject = php_str(&subject),
        ok = php_str(&ok),
        invalid = php_str(t.invalid),
        limit = php_str(t.limit),
        fail = php_str(t.fail),
        max = MAX_PER_HOUR,
    )
}

/// Script des formulaires ajouté au site quand une page en contient (destinataire réglé), sinon
/// avertissement.
pub(crate) fn handler_file(
    site: &Site,
    forms: usize,
    files: &mut Vec<OutputFile>,
    warnings: &mut Vec<String>,
) {
    if forms == 0 || files.iter().any(|f| f.path == HANDLER) {
        return;
    }
    if site.settings.forms.recipient.trim().is_empty() {
        warnings.push(
            "formulaires non reçus : adresse de réception absente (Paramètres › Formulaires)"
                .into(),
        );
        return;
    }
    files.push(OutputFile {
        path: HANDLER.into(),
        bytes: handler(site).into_bytes(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> Site {
        let mut s = Site::new("Atelier « Lumen »", "fr");
        s.base_url = Some("https://www.lumen.fr".into());
        s.settings.forms.recipient = "bonjour@lumen.fr".into();
        s
    }

    #[test]
    fn formulaires_relies() {
        let html = r#"<html><body><form class="f" data-revel-form="contact" action=""><input name="email"></form><form action="/autre.php"></form></body></html>"#;
        let (out, n) = wire(&site(), html.to_string(), true);
        assert_eq!(n, 1);
        assert!(out.contains(r#"<form class="f" data-revel-form="contact" action="/revel-form.php" method="post" data-revel-error="#), "{out}");
        assert!(
            out.contains(r#"name="_revel_hp""#)
                && out.contains(r#"name="_revel_form" value="contact""#)
        );
        assert!(out.contains(r#"<form action="/autre.php">"#));
        assert!(out.contains("<script>document.addEventListener(\"submit\""));
        assert_eq!(out.matches("_revel_hp").count(), 1);
        // Canevas de l'éditeur : formulaire relié, sans script.
        let (out, _) = wire(&site(), html.to_string(), false);
        assert!(!out.contains("<script>"));
        // Sans formulaire Revel : page inchangée.
        let plain = "<form action=\"/x\"></form>".to_string();
        assert_eq!(wire(&site(), plain.clone(), true), (plain, 0));
    }

    #[test]
    fn script_php() {
        let php = handler(&site());
        assert!(php.starts_with("<?php"));
        assert!(php.contains("const RECIPIENT = 'bonjour@lumen.fr';"));
        assert!(php.contains("const FROM = 'site@lumen.fr';"));
        assert!(php.contains("const SITE = 'Atelier « Lumen »';"));
        assert!(php.contains("Merci, votre message a bien été envoyé."));
        assert_eq!(php_str("l'a\\b"), r"'l\'a\\b'");
        let mut files = Vec::new();
        let mut warnings = Vec::new();
        let mut s = site();
        s.settings.forms.recipient.clear();
        handler_file(&s, 1, &mut files, &mut warnings);
        assert!(files.is_empty() && warnings.len() == 1);
        handler_file(&site(), 1, &mut files, &mut warnings);
        assert_eq!(files[0].path, HANDLER);
    }
}
