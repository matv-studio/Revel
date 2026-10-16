// Pas de console supplémentaire sous Windows en version publiée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Relais des assistants IA (`revel --mcp`, MCP sur l'entrée et la sortie standard) : lancé par
    // l'assistant, il transmet chaque outil à l'application ouverte (D-41). Sous Windows, les
    // entrées et sorties fournies par l'assistant sont héritées même sans console.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(revel_app_lib::mcp_relay());
    }
    revel_app_lib::run()
}
