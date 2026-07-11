//! OS credential store helpers for saving and loading API keys.
//!
//! API keys are stored under an account name derived from the provider *config
//! name* (`api_key_{name}`). This keeps keys for multiple configs of the same
//! provider type separate. Legacy entries keyed only by provider type
//! (`api_key_{provider}`) will continue to work as long as the config name
//! matches the provider type.

#[allow(dead_code)]
const KEYRING_SERVICE: &str = "com.nevertiree.babel-ebook";

fn account_name(name: &str) -> String {
    format!("api_key_{name}")
}

/// Stores the API key for a provider config in the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn store_api_key(name: String, api_key: String) -> Result<(), String> {
    let account = account_name(&name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    entry.set_password(&api_key).map_err(|e| e.to_string())
}

/// Loads the API key for a provider config from the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_api_key(name: String) -> Result<Option<String>, String> {
    let account = account_name(&name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Deletes the stored API key for a provider config from the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn delete_api_key(name: String) -> Result<(), String> {
    let account = account_name(&name);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
