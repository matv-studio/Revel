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
