//! OS credential store helpers for saving and loading API keys.

#[allow(dead_code)]
const KEYRING_SERVICE: &str = "com.nevertiree.babel-ebook";

/// Stores the API key for a provider in the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn store_api_key(provider: String, api_key: String) -> Result<(), String> {
    let account = format!("api_key_{provider}");
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    entry.set_password(&api_key).map_err(|e| e.to_string())
}

/// Loads the API key for a provider from the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_api_key(provider: String) -> Result<Option<String>, String> {
    let account = format!("api_key_{provider}");
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Deletes the stored API key for a provider from the OS credential store.
#[allow(dead_code, clippy::needless_pass_by_value)]
#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    let account = format!("api_key_{provider}");
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
