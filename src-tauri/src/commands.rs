use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{Config, ConfigStore};
use crate::state::AppState;
use crate::traits::EngineCommand;

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

    // Hot-reload keyboard hook config
    crate::keyboard_hook::KeyboardHook::update_config(&config.keyboard);

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
    crate::keyboard_hook::KeyboardHook::update_config(&config.keyboard);
    Ok(new_state)
}

/// Toggles autostart. Returns new state.
/// NOTE: Actual registry modification is handled by tauri-plugin-autostart (T11).
/// This command updates the config store only. Tray and T11 handle registry.
#[tauri::command]
pub fn toggle_autostart(state: State<AppState>) -> Result<bool, String> {
    let mut config = state.config_store.load();
    config.general.autostart = !config.general.autostart;
    let new_state = config.general.autostart;
    state.config_store.save(&config)?;
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
