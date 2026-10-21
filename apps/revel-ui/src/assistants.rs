//! Réglages › Assistants IA (4.2, D-41) : serveur local (activé, port, état), clé d'accès,
//! configuration prête à copier pour chaque assistant, dernières actions des assistants.

use crate::dialog::Ask;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{McpActivity, McpStatus};

/// Assistants proposés : identifiant, nom (marque, non traduit).
const CLIENTS: &[(&str, &str)] = &[
    ("claude-desktop", "Claude Desktop"),
    ("claude-code", "Claude Code"),
    ("codex", "ChatGPT · Codex"),
    ("lm-studio", "LM Studio"),
    ("open-webui", "Open WebUI"),
];

/// Chaîne JSON (guillemets et barres obliques inverses échappés).
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// Programme sous Windows (chemin avec `\`).
fn windows(program: &str) -> bool {
    program.contains('\\')
}

/// Configuration à copier pour un assistant lancé en stdio (`<programme> --mcp`).
fn snippet(client: &str, program: &str) -> String {
    match client {
        "claude-code" => {
            format!("claude mcp add --transport stdio --scope user revel -- \"{program}\" --mcp")
        }
        // Chaîne littérale TOML (entre apostrophes) : chemin Windows sans échappement.
        "codex" => format!("[mcp_servers.revel]\ncommand = '{program}'\nargs = [\"--mcp\"]"),
        _ => format!(
            "{{\n  \"mcpServers\": {{\n    \"revel\": {{\n      \"command\": {},\n      \"args\": [\"--mcp\"]\n    }}\n  }}\n}}",
            json_str(program)
        ),
    }
}

/// Nom lisible d'un assistant (nom donné par l'assistant au protocole).
fn client_name(raw: &str) -> String {
    let l = raw.to_lowercase();
    let known = [
        ("claude-code", "Claude Code"),
        ("claude", "Claude"),
        ("codex", "Codex"),
        ("chatgpt", "ChatGPT"),
        ("openai", "ChatGPT"),
        ("lm-studio", "LM Studio"),
        ("lmstudio", "LM Studio"),
        ("open-webui", "Open WebUI"),
        ("openwebui", "Open WebUI"),
    ];
    known
        .iter()
        .find(|(k, _)| l.contains(k))
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// Action d'un assistant en clair : « Claude · Texte modifié ».
pub(crate) fn activity_label(ui: Ui, a: &McpActivity) -> String {
    let name = if a.client.trim().is_empty() {
        ui.now("mcp-client-unknown", &[])
    } else {
        client_name(&a.client)
    };
    format!("{name} · {}", ui.now(&format!("mcp-tool-{}", a.tool), &[]))
}

/// Heure d'une action (`14:32`, précédée du jour dans la langue de l'interface un autre jour).
pub(crate) fn when(at: u64, lang: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at as f64));
    let now = js_sys::Date::new_0();
    let hm = format!("{:02}:{:02}", d.get_hours(), d.get_minutes());
    if d.to_date_string() == now.to_date_string() {
        return hm;
    }
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"day".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"month".into(), &"2-digit".into());
    format!(
        "{} {hm}",
        String::from(d.to_locale_date_string(lang, &opts))
    )
}

/// Bouton « copier » : coche affichée un instant après la copie.
#[component]
pub(crate) fn CopyBtn(#[prop(into)] text: Signal<String>, label: &'static str) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let done = RwSignal::new(false);
    view! {
        <button class="icon-btn icon-btn--small copy-btn" class:is-done=move || done.get()
            title=ui.t(label) aria-label=ui.t(label)
            on:click=move |_| {
                api::copy_text(&text.get_untracked());
                done.set(true);
                set_timeout(move || { if !done.is_disposed() { done.set(false) } },
                    std::time::Duration::from_millis(1400));
            }>
            {move || if done.get() { icons::check() } else { icons::copy() }}
        </button>
    }
}

