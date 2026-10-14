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

pub fn site() -> AnyView {
    svg(r#"<rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M2 6h12"/>"#)
}

pub fn build() -> AnyView {
    svg(r#"<path d="M3 13l4-4M9.5 2.5l4 4-5 5-4-4z"/>"#)
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
