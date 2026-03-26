use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::config::{Config, ConfigStore};
use crate::state::AppState;
use crate::traits::EngineCommand;

/// Sync keyboard hook state — pauses it when global smooth scrolling is disabled.
fn sync_keyboard_hook(config: &Config) {
    if config.general.enabled {
        crate::keyboard_hook::KeyboardHook::update_config(&config.keyboard);
    } else {
        let mut paused = config.keyboard.clone();
        paused.enabled = false;
        crate::keyboard_hook::KeyboardHook::update_config(&paused);
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppStatus {
    pub enabled: bool,
    pub keyboard_enabled: bool,
    pub autostart_enabled: bool,
}

/// Returns the current configuration.
#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, String> {
    Ok(state.config_store.load())
}

/// Saves configuration and hot-reloads engine.
#[tauri::command]
pub fn save_config(config: Config, state: State<AppState>) -> Result<(), String> {
    let mut config = config;
    config.sanitize();
    state.config_store.save(&config)?;

    // Send reload to engine
    state
        .engine_tx
        .send(EngineCommand::Reload(Box::new(config.clone())))
        .map_err(|e| e.to_string())?;

    // Hot-reload keyboard hook — respects global enabled state
    sync_keyboard_hook(&config);

    Ok(())
}

/// Toggles scroll smoothing on/off. Returns new enabled state.
#[tauri::command]
pub fn toggle_enabled(state: State<AppState>) -> Result<bool, String> {
    let mut config = state.config_store.load();
    config.general.enabled = !config.general.enabled;
    let new_state = config.general.enabled;
    state.config_store.save(&config)?;
    state
        .engine_tx
        .send(EngineCommand::SetEnabled(new_state))
        .map_err(|e| e.to_string())?;
    sync_keyboard_hook(&config);
    Ok(new_state)
}

/// Toggles keyboard smooth scrolling. Returns new enabled state.
#[tauri::command]
pub fn toggle_keyboard(state: State<AppState>) -> Result<bool, String> {
    let mut config = state.config_store.load();
    config.keyboard.enabled = !config.keyboard.enabled;
    let new_state = config.keyboard.enabled;
    state.config_store.save(&config)?;
    state
        .engine_tx
        .send(EngineCommand::Reload(Box::new(config.clone())))
        .map_err(|e| e.to_string())?;
    sync_keyboard_hook(&config);
    Ok(new_state)
}

/// Toggles autostart. Returns new state.
#[tauri::command]
pub fn toggle_autostart(state: State<AppState>, app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;

    let autostart_manager = app.autolaunch();
    let is_currently_enabled = autostart_manager.is_enabled().unwrap_or(false);

    if is_currently_enabled {
        autostart_manager.disable().map_err(|e| e.to_string())?;
    } else {
        autostart_manager.enable().map_err(|e| e.to_string())?;
    }

    let new_state = !is_currently_enabled;

    let mut config = state.config_store.load();
    config.general.autostart = new_state;
    if let Err(e) = state.config_store.save(&config) {
        // Rollback OS autostart state on config save failure
        if new_state {
            let _ = autostart_manager.disable();
        } else {
            let _ = autostart_manager.enable();
        }
        return Err(e);
    }
    Ok(new_state)
}

/// Returns current app status for UI initialization.
#[tauri::command]
pub fn get_status(state: State<AppState>) -> Result<AppStatus, String> {
    let config = state.config_store.load();
    Ok(AppStatus {
        enabled: config.general.enabled,
        keyboard_enabled: config.keyboard.enabled,
        autostart_enabled: config.general.autostart,
    })
}

/// Manually triggers an update check. Used by UI "Check for Updates" button.
/// Returns an error in portable mode (NSIS updater is not compatible).
#[tauri::command]
pub async fn check_for_updates(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    if state.portable {
        return Err("Auto-update is not available in portable mode.".to_string());
    }

    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    Ok(update.is_some())
}
