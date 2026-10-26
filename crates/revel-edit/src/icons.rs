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
    ("fleche-bas", r#"<path d="M12 5v14M6 13l6 6 6-6"/>"#),
    ("fleche-haut", r#"<path d="M12 19V5M6 11l6-6 6 6"/>"#),
    ("chevron-gauche", r#"<path d="m15 6-6 6 6 6"/>"#),
    ("chevron-haut", r#"<path d="m6 15 6-6 6 6"/>"#),
    (
        "doubles-chevrons",
        r#"<path d="m7 6 6 6-6 6M13 6l6 6-6 6"/>"#,
    ),
    (
        "retour",
        r#"<path d="M9 7 4 12l5 5M4 12h11a5 5 0 0 1 0 10h-2"/>"#,
    ),
    (
        "actualiser",
        r#"<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3M19.5 4v4.5H15"/>"#,
    ),
    (
        "agrandir",
        r#"<path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7"/>"#,
    ),
    (
        "reduire",
        r#"<path d="M10 4v6H4M14 20v-6h6M4 10l6-6M20 14l-6 6"/>"#,
    ),
    (
        "parametres",
        r#"<circle cx="12" cy="12" r="3"/><path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8"/>"#,
    ),
    ("filtre", r#"<path d="M4 5h16l-6 7.5V19l-4 1.5v-8z"/>"#),
    (
        "grille",
        r#"<rect x="4" y="4" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1"/>"#,
    ),
    (
        "liste",
        r#"<path d="M9 7h11M9 12h11M9 17h11M4.5 7h.01M4.5 12h.01M4.5 17h.01"/>"#,
    ),
    (
        "plus-cercle",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 8v8M8 12h8"/>"#,
    ),
    (
        "coche-cercle",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="m8.5 12.5 2.5 2.5 4.5-5"/>"#,
    ),
    (
        "croix-cercle",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="m9 9 6 6M15 9l-6 6"/>"#,
    ),
    (
        "aide",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17v.5"/>"#,
    ),
    (
        "cadenas",
        r#"<rect x="5" y="10.5" width="14" height="10" rx="2"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5"/>"#,
    ),
    (
        "oeil",
        r#"<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>"#,
    ),
    (
        "cloche",
        r#"<path d="M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 2H4.5zM10 20.5h4"/>"#,
    ),
    ("signet", r#"<path d="M7 4h10v16.5l-5-3.5-5 3.5z"/>"#),
    (
        "panier",
        r#"<path d="M4 6h2l2 10h10l2-7H7.2"/><circle cx="9.5" cy="19.5" r="1.25"/><circle cx="16.5" cy="19.5" r="1.25"/>"#,
    ),
    (
        "sac",
        r#"<path d="M5.5 8h13l-1 12h-11z"/><path d="M9 10V7a3 3 0 0 1 6 0v3"/>"#,
    ),
    (
        "carte-bancaire",
        r#"<rect x="3" y="5.5" width="18" height="13" rx="2"/><path d="M3 10h18M7 15h3"/>"#,
    ),
    (
        "cadeau",
        r#"<rect x="4" y="9" width="16" height="11" rx="1"/><path d="M3.5 9h17v-3h-17zM12 6v14M12 6c-2-3-6-2.5-4.5 0M12 6c2-3 6-2.5 4.5 0"/>"#,
    ),
    (
        "document",
        r#"<path d="M6.5 3.5h7l4 4v13h-11z"/><path d="M13.5 3.5v4h4M9 12.5h6M9 16h6"/>"#,
    ),
    (
        "dossier",
        r#"<path d="M3.5 6.5a1 1 0 0 1 1-1H10l2 2.5h7.5a1 1 0 0 1 1 1V18a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1z"/>"#,
    ),
    (
        "crayon",
        r#"<path d="M15.5 4.5 19.5 8.5 9 19l-5 1 1-5z"/><path d="m13.5 6.5 4 4"/>"#,
    ),
    (
        "corbeille",
        r#"<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 13h9l1-13M10 11v5.5M14 11v5.5"/>"#,
    ),
    (
        "micro",
        r#"<rect x="9" y="3.5" width="6" height="11" rx="3"/><path d="M6 11.5a6 6 0 0 0 12 0M12 17.5v3"/>"#,
    ),
    (
        "haut-parleur",
        r#"<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11"/>"#,
    ),
    (
        "video",
        r#"<rect x="3" y="6" width="13" height="12" rx="2"/><path d="m16 10.5 5-3v9l-5-3"/>"#,
    ),
    (
        "musique",
        r#"<path d="M9 17.5V5.5l10-2v12"/><circle cx="6.5" cy="17.5" r="2.5"/><circle cx="16.5" cy="15.5" r="2.5"/>"#,
    ),
    ("pause", r#"<path d="M9 6v12M15 6v12"/>"#),
    (
        "soleil",
        r#"<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"/>"#,
    ),
    (
        "lune",
        r#"<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"/>"#,
    ),
    (
        "feuille",
        r#"<path d="M5 19c0-8 5-13.5 14.5-14 .5 9.5-5 14.5-13 14.5zM5 19l7-7"/>"#,
    ),
    (
        "goutte",
        r#"<path d="M12 3.5s6 6.7 6 11a6 6 0 0 1-12 0c0-4.3 6-11 6-11z"/>"#,
    ),
    (
        "flamme",
        r#"<path d="M12 21a6 6 0 0 0 6-6c0-4.5-4-6-4.5-10.5C10 7 9 9.5 9.5 12 8 11.5 7 10 7 8.5 6.2 10 6 12 6 15a6 6 0 0 0 6 6z"/>"#,
    ),
    (
        "trophee",
        r#"<path d="M8 4h8v5a4 4 0 0 1-8 0zM8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8.5 20.5h7M10 17h4"/>"#,
    ),
    (
        "drapeau",
        r#"<path d="M5.5 21V4M5.5 4.5h11l-2 4 2 4h-11"/>"#,
    ),
    (
        "fusee",
        r#"<path d="M12 3c3 2 4.5 5 4.5 9l-2 3h-5l-2-3c0-4 1.5-7 4.5-9z"/><circle cx="12" cy="9.5" r="1.5"/><path d="M9.5 15 7 18.5l3-.5M14.5 15l2.5 3.5-3-.5M12 17v4"/>"#,
    ),
    (
        "main",
        r#"<path d="M8.5 12V5.5a1.5 1.5 0 0 1 3 0V11M11.5 10V4.5a1.5 1.5 0 0 1 3 0V11M14.5 10.5V6a1.5 1.5 0 0 1 3 0v8a7 7 0 0 1-7 7c-2 0-3.5-.8-4.7-2.4L3.5 15a1.5 1.5 0 0 1 2.3-1.9l2.7 2.4"/>"#,
    ),
    (
        "pouce",
        r#"<path d="M7.5 10.5v9.5h-3v-9.5zM7.5 19.5h9.2a2 2 0 0 0 2-1.6l1.2-6a2 2 0 0 0-2-2.4H14l.8-3.6A1.8 1.8 0 0 0 13 3.5l-5.5 7"/>"#,
    ),
    (
        "sourire",
        r#"<circle cx="12" cy="12" r="8.5"/><path d="M8.5 14a4 4 0 0 0 7 0M9 9.5h.01M15 9.5h.01"/>"#,
    ),
    (
        "equipe",
        r#"<circle cx="9" cy="8.5" r="3"/><circle cx="16.5" cy="9.5" r="2.5"/><path d="M3.5 19a5.5 5.5 0 0 1 11 0M14.5 14.6a4.5 4.5 0 0 1 6 4.4"/>"#,
    ),
    (
        "batiment",
        r#"<path d="M5 20.5V4.5h9v16M14 9.5h5v11M3 20.5h18M8 8h3M8 11.5h3M8 15h3"/>"#,
    ),
    (
        "voiture",
        r#"<path d="M4 16.5V12l2-5h12l2 5v4.5zM4 12h16"/><circle cx="7.5" cy="16.5" r="1.75"/><circle cx="16.5" cy="16.5" r="1.75"/>"#,
    ),
    (
        "avion",
        r#"<path d="M21 4 3 11l7 2.5L12.5 21z"/><path d="M10 13.5 21 4"/>"#,
    ),
    (
        "tasse",
        r#"<path d="M5 8.5h11v6a5 5 0 0 1-5 5h-1a5 5 0 0 1-5-5zM16 10h1.5a2.5 2.5 0 0 1 0 5H16M8.5 3.5v2M12.5 3.5v2"/>"#,
    ),
    ("code", r#"<path d="m9 7-5 5 5 5M15 7l5 5-5 5"/>"#),
    (
        "terminal",
        r#"<rect x="3" y="4.5" width="18" height="15" rx="2"/><path d="m7 9.5 3 2.5-3 2.5M12.5 15h4.5"/>"#,
    ),
    (
        "cube",
        r#"<path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z"/><path d="M4 7.5 12 12l8-4.5M12 12v9"/>"#,
    ),
    (
        "calques",
        r#"<path d="M12 4 21 8.5 12 13 3 8.5z"/><path d="m3 12.5 9 4.5 9-4.5M3 16.5l9 4.5 9-4.5"/>"#,
    ),
    (
        "palette",
        r#"<path d="M12 3.5a8.5 8.5 0 0 0 0 17c1 0 1.5-.7 1.5-1.5 0-1.5-1.5-1.5-1.5-3 0-1 .8-1.5 1.8-1.5H16a4.5 4.5 0 0 0 4.5-4.5c0-3.6-3.8-6.5-8.5-6.5z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10.5" cy="7.5" r="1"/><circle cx="15" cy="8" r="1"/>"#,
    ),
    (
        "pinceau",
        r#"<path d="M19.5 4.5 11 13l-1.5-1.5L18 3zM9.5 13.5c-2.5 0-4 1.5-4 4 0 1.5-1 2.5-2 2.5 4 1.5 8 0 8-4.5z"/>"#,
    ),
    (
        "graphique",
        r#"<path d="M4 4v16h16M8 16v-5M12 16V8M16 16v-3"/>"#,
    ),
    (
        "tendance",
        r#"<path d="m3.5 17 6-6 4 4 7-7.5M15 7.5h5.5V13"/>"#,
    ),
];

/// Onglets du choix d'icône (clé de traduction, icônes), dans l'ordre ; les icônes importées ont
/// leur propre onglet (D-125).
pub const GROUPS: &[(&str, &[&str])] = &[
    (
        "icons-arrows",
        &[
            "fleche-droite",
            "fleche-gauche",
            "fleche-haut",
            "fleche-bas",
            "fleche-haut-droite",
            "fleche-epaisse",
            "chevron-droite",
            "chevron-gauche",
            "chevron-haut",
            "chevron-bas",
            "doubles-chevrons",
            "retour",
            "actualiser",
            "agrandir",
            "reduire",
        ],
    ),
    (
        "icons-interface",
        &[
            "plus",
            "moins",
            "croix",
            "coche",
            "plus-cercle",
            "coche-cercle",
            "croix-cercle",
            "menu",
            "recherche",
            "filtre",
            "parametres",
            "grille",
            "liste",
            "info",
            "aide",
            "alerte",
            "cadenas",
            "oeil",
            "cloche",
            "signet",
            "lien",
            "lien-externe",
            "telecharger",
            "partager",
            "crayon",
            "corbeille",
        ],
    ),
    (
        "icons-contact",
        &[
            "maison",
            "utilisateur",
            "equipe",
            "courriel",
            "telephone",
            "message",
            "lieu",
            "calendrier",
            "horloge",
            "globe",
            "batiment",
            "mallette",
        ],
    ),
    (
        "icons-media",
        &[
            "image",
            "camera",
            "video",
            "lecture",
            "pause",
            "musique",
            "micro",
            "haut-parleur",
            "document",
            "dossier",
            "citation",
        ],
    ),
    (
        "icons-objects",
        &[
            "etoile",
            "coeur",
            "pouce",
            "sourire",
            "main",
            "eclair",
            "ampoule",
            "cible",
            "trophee",
            "drapeau",
            "fusee",
            "soleil",
            "lune",
            "feuille",
            "goutte",
            "flamme",
            "tasse",
            "panier",
            "sac",
            "carte-bancaire",
            "cadeau",
            "voiture",
            "avion",
        ],
    ),
    (
        "icons-design",
        &[
            "code",
            "terminal",
            "cube",
            "calques",
            "palette",
            "pinceau",
            "graphique",
            "tendance",
        ],
    ),
];

/// Dessin importé (fichier SVG) prêt à placer dans un `<svg>` de la page : `viewBox` et contenu,
/// sans script, sans gestionnaire d'événement ni adresse `javascript:` (D-125). `None` si ce
/// n'est pas un SVG.
pub fn clean_svg(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find("<svg")?;
    let open_end = start + lower[start..].find('>')?;
    let close = lower.rfind("</svg>")?;
    if close <= open_end {
        return None;
    }
    let head = &text[start..open_end];
    let attr = |name: &str| -> Option<String> {
        let l = head.to_ascii_lowercase();
        let i = l.find(&format!(" {name}=\""))? + name.len() + 3;
        let j = i + head[i..].find('"')?;
        Some(head[i..j].to_string())
    };
    let num =
        |v: Option<String>| v.and_then(|v| v.trim_end_matches("px").trim().parse::<f64>().ok());
    let view_box = attr("viewbox").or_else(|| {
        let (w, h) = (num(attr("width"))?, num(attr("height"))?);
        Some(format!("0 0 {w} {h}"))
    })?;
    let mut inner = text[open_end + 1..close].to_string();
    // Éléments actifs retirés avec leur contenu.
    for tag in ["script", "foreignobject", "iframe", "object", "embed"] {
        loop {
            let l = inner.to_ascii_lowercase();
            let Some(a) = l.find(&format!("<{tag}")) else {
                break;
            };
            let end = l[a..]
                .find(&format!("</{tag}>"))
                .map(|e| a + e + tag.len() + 3)
                .or_else(|| l[a..].find("/>").map(|e| a + e + 2))
                .unwrap_or(l.len());
            inner.replace_range(a..end, "");
        }
    }
    // Gestionnaires d'événements (`onload="…"`) et adresses `javascript:`.
    let mut out = String::with_capacity(inner.len());
    let b = inner.as_bytes();
    let low = inner.to_ascii_lowercase();
    let mut i = 0;
    while i < inner.len() {
        let rest = &inner[i..];
        let l = &low[i..];
        let ws = i > 0 && b[i - 1].is_ascii_whitespace();
        if ws
            && l.starts_with("on")
            && l[2..].starts_with(|c: char| c.is_ascii_alphabetic())
            && let Some(eq) = l
                .find('=')
                .filter(|e| l[..*e].chars().all(|c| c.is_ascii_alphanumeric()))
            && let q = rest[eq + 1..].chars().next().unwrap_or(' ')
            && (q == '"' || q == '\'')
            && let Some(e) = rest[eq + 2..].find(q)
        {
            i += eq + 2 + e + 1;
            continue;
        }
        if l.starts_with("javascript:") {
            out.push('#');
            i += "javascript:".len();
            continue;
        }
        let c = rest.chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    // Réglages de dessin de l'élément `svg` gardés sur un groupe (contour, remplissage).
    let keep: Vec<String> = [
        "fill",
        "stroke",
        "stroke-width",
        "stroke-linecap",
        "stroke-linejoin",
        "fill-rule",
    ]
    .iter()
    .filter_map(|a| attr(a).map(|v| format!(" {a}=\"{}\"", v.replace('"', ""))))
    .collect();
    let body = out.trim().to_string();
    Some((
        view_box,
        if keep.is_empty() {
            body
        } else {
            format!("<g{}>{body}</g>", keep.concat())
        },
    ))
}

/// Commande qui met le dessin d'une icône dans l'élément `svg` `at` : nom (`data-icon`), cadre
/// (`viewBox`), contenu ; dessin au trait des icônes de Revel (`line`), ou dessin importé, rempli
/// de la couleur du texte sauf réglage contraire du fichier.
pub fn set_icon(
    site: &revel_model::Site,
    at: &crate::address::Target,
    name: &str,
    view_box: &str,
    inner: &str,
    line: bool,
) -> crate::command::Command {
    use crate::command::Command;
    let set = |n: &str, v: &str| Command::SetAttr {
        at: at.clone(),
        name: n.into(),
        value: Some(v.into()),
    };
    let mut commands = vec![set("data-icon", name), set("viewBox", view_box)];
    if line {
        commands.extend([
            set("fill", "none"),
            set("stroke", "currentColor"),
            set("stroke-width", "2"),
            set("stroke-linecap", "round"),
            set("stroke-linejoin", "round"),
        ]);
    } else {
        commands.push(set("fill", "currentColor"));
        commands.push(set("stroke", "none"));
        let has = |n: &str| matches!(crate::address::node(site, at), Ok(revel_model::Node::Element { attrs, .. }) if attrs.iter().any(|a| a.name == n));
        for n in ["stroke-width", "stroke-linecap", "stroke-linejoin"] {
            if has(n) {
                commands.push(Command::RemoveAttr {
                    at: at.clone(),
                    name: n.into(),
                });
            }
        }
    }
    commands.push(Command::Replace {
        parent: crate::address::Parent::of(at, crate::address::Branch::Children),
        index: 0,
        node: revel_model::Node::Html { html: inner.into() },
    });
    Command::Batch { commands }
}

/// Contenu du dessin `name`.
pub fn icon(name: &str) -> Option<&'static str> {
    ICONS.iter().find(|(n, _)| *n == name).map(|(_, p)| *p)
}
