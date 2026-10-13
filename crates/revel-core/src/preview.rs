//! Aperçu local : serveur HTTP sur 127.0.0.1 (port libre) servant un dossier généré.
//! `/a/` sert `a/index.html` ; une page absente sert `404.html` avec le code 404.

use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

pub struct Preview {
    pub url: String,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Preview {
    pub fn stop(&mut self) {
        if let Some(s) = self.stop.take() {
            let _ = s.send(());
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Démarre le serveur (à appeler dans un environnement tokio).
pub async fn start(dir: PathBuf) -> std::io::Result<Preview> {
    let not_found = ServeFile::new(dir.join("404.html"));
    let app = Router::new().fallback_service(
        ServeDir::new(&dir)
            .append_index_html_on_directories(true)
            .not_found_service(not_found),
    );
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
    });
    Ok(Preview {
        url: format!("http://{addr}/"),
        stop: Some(tx),
    })
}
