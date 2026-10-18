//! Dates affichées (D-73) : écriture d'une date reliée à un champ du contenu, choisie dans une
//! liste (comme le format de date de Webflow) et écrite en filtre d'expression.

/// Noms des mois d'une langue (forme longue, forme abrégée).
fn month_names(locale: &str) -> ([&'static str; 12], [&'static str; 12]) {
    match locale.split(['-', '_']).next().unwrap_or("fr") {
        "en" => (
            [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ],
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ],
        ),
        "de" => (
            [
                "Januar",
                "Februar",
                "März",
                "April",
                "Mai",
                "Juni",
                "Juli",
                "August",
                "September",
                "Oktober",
                "November",
                "Dezember",
            ],
            [
                "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.",
                "Nov.", "Dez.",
            ],
        ),
        "es" => (
            [
                "enero",
                "febrero",
                "marzo",
                "abril",
                "mayo",
                "junio",
                "julio",
                "agosto",
                "septiembre",
                "octubre",
                "noviembre",
                "diciembre",
            ],
            [
                "ene.", "feb.", "mar.", "abr.", "may.", "jun.", "jul.", "ago.", "sept.", "oct.",
                "nov.", "dic.",
            ],
        ),
        "it" => (
            [
                "gennaio",
                "febbraio",
                "marzo",
                "aprile",
                "maggio",
                "giugno",
                "luglio",
                "agosto",
                "settembre",
                "ottobre",
                "novembre",
                "dicembre",
            ],
            [
                "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
            ],
        ),
        _ => (
            [
                "janvier",
                "février",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "août",
                "septembre",
                "octobre",
                "novembre",
                "décembre",
            ],
            [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ],
        ),
    }
}

/// Date en toutes lettres : `long` (16 octobre 2026), `month` (octobre 2026), `short`
/// (16 oct. 2026). `month` : 1 à 12.
pub fn text(year: i32, month: u32, day: u32, locale: &str, style: &str) -> String {
    let (long, short) = month_names(locale);
    let m = (month.clamp(1, 12) - 1) as usize;
    let lang = locale.split(['-', '_']).next().unwrap_or("fr");
    match (style, lang) {
        ("month", _) => format!("{} {year}", long[m]),
        ("short", "en") => format!("{} {day}, {year}", short[m]),
        ("short", "de") => format!("{day}. {} {year}", short[m]),
        ("short", _) => format!("{day} {} {year}", short[m]),
        (_, "en") => format!("{} {day}, {year}", long[m]),
        (_, "de") => format!("{day}. {} {year}", long[m]),
        (_, "es") => format!("{day} de {} de {year}", long[m]),
        _ => format!("{day} {} {year}", long[m]),
    }
}

/// Écritures proposées pour une date reliée : filtre ajouté à l'expression (`locale` : langue
/// de la page rendue), exemple affiché pour le 16 octobre 2026.
pub const FORMATS: &[(&str, &str)] = &[
    ("date(\"%d.%m.%Y\")", "16.10.2026"),
    ("date(\"%d/%m/%Y\")", "16/10/2026"),
    ("date(\"%Y-%m-%d\")", "2026-10-16"),
    ("date_text(locale, \"long\")", ""),
    ("date_text(locale, \"short\")", ""),
    ("date_text(locale, \"month\")", ""),
    ("date(\"%Y\")", "2026"),
];

/// Exemple affiché d'une écriture (date du 16 octobre 2026).
pub fn example(format: &str, lang: &str) -> String {
    let style = ["long", "short", "month"]
        .into_iter()
        .find(|s| format.contains(&format!("\"{s}\"")));
    match (format.starts_with("date_text("), style) {
        (true, Some(s)) => text(2026, 10, 16, lang, s),
        _ => FORMATS
            .iter()
            .find(|(f, _)| *f == format)
            .map(|(_, e)| e.to_string())
            .unwrap_or_else(|| format.to_string()),
    }
}

/// Écriture (forme de [`FORMATS`]) d'un filtre lu dans une expression (`date_text("fr",
/// "long")` est lu comme `date_text(locale, "long")`) ; `None` : autre filtre.
pub fn format_of(filter: &str) -> Option<String> {
    // Guillemets simples acceptés (`date('%d.%m.%Y')`).
    let f = filter.trim();
    let dq;
    let f = if f.contains('\'') && !f.contains('"') {
        dq = f.replace('\'', "\"");
        dq.as_str()
    } else {
        f
    };
    if FORMATS.iter().any(|(x, _)| *x == f) {
        return Some(f.to_string());
    }
    let inner = f.strip_prefix("date_text(")?.strip_suffix(')')?;
    let (_, style) = inner.split_once(',')?;
    let style = crate::canvas::literal(style.trim())?;
    let generic = format!("date_text(locale, \"{style}\")");
    FORMATS
        .iter()
        .any(|(x, _)| *x == generic)
        .then_some(generic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecritures() {
        assert_eq!(text(2026, 10, 16, "fr", "long"), "16 octobre 2026");
        assert_eq!(text(2026, 10, 16, "en-GB", "short"), "Oct 16, 2026");
        assert_eq!(
            example("date_text(locale, \"month\")", "fr"),
            "octobre 2026"
        );
        assert_eq!(example("date(\"%d.%m.%Y\")", "fr"), "16.10.2026");
        assert_eq!(
            format_of("date_text(\"en\", \"long\")").as_deref(),
            Some("date_text(locale, \"long\")")
        );
        assert_eq!(
            format_of("date(\"%d.%m.%Y\")").as_deref(),
            Some("date(\"%d.%m.%Y\")")
        );
        assert_eq!(format_of("upper"), None);
    }
}
