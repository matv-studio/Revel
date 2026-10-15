//! Icônes (traits de 1,5 px, 16 px), dessinées pour Revel.

use leptos::prelude::*;

fn svg(path: &'static str) -> AnyView {
    view! {
        <svg class="icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"
            fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"
            inner_html=path></svg>
    }
    .into_any()
}

/// Marque de Revel : un « r » géométrique (fût et quart de cercle).
pub fn logo() -> AnyView {
    view! {
        <svg class="logo-mark" viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
            <rect x="0" y="0" width="24" height="24" rx="6" fill="currentColor"/>
            <path d="M8 18V11a5 5 0 0 1 5-5h3" fill="none" stroke="var(--bg)" stroke-width="2.4" stroke-linecap="round"/>
            <circle cx="16.5" cy="16.5" r="1.6" fill="var(--bg)"/>
        </svg>
    }
    .into_any()
}

pub fn grid() -> AnyView {
    svg(
        r#"<rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="9" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="2.5" y="9" width="4.5" height="4.5" rx="1"/><rect x="9" y="9" width="4.5" height="4.5" rx="1"/>"#,
    )
}

pub fn upload() -> AnyView {
    svg(r#"<path d="M8 11V3M5 6l3-3 3 3M3 11v2h10v-2"/>"#)
}

pub fn settings() -> AnyView {
    svg(
        r#"<circle cx="8" cy="8" r="2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/>"#,
    )
}

pub fn play() -> AnyView {
    svg(r#"<path d="M5 3.5v9l7-4.5z"/>"#)
}

pub fn close() -> AnyView {
    svg(r#"<path d="M4 4l8 8M12 4l-8 8"/>"#)
}

pub fn folder() -> AnyView {
    svg(
        r#"<path d="M2 4.5a1 1 0 0 1 1-1h3l1.5 1.5H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>"#,
    )
}

pub fn import() -> AnyView {
    svg(r#"<path d="M8 2v8M5 7l3 3 3-3M3 12.5h10"/>"#)
}

pub fn plus() -> AnyView {
    svg(r#"<path d="M8 3v10M3 8h10"/>"#)
}

pub fn edit() -> AnyView {
    svg(r#"<path d="M10.5 3l2.5 2.5L6 12.5H3.5V10z"/>"#)
}

pub fn trash() -> AnyView {
    svg(r#"<path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.5 8.5h6l.5-8.5"/>"#)
}

pub fn check() -> AnyView {
    svg(r#"<path d="M3 8.5l3 3 7-7"/>"#)
}

/// Ouvrir dans le navigateur.
pub fn external() -> AnyView {
    svg(
        r#"<path d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M11.5 9.5v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h3"/>"#,
    )
}

pub fn copy() -> AnyView {
    svg(
        r#"<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 5.5v-2a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2"/>"#,
    )
}

/// Compte relié (deux maillons).
pub fn link() -> AnyView {
    svg(
        r#"<path d="M6.5 9.5l3-3M7 4.5l1-1a2.8 2.8 0 0 1 4 4l-1 1M9 11.5l-1 1a2.8 2.8 0 0 1-4-4l1-1"/>"#,
    )
}

/// Ordinateur (point de rupture de base).
pub fn desktop() -> AnyView {
    svg(r#"<rect x="1.5" y="2.5" width="13" height="8.5" rx="1"/><path d="M5.5 13.5h5M8 11v2.5"/>"#)
}

/// Tablette (point de rupture intermédiaire).
pub fn tablet() -> AnyView {
    svg(r#"<rect x="3" y="1.5" width="10" height="13" rx="1.5"/><path d="M7 12.5h2"/>"#)
}

/// Téléphone (petit point de rupture).
pub fn mobile() -> AnyView {
    svg(r#"<rect x="4.5" y="1.5" width="7" height="13" rx="1.5"/><path d="M7.25 12.5h1.5"/>"#)
}

pub fn undo() -> AnyView {
    svg(r#"<path d="M5.5 3.5 2.5 6.5l3 3"/><path d="M2.5 6.5h7a4 4 0 0 1 0 8H7"/>"#)
}

pub fn redo() -> AnyView {
    svg(r#"<path d="m10.5 3.5 3 3-3 3"/><path d="M13.5 6.5h-7a4 4 0 0 0 0 8H9"/>"#)
}

/// Élément HTML (calques).
pub fn box_() -> AnyView {
    svg(r#"<rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/>"#)
}

/// Texte (calques).
pub fn text() -> AnyView {
    svg(r#"<path d="M3 3.5h10M8 3.5v9"/>"#)
}

/// Expression `{{ … }}` (calques).
pub fn braces() -> AnyView {
    svg(
        r#"<path d="M5.5 2.5C4 2.5 4 3.5 4 5s-.5 3-1.5 3c1 0 1.5 1.5 1.5 3s0 2.5 1.5 2.5M10.5 2.5c1.5 0 1.5 1 1.5 2.5s.5 3 1.5 3c-1 0-1.5 1.5-1.5 3s0 2.5-1.5 2.5"/>"#,
    )
}

/// Condition (calques).
pub fn branch() -> AnyView {
    svg(
        r#"<circle cx="4.5" cy="3.5" r="1.5"/><circle cx="4.5" cy="12.5" r="1.5"/><circle cx="11.5" cy="5.5" r="1.5"/><path d="M4.5 5v6M11.5 7c0 2.5-2 3.5-5.5 4"/>"#,
    )
}

/// Boucle (calques).
pub fn repeat() -> AnyView {
    svg(
        r#"<path d="M3 7V6a2.5 2.5 0 0 1 2.5-2.5h7M10.5 1.5l2 2-2 2M13 9v1a2.5 2.5 0 0 1-2.5 2.5h-7M5.5 14.5l-2-2 2-2"/>"#,
    )
}

/// Composant (calques).
pub fn component() -> AnyView {
    svg(r#"<path d="M8 1.5 14.5 8 8 14.5 1.5 8z"/><path d="M8 5 11 8 8 11 5 8z"/>"#)
}

/// Image (calques).
pub fn image() -> AnyView {
    svg(
        r#"<rect x="2" y="3" width="12" height="10" rx="1.5"/><circle cx="6" cy="6.5" r="1.2"/><path d="m2.5 12 4-3.5 3 2.5 2-1.5 2 2"/>"#,
    )
}

/// Code (HTML brut, variable).
pub fn code() -> AnyView {
    svg(r#"<path d="M5.5 4.5 2 8l3.5 3.5M10.5 4.5 14 8l-3.5 3.5"/>"#)
}

/// Emplacement du contenu dans un gabarit.
pub fn slot() -> AnyView {
    svg(r#"<rect x="2.5" y="2.5" width="11" height="11" rx="1.5" stroke-dasharray="2 2"/>"#)
}

pub fn arrow_up() -> AnyView {
    svg(r#"<path d="M8 13V3M4 7l4-4 4 4"/>"#)
}

pub fn arrow_down() -> AnyView {
    svg(r#"<path d="M8 3v10M4 9l4 4 4-4"/>"#)
}

/// Envelopper dans un nouvel élément.
pub fn wrap() -> AnyView {
    svg(
        r#"<rect x="1.5" y="1.5" width="13" height="13" rx="2"/><rect x="5" y="5" width="6" height="6" rx="1"/>"#,
    )
}

pub fn chevron_down() -> AnyView {
    svg(r#"<path d="m4.5 6.5 3.5 3.5 3.5-3.5"/>"#)
}

pub fn chevron_right() -> AnyView {
    svg(r#"<path d="m6.5 4.5 3.5 3.5-3.5 3.5"/>"#)
}

pub fn arrow_right() -> AnyView {
    svg(r#"<path d="M3 8h10M9 4l4 4-4 4"/>"#)
}

/// Inverser l'ordre.
pub fn reverse() -> AnyView {
    svg(r#"<path d="M4.5 2.5 2 5l2.5 2.5M2 5h9.5M11.5 8.5 14 11l-2.5 2.5M14 11H4.5"/>"#)
}

/// Pipette.
pub fn eyedropper() -> AnyView {
    svg(
        r#"<path d="M10.5 2.5a1.8 1.8 0 0 1 2.5 0l.5.5a1.8 1.8 0 0 1 0 2.5L12 7l-3-3z"/><path d="M9 4 3.5 9.5 3 12l-1 1.5.5.5L4 13l2.5-.5L12 7"/>"#,
    )
}

pub fn text_left() -> AnyView {
    svg(r#"<path d="M2.5 4h11M2.5 8h7M2.5 12h9"/>"#)
}

pub fn text_center() -> AnyView {
    svg(r#"<path d="M2.5 4h11M4.5 8h7M3.5 12h9"/>"#)
}

pub fn text_right() -> AnyView {
    svg(r#"<path d="M2.5 4h11M6.5 8h7M4.5 12h9"/>"#)
}

pub fn text_justify() -> AnyView {
    svg(r#"<path d="M2.5 4h11M2.5 8h11M2.5 12h11"/>"#)
}

pub fn italic() -> AnyView {
    svg(r#"<path d="M6.5 3h6M3.5 13h6M9.5 3l-3 10"/>"#)
}

pub fn underline() -> AnyView {
    svg(r#"<path d="M4.5 2.5v5a3.5 3.5 0 0 0 7 0v-5M3.5 13.5h9"/>"#)
}

pub fn strike() -> AnyView {
    svg(
        r#"<path d="M2.5 8h11M11 4.5C10.5 3.3 9.4 2.5 8 2.5 6 2.5 4.8 3.6 4.8 5c0 1 .6 1.7 1.7 2.2M5 11.3c.5 1.3 1.6 2.2 3.1 2.2 2 0 3.3-1.1 3.3-2.6 0-.6-.2-1.1-.5-1.5"/>"#,
    )
}

/// Angles indépendants.
pub fn corners() -> AnyView {
    svg(
        r#"<path d="M2.5 6V4.5a2 2 0 0 1 2-2H6M10 2.5h1.5a2 2 0 0 1 2 2V6M13.5 10v1.5a2 2 0 0 1-2 2H10M6 13.5H4.5a2 2 0 0 1-2-2V10"/>"#,
    )
}

/// Bordure : tous les côtés.
pub fn side_all() -> AnyView {
    svg(r#"<rect x="2.5" y="2.5" width="11" height="11" rx="1"/>"#)
}

fn side(active: &'static str) -> AnyView {
    view! {
        <svg class="icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"
            fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2.5" y="2.5" width="11" height="11" rx="1" stroke-dasharray="1.5 2" opacity=".55"/>
            <path d=active/>
        </svg>
    }
    .into_any()
}

pub fn side_top() -> AnyView {
    side("M2.5 2.5h11")
}

pub fn side_right() -> AnyView {
    side("M13.5 2.5v11")
}

pub fn side_bottom() -> AnyView {
    side("M2.5 13.5h11")
}

pub fn side_left() -> AnyView {
    side("M2.5 2.5v11")
}

/// Réinitialiser (flèche qui revient en arrière).
pub fn reset() -> AnyView {
    svg(r#"<path d="M3 8a5 5 0 1 0 1.5-3.6"/><path d="M3.5 2.5v2.5H6"/>"#)
}

/// Une page.
pub fn page() -> AnyView {
    svg(
        r#"<path d="M4.5 2h4.5l2.5 2.5V13a.5.5 0 0 1-.5.5h-6.5A.5.5 0 0 1 4 13V2.5A.5.5 0 0 1 4.5 2z"/>"#,
    )
}

/// Collections du CMS.
pub fn database() -> AnyView {
    svg(
        r#"<ellipse cx="8" cy="3.8" rx="5" ry="1.8"/><path d="M3 3.8v8.4c0 1 2.2 1.8 5 1.8s5-.8 5-1.8V3.8M3 8c0 1 2.2 1.8 5 1.8s5-.8 5-1.8"/>"#,
    )
}

/// Interactions (éclair).
pub fn bolt() -> AnyView {
    svg(r#"<path d="M9 1.5 3.5 9H8l-1 5.5L12.5 7H8z"/>"#)
}

/// Plus d'actions.
pub fn more() -> AnyView {
    view! {
        <svg class="icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="currentColor">
            <circle cx="3.5" cy="8" r="1.2"/><circle cx="8" cy="8" r="1.2"/><circle cx="12.5" cy="8" r="1.2"/>
        </svg>
    }
    .into_any()
}

/// Disposition des panneaux.
pub fn layout() -> AnyView {
    svg(
        r#"<rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="M5.5 2.5v11M10.5 2.5v11"/>"#,
    )
}

/// Recherche.
pub fn search() -> AnyView {
    svg(r#"<circle cx="7" cy="7" r="4.5"/><path d="m10.5 10.5 3 3"/>"#)
}

/// Affichage du canevas (règles, guides).
pub fn ruler() -> AnyView {
    svg(
        r#"<rect x="1.75" y="5.25" width="12.5" height="5.5" rx="1"/><path d="M4.5 5.25v2.25M7 5.25v1.5M9.5 5.25v2.25M12 5.25v1.5"/>"#,
    )
}

/// Champ date.
pub fn calendar() -> AnyView {
    svg(
        r#"<rect x="2.75" y="3.75" width="10.5" height="9.5" rx="1.5"/><path d="M2.75 6.75h10.5M5.5 2.5v2.5M10.5 2.5v2.5"/>"#,
    )
}

/// Champ oui / non.
pub fn toggle() -> AnyView {
    svg(
        r#"<rect x="1.75" y="4.75" width="12.5" height="6.5" rx="3.25"/><circle cx="11" cy="8" r="1.75"/>"#,
    )
}

/// Champ nombre.
pub fn hash() -> AnyView {
    svg(r#"<path d="M6.25 2.75 5 13.25M11 2.75 9.75 13.25M3 6h10.5M2.5 10H13"/>"#)
}

/// Poignée de déplacement.
pub fn grip() -> AnyView {
    svg(
        r#"<g fill="currentColor" stroke="none"><circle cx="6" cy="4" r="1.1"/><circle cx="10" cy="4" r="1.1"/><circle cx="6" cy="8" r="1.1"/><circle cx="10" cy="8" r="1.1"/><circle cx="6" cy="12" r="1.1"/><circle cx="10" cy="12" r="1.1"/></g>"#,
    )
}

/// Caché (œil barré).
pub fn eye_off() -> AnyView {
    svg(
        r#"<path d="M2 8s2.25-4.25 6-4.25S14 8 14 8s-2.25 4.25-6 4.25S2 8 2 8z"/><circle cx="8" cy="8" r="1.75"/><path d="M3 13 13 3"/>"#,
    )
}

/// Avertissement.
pub fn warning() -> AnyView {
    svg(
        r#"<path d="M8 2.5 14 13H2L8 2.5z"/><path d="M8 6.75v3"/><circle cx="8" cy="11.4" r=".4" fill="currentColor"/>"#,
    )
}

/// Aide d'un champ (texte au survol).
pub fn info() -> AnyView {
    svg(
        r#"<circle cx="8" cy="8" r="5.75"/><path d="M8 7.25v3.5"/><circle cx="8" cy="5.1" r=".4" fill="currentColor"/>"#,
    )
}

/// Sélectionner l'élément parent.
pub fn parent() -> AnyView {
    svg(r#"<path d="M5 6.5 8 3.5l3 3M8 3.5v9"/>"#)
}

/// Tout replier.
pub fn collapse() -> AnyView {
    svg(r#"<path d="m4.5 6 3.5-3 3.5 3M4.5 10l3.5 3 3.5-3"/>"#)
}

/// Vue en liste.
pub fn list() -> AnyView {
    svg(r#"<path d="M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.01M2.5 8h.01M2.5 12h.01"/>"#)
}

/// Globe (langues).
pub fn globe() -> AnyView {
    svg(
        r#"<circle cx="8" cy="8" r="6"/><path d="M2 8h12M8 2c1.8 1.7 2.6 3.7 2.6 6S9.8 12.3 8 14M8 2C6.2 3.7 5.4 5.7 5.4 8s.8 4.3 2.6 6"/>"#,
    )
}

/// Gras.
pub fn bold() -> AnyView {
    svg(r#"<path d="M5 3h4a2.5 2.5 0 0 1 0 5H5zM5 8h4.8a2.5 2.5 0 0 1 0 5H5z"/>"#)
}

/// Liste numérotée.
pub fn list_ordered() -> AnyView {
    svg(
        r#"<path d="M7 4h7M7 8h7M7 12h7"/><path d="M2.5 3.2 3.5 2.7V6M2.3 9.6c.3-.6 1.9-.7 1.9.3 0 .7-1.9 1.6-1.9 2.6h2" stroke-width="1.2"/>"#,
    )
}

/// Vidéo (pellicule).
pub fn video() -> AnyView {
    svg(r#"<rect x="2" y="3.5" width="9" height="9" rx="1.5"/><path d="m11 7 3-2v6l-3-2"/>"#)
}

/// Son (note).
pub fn audio() -> AnyView {
    svg(
        r#"<path d="M6 12V4l7-1.5v8"/><circle cx="4.5" cy="12" r="1.5"/><circle cx="11.5" cy="10.5" r="1.5"/>"#,
    )
}

/// Document (feuille cornée).
pub fn file() -> AnyView {
    svg(r#"<path d="M4 2h5l3 3v9H4z"/><path d="M9 2v3h3M6 8.5h4M6 11h4"/>"#)
}

/// Police de caractères.
pub fn font() -> AnyView {
    svg(r#"<path d="M3 13 6.5 3h1L11 13M4.3 9.5h5.4M11.5 13h2.5"/>"#)
}

/// Nouveau dossier.
pub fn folder_plus() -> AnyView {
    svg(
        r#"<path d="M2 4.5a1 1 0 0 1 1-1h3l1.5 1.5H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/><path d="M8 7v4M6 9h4"/>"#,
    )
}

/// Tri (flèches haut et bas).
pub fn sort() -> AnyView {
    svg(r#"<path d="M5 3v10M2.5 10.5 5 13l2.5-2.5M11 13V3M8.5 5.5 11 3l2.5 2.5"/>"#)
}

/// Filtre (entonnoir).
pub fn filter() -> AnyView {
    svg(r#"<path d="M2.5 3.5h11L9.5 8.5v4l-3 1.5V8.5z"/>"#)
}

/// Retour (chevron vers la gauche).
pub fn back() -> AnyView {
    svg(r#"<path d="M10 3.5 5.5 8l4.5 4.5"/>"#)
}

/// Brouillon (crayon barré d'un trait).
pub fn draft() -> AnyView {
    svg(r#"<path d="M3 13h3l7-7-3-3-7 7z"/><path d="M8.5 4.5l3 3"/>"#)
}

/// Visible (propriété de visibilité).
pub fn eye() -> AnyView {
    svg(
        r#"<path d="M2 8s2.25-4.25 6-4.25S14 8 14 8s-2.25 4.25-6 4.25S2 8 2 8z"/><circle cx="8" cy="8" r="1.75"/>"#,
    )
}

/// Détacher (une utilisation de composant).
pub fn unlink() -> AnyView {
    svg(
        r#"<path d="M7 4.5l1-1a2.8 2.8 0 0 1 4 4l-1 1M9 11.5l-1 1a2.8 2.8 0 0 1-4-4l1-1M2.5 2.5l2 2M13.5 13.5l-2-2"/>"#,
    )
}

/// Relier à une propriété de composant (point, comme Webflow).
pub fn prop_dot() -> AnyView {
    svg(r#"<circle cx="8" cy="8" r="5.25"/><circle cx="8" cy="8" r="1.75" fill="currentColor"/>"#)
}

/// Traduction automatique (D-36) : étincelle.
pub fn sparkle() -> AnyView {
    svg(
        r#"<path d="M7 2.5l1.2 3.3L11.5 7 8.2 8.2 7 11.5 5.8 8.2 2.5 7l3.3-1.2z"/><path d="M12.5 10.5v3M11 12h3"/>"#,
    )
}

/// Télécharger.
pub fn download() -> AnyView {
    svg(r#"<path d="M8 3v8M5 8l3 3 3-3M3 11v2h10v-2"/>"#)
}