#[component]
pub fn Assistants() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let status = RwSignal::new(None::<McpStatus>);
    let acts = RwSignal::new(Vec::<McpActivity>::new());
    let busy = RwSignal::new(false);
    let shown = RwSignal::new(false);
    let client = RwSignal::new("claude-desktop");
    let load = move || {
        spawn_local(async move {
            match api::mcp_status().await {
                Ok(s) if !status.is_disposed() => status.set(Some(s)),
                Ok(_) => {}
                Err(e) => ui.fail(e),
            }
        })
    };
    let load_acts = move || {
        spawn_local(async move {
            if let Ok(a) = api::mcp_activity().await
                && !acts.is_disposed()
                && acts.with_untracked(|x| *x != a)
            {
                acts.set(a);
            }
        })
    };
    load();
    load_acts();
    // Dernières actions relues pendant que la page est ouverte.
    if let Ok(h) = set_interval_with_handle(load_acts, std::time::Duration::from_millis(2000)) {
        on_cleanup(move || h.clear());
    }
    let set = move |enabled: bool, port: u16| {
        busy.set(true);
        spawn_local(async move {
            match api::mcp_set(enabled, port).await {
                Ok(s) => status.set(Some(s)),
                Err(e) => ui.fail(e),
            }
            busy.set(false);
        });
    };
    let renew = move || {
        ui.ask.set(Some(Ask::confirm(
            ui.now("mcp-key-renew-title", &[]),
            None,
            ui.now("mcp-key-renew", &[]),
            move || {
                spawn_local(async move {
                    match api::mcp_new_token().await {
                        Ok(s) => status.set(Some(s)),
                        Err(e) => ui.fail(e),
                    }
                })
            },
        )));
    };
    let enabled = move || status.with(|s| s.as_ref().is_some_and(|s| s.enabled));
    let running = move || status.with(|s| s.as_ref().is_some_and(|s| s.running));
    let port = move || status.with(|s| s.as_ref().map(|s| s.port).unwrap_or_default());
    let token = Signal::derive(move || {
        status.with(|s| s.as_ref().map(|s| s.token.clone()).unwrap_or_default())
    });
    let program = move || {
        status.with(|s| {
            s.as_ref()
                .and_then(|s| s.program.clone())
                .unwrap_or_else(|| "revel".into())
        })
    };
    let url = Signal::derive(move || {
        status.with(|s| s.as_ref().map(|s| s.url.clone()).unwrap_or_default())
    });
    let docker_url = Signal::derive(move || format!("http://host.docker.internal:{}/mcp", port()));
    let spec_url = Signal::derive(move || format!("http://127.0.0.1:{}/openapi.json", port()));
    let code = Signal::derive(move || snippet(client.get(), &program()));
    // Emplacement de la configuration (chemin du fichier de Codex selon le système).
    let place = move || match client.get() {
        "codex" if windows(&program()) => "%USERPROFILE%\\.codex\\config.toml".to_string(),
        "codex" => "~/.codex/config.toml".to_string(),
        c => ui.tx(&format!("mcp-where-{c}"), &[]),
    };
    let state = move || {
        status.with(|s| match s {
            None => (String::new(), ""),
            Some(s) if s.running => (
                ui.tx("mcp-state-on", &[("addr", format!("127.0.0.1:{}", s.port))]),
                "is-on",
            ),
            Some(s) if s.enabled && s.error.is_some() => {
                let e = s.error.clone().unwrap_or_default();
                let busy_port = ["in use", "10048", "AddrInUse", "address already"]
                    .iter()
                    .any(|k| e.contains(k));
                (
                    ui.tx(
                        if busy_port {
                            "mcp-state-port-busy"
                        } else {
                            "mcp-state-failed"
                        },
                        &[],
                    ),
                    "is-error",
                )
            }
            Some(_) => (ui.tx("mcp-state-off", &[]), ""),
        })
    };
    let field = move |key: &'static str, value: Signal<String>, secret: bool| {
        view! {
            <div class="mcp-field">
                <span class="mcp-field-name">{ui.t(key)}</span>
                <code class="code mcp-field-value">{move || if secret && !shown.get() {
                    "•".repeat(24)
                } else {
                    value.get()
                }}</code>
                <CopyBtn text=value label="action-copy"/>
            </div>
        }
    };
    view! {
        <div class="section mcp">
            <h2 title=ui.t("settings-mcp-help")>{ui.t("settings-mcp")}</h2>
            <div class="form-grid">
                <span class="label" title=ui.t("mcp-server-help")>{ui.t("mcp-server")}</span>
                <div class="mcp-server">
                    <label class="switch">
                        <input type="checkbox" class="mcp-enable" prop:checked=enabled
                            prop:disabled=move || busy.get() || status.with(Option::is_none)
                            aria-label=ui.t("mcp-server")
                            on:change=move |e| set(event_target_checked(&e), port())/>
                        <span class="switch-track"></span>
                    </label>
                    <span class=move || format!("mcp-state {}", state().1)
                        title=move || status.with(|s| s.as_ref().and_then(|s| s.error.clone()).unwrap_or_default())>
                        <span class="mcp-dot"></span>{move || state().0}
                    </span>
                </div>
                <span class="label" title=ui.t("mcp-port-help")>{ui.t("mcp-port")}</span>
                <input class="input input--port mcp-port" type="number" min="1024" max="65535"
                    prop:value=move || port().to_string()
                    prop:disabled=move || busy.get()
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        match v.trim().parse::<u16>() {
                            Ok(p) if p >= 1024 && p != port() => set(enabled(), p),
                            _ => {
                                // Valeur refusée : port actuel rétabli.
                                let t: web_sys::HtmlInputElement = event_target(&e);
                                t.set_value(&port().to_string());
                            }
                        }
                    }/>
                <span class="label" title=ui.t("mcp-key-help")>{ui.t("mcp-key")}</span>
                <div class="mcp-key">
                    <code class="code mcp-key-value">{move || if shown.get() { token.get() } else { "•".repeat(24) }}</code>
                    <button class="icon-btn icon-btn--small mcp-key-show"
                        title=move || ui.tx(if shown.get() { "mcp-key-hide" } else { "mcp-key-show" }, &[])
                        aria-label=move || ui.tx(if shown.get() { "mcp-key-hide" } else { "mcp-key-show" }, &[])
                        on:click=move |_| shown.update(|s| *s = !*s)>
                        {move || if shown.get() { icons::eye_off() } else { icons::eye() }}
                    </button>
                    <CopyBtn text=token label="mcp-key-copy"/>
                    <button class="icon-btn icon-btn--small mcp-key-renew" title=ui.t("mcp-key-renew-help")
                        aria-label=ui.t("mcp-key-renew") on:click=move |_| renew()>{icons::reset()}</button>
                </div>
                <span class="label" title=ui.t("mcp-openapi-help")>"OpenAPI"</span>
                <div class="mcp-key mcp-openapi">
                    <code class="code mcp-key-value">{move || spec_url.get()}</code>
                    <CopyBtn text=spec_url label="action-copy"/>
                </div>
            </div>
            <div class="mcp-connect" class:is-off=move || !running()
                title=move || (!running()).then(|| ui.tx("mcp-connect-off", &[]))>
                <div class="mcp-connect-head">
                    <h3>{ui.t("mcp-connect")}</h3>
                    <div class="segmented mcp-clients" role="tablist">
                        {CLIENTS.iter().map(|(id, name)| {
                            let id: &'static str = id;
                            view! {
                                <button role="tab" data-client=id class:is-on=move || client.get() == id
                                    aria-selected=move || (client.get() == id).to_string()
                                    on:click=move |_| client.set(id)>{*name}</button>
                            }
                        }).collect_view()}
                    </div>
                </div>
                <p class="mcp-where" title=move || ui.tx(&format!("mcp-how-{}", client.get()), &[])>
                    {icons::info()}<span class="path">{place}</span>
                </p>
                {move || if client.get() == "open-webui" {
                    view! {
                        <div class="mcp-fields">
                            {field("mcp-field-type", Signal::derive(|| "MCP (Streamable HTTP)".to_string()), false)}
                            {field("mcp-field-url", url, false)}
                            {field("mcp-field-url-docker", docker_url, false)}
                            {field("mcp-field-auth", Signal::derive(|| "Bearer".to_string()), false)}
                            {field("mcp-field-key", token, true)}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="snippet">
                            <pre class="snippet-code">{move || code.get()}</pre>
                            <CopyBtn text=code label="action-copy"/>
                        </div>
                    }.into_any()
                }}
            </div>
            <div class="mcp-activity">
                <h3>{ui.t("mcp-activity")}</h3>
                {move || {
                    let list = acts.get();
                    if list.is_empty() {
                        view! { <p class="muted mcp-activity-empty">{ui.t("mcp-activity-empty")}</p> }.into_any()
                    } else {
                        view! {
                            <ul class="mcp-acts">
                                {list.into_iter().take(12).map(|a| {
                                    let full = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(a.at as f64))
                                        .to_locale_string(&ui.lang(), &wasm_bindgen::JsValue::UNDEFINED);
                                    let name = if a.client.trim().is_empty() {
                                        ui.now("mcp-client-unknown", &[])
                                    } else {
                                        client_name(&a.client)
                                    };
                                    view! {
                                        <li class="mcp-act" data-tool=a.tool.clone()>
                                            <span class="mcp-act-time" title=String::from(full)>{when(a.at, &ui.lang())}</span>
                                            <span class="mcp-act-client">{name}</span>
                                            <span class="mcp-act-what">{ui.tx(&format!("mcp-tool-{}", a.tool), &[])}</span>
                                            {a.target.map(|t| view! { <code class="code-tag mcp-act-target">{t}</code> })}
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configurations() {
        let win = r"C:\Program Files\Revel\revel.exe";
        let json = snippet("claude-desktop", win);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["mcpServers"]["revel"]["command"], win);
        assert_eq!(v["mcpServers"]["revel"]["args"][0], "--mcp");
        assert_eq!(snippet("lm-studio", win), json);
        assert_eq!(
            snippet("codex", win),
            "[mcp_servers.revel]\ncommand = 'C:\\Program Files\\Revel\\revel.exe'\nargs = [\"--mcp\"]"
        );
        assert!(
            snippet(
                "claude-code",
                "/Applications/Revel.app/Contents/MacOS/revel"
            )
            .ends_with("-- \"/Applications/Revel.app/Contents/MacOS/revel\" --mcp")
        );
        assert_eq!(client_name("claude-ai"), "Claude");
        assert_eq!(client_name("claude-code"), "Claude Code");
        assert_eq!(client_name("codex-mcp-client"), "Codex");
        assert_eq!(client_name("mon-outil"), "mon-outil");
    }
}
