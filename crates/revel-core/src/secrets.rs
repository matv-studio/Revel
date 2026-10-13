//! Mots de passe et jetons : trousseau du système (Windows Credential Manager,
//! trousseau macOS, Secret Service sous Linux). Jamais écrits dans un fichier de Revel.

use std::collections::HashMap;
use std::sync::Mutex;

pub const SERVICE: &str = "app.revel-studio.desktop";

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

/// Trousseau du système.
#[cfg(feature = "keyring")]
pub struct Keyring;

#[cfg(feature = "keyring")]
impl SecretStore for Keyring {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let e = keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
        match e.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let e = keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
        match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Stockage en mémoire (essais, ligne de commande).
#[derive(Default)]
pub struct Memory(Mutex<HashMap<String, String>>);

impl SecretStore for Memory {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().map_err(|e| e.to_string())?.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        self.0.lock().map_err(|e| e.to_string())?.remove(key);
        Ok(())
    }
}
