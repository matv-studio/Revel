//! Icônes proposées par l'élément « Icône » (3.7) : dessins au trait (grille de 24, trait de 2,
//! `currentColor`), dessinés pour Revel. Insérées en `<svg>` dans la page : couleur et taille se
//! règlent par les styles (couleur du texte, largeur et hauteur).

/// Nom (identifiant, en français) et contenu du dessin.
pub const ICONS: &[(&str, &str)] = &[
    ("fleche-droite", r#"<path d="M5 12h14M13 6l6 6-6 6"/>"#),
    ("fleche-gauche", r#"<path d="M19 12H5M11 6l-6 6 6 6"/>"#),
    ("fleche-haut-droite", r#"<path d="M7 17 17 7M8 7h9v9"/>"#),
    (
        "fleche-epaisse",
        r#"<path fill="currentColor" stroke="none" d="M3 9h9V4l9 8-9 8v-5H3z"/>"#,
    ),
    ("chevron-droite", r#"<path d="m9 6 6 6-6 6"/>"#),
    ("chevron-bas", r#"<path d="m6 9 6 6 6-6"/>"#),
    ("plus", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("moins", r#"<path d="M5 12h14"/>"#),
    ("croix", r#"<path d="M6 6l12 12M18 6 6 18"/>"#),
    ("coche", r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#),
    ("menu", r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#),
    (
        "recherche",
        r#"<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/>"#,
    ),
    (
        "maison",
        r#"<path d="M4 11 12 4l8 7v8.5a.5.5 0 0 1-.5.5H15v-6H9v6H4.5a.5.5 0 0 1-.5-.5z"/>"#,
    ),
    (
        "utilisateur",
        r#"<circle cx="12" cy="8" r="4"/><path d="M4.5 20a7.5 7.5 0 0 1 15 0"/>"#,
    ),
    (
        "courriel",
        r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m4 7 8 6 8-6"/>"#,
    ),
    (
        "telephone",
        r#"<path d="M6.5 3.5h3l1.5 4-2 1.5a11 11 0 0 0 6 6l1.5-2 4 1.5v3a2 2 0 0 1-2 2A16.5 16.5 0 0 1 4.5 5.5a2 2 0 0 1 2-2z"/>"#,
    ),
    (
        "lieu",
        r#"<path d="M12 21s-7-6.2-7-11.5a7 7 0 0 1 14 0C19 14.8 12 21 12 21z"/><circle cx="12" cy="9.5" r="2.5"/>"#,
    ),
    (
        "calendrier",
        r#"<rect x="3.5" y="5" width="17" height="15" rx="2"/><path d="M3.5 10h17M8 3v4M16 3v4"/>"#,
    ),
    (
        "horloge",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#,
    ),
    (
        "etoile",
        r#"<path d="m12 3.5 2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z"/>"#,
    ),
    (
        "coeur",
        r#"<path d="M12 20s-7.5-4.6-7.5-10.2A4.3 4.3 0 0 1 12 7.1a4.3 4.3 0 0 1 7.5 2.7C19.5 15.4 12 20 12 20z"/>"#,
    ),
    (
        "lien",
        r#"<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>"#,
    ),
    (
        "lien-externe",
        r#"<path d="M14 4h6v6M20 4l-9 9M18 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10"/>"#,
    ),
    (
        "telecharger",
        r#"<path d="M12 4v11M7 10.5l5 5 5-5M5 20h14"/>"#,
    ),
    (
        "lecture",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="m10 8.5 5.5 3.5-5.5 3.5z"/>"#,
    ),
    (
        "image",
        r#"<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><circle cx="9" cy="10" r="1.75"/><path d="m4 18 5-5 4 4 2.5-2.5L20 19"/>"#,
    ),
    (
        "camera",
        r#"<path d="M4 8h3l1.5-2.5h7L17 8h3a1 1 0 0 1 1 1v9.5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13" r="3.5"/>"#,
    ),
    (
        "message",
        r#"<path d="M4.5 5h15a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H10l-4.5 3.5V17h-1a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z"/>"#,
    ),
    (
        "citation",
        r#"<path d="M9.5 7H6a1 1 0 0 0-1 1v4h4.5v4.5L6 19M19 7h-3.5a1 1 0 0 0-1 1v4H19v4.5L15.5 19"/>"#,
    ),
    (
        "info",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.5v.5"/>"#,
    ),
    (
        "alerte",
        r#"<path d="M12 4 21 19.5H3z"/><path d="M12 10v4.5M12 17v.5"/>"#,
    ),
    (
        "globe",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.5 2.6 3.5 5.5 3.5 8.5s-1 5.9-3.5 8.5c-2.5-2.6-3.5-5.5-3.5-8.5s1-5.9 3.5-8.5z"/>"#,
    ),
    (
        "mallette",
        r#"<rect x="3.5" y="7" width="17" height="12.5" rx="2"/><path d="M9 7V5.5A1.5 1.5 0 0 1 10.5 4h3A1.5 1.5 0 0 1 15 5.5V7M3.5 12.5h17"/>"#,
    ),
    (
        "partager",
        r#"<circle cx="6.5" cy="12" r="2.5"/><circle cx="17.5" cy="6" r="2.5"/><circle cx="17.5" cy="18" r="2.5"/><path d="m8.7 10.8 6.6-3.6M8.7 13.2l6.6 3.6"/>"#,
    ),
    ("eclair", r#"<path d="M13 3 5 13.5h6L10 21l8-10.5h-6z"/>"#),
    (
        "ampoule",
        r#"<path d="M9 17.5h6M10 20.5h4M12 3.5a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V17h5v-.6c0-.8.4-1.5 1-2A6 6 0 0 0 12 3.5z"/>"#,
    ),
    (
        "cible",
        r#"<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="4.5"/><circle cx="12" cy="12" r=".75"/>"#,
    ),
];

/// Contenu du dessin `name`.
pub fn icon(name: &str) -> Option<&'static str> {
    ICONS.iter().find(|(n, _)| *n == name).map(|(_, p)| *p)
}
