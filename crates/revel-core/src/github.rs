//! Compte GitHub et publication sur GitHub Pages.

use crate::{Core, CoreError, Result};
use revel_git::github::{self, Client, Poll, Token};
use revel_git::publish::{Event, Extras, Target};
use revel_protocol::{DeployEvent, DeploySummary, GithubEvent, HostWarning};
use std::path::Path;
use std::sync::atomic::Ordering;

/// Jeton GitHub dans le trousseau (un seul compte connecté).
pub const TOKEN_KEY: &str = "github:token";

/// Préfixe d'adresse de dépôt accepté : `proprietaire/nom`.
fn valid_repo(full: &str) -> bool {
    let mut parts = full.split('/');
    let ok = |s: Option<&str>| {
        s.is_some_and(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

impl Core {
    fn github_token_raw(&self) -> Result<Option<Token>> {
        Ok(self
            .secrets
            .get(TOKEN_KEY)
            .map_err(CoreError::Secret)?
            .and_then(|t| serde_json::from_str(&t).ok()))
    }

    /// Jeton valide (renouvelé s'il expire bientôt) ; erreur si aucun compte n'est connecté.
    fn github_token(&self) -> Result<Token> {
        let t = self.github_token_raw()?.ok_or(CoreError::GithubSignedOut)?;
        if !t.needs_refresh(github::now()) {
            return Ok(t);
        }
        match github::refresh(&t) {
            Ok(n) => {
                self.store_github_token(&n)?;
                Ok(n)
            }
            Err(revel_git::GitError::SignedOut) => {
                self.github_disconnect()?;
                Err(CoreError::GithubSignedOut)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn store_github_token(&self, t: &Token) -> Result<()> {
        let v = serde_json::to_string(t).map_err(|e| CoreError::Other(e.to_string()))?;
        self.secrets.set(TOKEN_KEY, &v).map_err(CoreError::Secret)
    }

    /// Compte GitHub connecté (identifiant), sans requête réseau.
    pub fn github_account(&self) -> Option<String> {
        let has_token = self.secrets.get(TOKEN_KEY).ok().flatten().is_some();
        if !has_token {
            return None;
        }
        self.with_state(|s| s.github_login.clone())
    }

    /// Déconnecte le compte : le jeton est retiré du trousseau.
    pub fn github_disconnect(&self) -> Result<()> {
        self.secrets.delete(TOKEN_KEY).map_err(CoreError::Secret)?;
        self.with_state(|s| s.github_login = None);
        Ok(())
    }

    /// Interrompt une connexion en cours (`github_connect`).
    pub fn github_cancel(&self) {
        self.github_cancel.store(true, Ordering::SeqCst);
    }

    /// Connexion par autorisation sur github.com (device flow) : le code à saisir est envoyé à
    /// l'interface, puis Revel attend la validation de l'utilisateur.
    pub async fn github_connect(
        &self,
        on_event: &(dyn Fn(GithubEvent) + Send + Sync),
    ) -> Result<String> {
        let r = self.github_connect_inner(on_event).await;
        if let Err(e) = &r {
            on_event(GithubEvent::Failed(e.failure()));
        }
        r
    }

    async fn github_connect_inner(
        &self,
        on_event: &(dyn Fn(GithubEvent) + Send + Sync),
    ) -> Result<String> {
        self.github_cancel.store(false, Ordering::SeqCst);
        let code = blocking(github::request_device_code).await?;
        on_event(GithubEvent::Code {
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
            expires_in: code.expires_in,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
        let mut interval = code.interval.max(5);
        let token = loop {
            // Attente par pas d'une seconde pour réagir vite à « Annuler ».
            for _ in 0..interval {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if self.github_cancel.load(Ordering::SeqCst) {
                    return Err(CoreError::GithubCancelled);
                }
            }
            if std::time::Instant::now() > deadline {
                return Err(revel_git::GitError::CodeExpired.into());
            }
            let dc = code.device_code.clone();
            match blocking(move || github::poll_token(&dc)).await? {
                Poll::Granted(t) => break t,
                Poll::Pending => {}
                Poll::SlowDown(i) => interval = i.max(interval + 5),
            }
        };
        self.store_github_token(&token)?;
        let access = token.access_token.clone();
        let user = blocking(move || Client::new(&access).user()).await?;
        self.with_state(|s| s.github_login = Some(user.login.clone()));
        on_event(GithubEvent::Connected {
            login: user.login.clone(),
        });
        Ok(user.login)
    }

    /// Publication sur GitHub Pages (site déjà généré dans `out`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn deploy_github_pages(
        &self,
        site: &Path,
        out: &Path,
        environment: &str,
        repo: &str,
        branch: &str,
        custom_domain: Option<&str>,
        mut warnings: Vec<HostWarning>,
        dry_run: bool,
        on_event: &(dyn Fn(DeployEvent) + Send + Sync),
    ) -> Result<DeploySummary> {
        let repo = repo.trim().trim_end_matches(".git").to_string();
        if !valid_repo(&repo) {
            return Err(CoreError::GithubRepoNotFound(repo));
        }
        let branch = if branch.trim().is_empty() {
            "gh-pages"
        } else {
            branch.trim()
        }
        .to_string();
        let domain = custom_domain
            .map(|d| {
                d.trim()
                    .trim_start_matches("https://")
                    .trim_end_matches('/')
                    .to_string()
            })
            .filter(|d| !d.is_empty());
        let token = self.github_token()?;
        let access = token.access_token.clone();
        let full = repo.clone();
        let (user, info) = blocking(move || {
            let c = Client::new(&access);
            Ok::<_, revel_git::GitError>((c.user()?, c.repo(&full)?))
        })
        .await?;
        let info = info.ok_or_else(|| CoreError::GithubRepoNotFound(repo.clone()))?;
        if !info.permissions.as_ref().is_some_and(|p| p.push) {
            return Err(CoreError::GithubNoPush(repo.clone()));
        }
        // Avertissements propres à GitHub Pages.
        if info.private {
            warnings.push(HostWarning {
                blocking: false,
                key: "deploy-warn-public-repo".into(),
                args: vec![],
            });
        }
        let (owner, name) = repo.split_once('/').unwrap_or((&repo, ""));
        let user_site = name.eq_ignore_ascii_case(&format!("{owner}.github.io"));
        on_event(DeployEvent::Warnings { warnings });

        // Adresse de projet (`proprietaire.github.io/depot/`) : les adresses depuis la racine du
        // site sont préfixées par `/depot` dans une copie dédiée du site généré.
        let out = if domain.is_none() && !user_site {
            let dest = out.with_file_name(format!(
                "{}@github-pages",
                out.file_name().and_then(|n| n.to_str()).unwrap_or("site")
            ));
            let (src, d, prefix) = (out.to_path_buf(), dest.clone(), format!("/{name}"));
            tokio::task::spawn_blocking(move || revel_project::rebase_dir(&src, &d, &prefix))
                .await
                .map_err(|e| CoreError::Other(e.to_string()))??;
            dest
        } else {
            out.to_path_buf()
        };

        let cache = site
            .join(revel_project::CACHE_DIR)
            .join("git")
            .join(repo.replace('/', "__"));
        let url = info.clone_url.clone();
        let access = token.access_token.clone();
        let message = format!("Publication du site ({environment}) par Revel");
        let email = user.noreply_email();
        let login = user.login.clone();
        let extras = Extras {
            nojekyll: true,
            cname: domain.clone(),
        };
        let branch_c = branch.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DeployEvent>();
        let job = tokio::task::spawn_blocking(move || {
            let target = Target {
                url: &url,
                branch: &branch_c,
                token: Some(&access),
                author_name: &login,
                author_email: &email,
                message: &message,
            };
            revel_git::publish::publish_dir(&out, &cache, &target, &extras, dry_run, &|e| {
                let _ = tx.send(match e {
                    Event::Fetching { done, total } => DeployEvent::Fetching { done, total },
                    Event::Preparing { done, total } => DeployEvent::Preparing { done, total },
                    Event::Pushing { done, total, .. } => DeployEvent::Uploaded { done, total },
                });
            })
        });
        // Relais de la progression pendant l'envoi.
        while let Some(e) = rx.recv().await {
            on_event(e);
        }
        let report = job.await.map_err(|e| CoreError::Other(e.to_string()))??;

        let mut pages_enabled = false;
        let mut public_url = domain
            .as_ref()
            .map(|d| format!("https://{d}/"))
            .unwrap_or_else(|| github::default_pages_url(owner, name));
        if !dry_run {
            let access = token.access_token.clone();
            let full = repo.clone();
            let b = branch.clone();
            let d = domain.clone();
            let (enabled, html) = blocking(move || {
                let c = Client::new(&access);
                match c.pages(&full)? {
                    None => {
                        let p = c.enable_pages(&full, &b)?;
                        if d.is_some() {
                            c.update_pages(&full, &b, d.as_deref())?;
                        }
                        Ok::<_, revel_git::GitError>((true, p.html_url))
                    }
                    Some(p) => {
                        let same_source = p
                            .source
                            .as_ref()
                            .is_some_and(|s| s.branch == b && s.path == "/");
                        if !same_source || p.cname.as_deref() != d.as_deref() {
                            c.update_pages(&full, &b, d.as_deref())?;
                        }
                        Ok((false, p.html_url))
                    }
                }
            })
            .await?;
            pages_enabled = enabled;
            if domain.is_none()
                && let Some(h) = html
            {
                public_url = h;
            }
        }
        let s = DeploySummary {
            uploaded: report.added + report.modified,
            deleted: report.deleted,
            total_files: report.total_files,
            bytes: report.bytes,
            dry_run,
            server_key: String::new(),
            server_fingerprint: String::new(),
            key_recorded: false,
            url: Some(public_url),
            pages_enabled,
        };
        on_event(DeployEvent::Finished(s.clone()));
        Ok(s)
    }
}

/// Appel bloquant (réseau, Git) hors du moteur asynchrone.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> std::result::Result<T, revel_git::GitError> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depot_valide() {
        assert!(valid_repo("matv-studio/alunites-revel-lab"));
        assert!(valid_repo("a/b.github.io"));
        assert!(!valid_repo("matv-studio"));
        assert!(!valid_repo("a/b/c"));
        assert!(!valid_repo("a/b c"));
    }
}
