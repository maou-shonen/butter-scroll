use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl};

use crate::config::{Config, ConfigStore, HotkeyConfig};
use crate::state::AppState;
use crate::traits::EngineCommand;

#[derive(Debug, Clone, Serialize)]
pub struct ToggleResult {
    pub action: String,
    pub exe_path: String,
    pub mode: String,
    pub list_count: usize,
}

/// Sync keyboard hook state — pauses it when global smooth scrolling is disabled.
#[cfg(target_os = "windows")]
fn sync_keyboard_hook(config: &Config) {
    if config.general.enabled {
        crate::keyboard_hook::KeyboardHook::update_config(&config.keyboard);
    } else {
        let mut paused = config.keyboard.clone();
        paused.enabled = false;
        crate::keyboard_hook::KeyboardHook::update_config(&paused);
    }
}

/// No-op sync on non-Windows targets so tests and diagnostics can run.
#[cfg(not(target_os = "windows"))]
fn sync_keyboard_hook(_config: &Config) {}

#[cfg(target_os = "windows")]
pub(crate) fn build_hotkey_manager(
    app: &AppHandle,
    state: &AppState,
    combo: &str,
) -> Result<crate::hotkey::HotkeyManager, String> {
    let app_handle = app.clone();
    let config_store = std::sync::Arc::clone(&state.config_store);
    let capture = crate::foreground::WindowsForegroundCapture::new();

    crate::hotkey::HotkeyManager::new(combo, move || {
        let foreground = match crate::foreground::capture_filtered(&capture) {
            Some(app) => app,
            None => {
                log::warn!("[hotkey] could not identify foreground app");
                return;
            }
        };

        let config = config_store.load();
        let app_filter = match config.app_filter {
            Some(app_filter) => app_filter,
            None => {
                log::warn!("[hotkey] app filter not configured");
                return;
            }
        };

        let in_list = app_filter
            .list
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&foreground.exe_path));
        let mode = match app_filter.mode {
            crate::config::AppFilterMode::Blacklist => "blacklist",
            crate::config::AppFilterMode::Whitelist => "whitelist",
        }
        .to_string();

        let app_handle = app_handle.clone();
        let exe_path = foreground.exe_path;
        let app_name = foreground.app_name;
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                show_confirm_dialog(app_handle, exe_path, app_name, in_list, mode).await
            {
                log::warn!("[hotkey] failed to show confirmation dialog: {error}");
            }
        });
    })
}

#[cfg(target_os = "windows")]
fn sync_hotkey_manager(
    app: &AppHandle,
    state: &AppState,
    previous_hotkey: &HotkeyConfig,
    next_hotkey: &HotkeyConfig,
) {
    let mut hotkey_slot = match state.hotkey_manager.lock() {
        Ok(slot) => slot,
        Err(error) => {
            log::warn!("[hotkey] failed to lock hotkey manager state: {error}");
            return;
        }
    };

    if !next_hotkey.enabled {
        *hotkey_slot = None;
        return;
    }

    if !previous_hotkey.enabled {
        match build_hotkey_manager(app, state, &next_hotkey.combo) {
            Ok(manager) => *hotkey_slot = Some(manager),
            Err(error) => log::warn!(
                "[hotkey] failed to enable hotkey '{}': {error}",
                next_hotkey.combo
            ),
        }
        return;
    }

    match hotkey_slot.as_mut() {
        Some(manager) => {
            if previous_hotkey.combo != next_hotkey.combo {
                if let Err(error) = manager.update_combo(&next_hotkey.combo) {
                    log::warn!(
                        "[hotkey] failed to update hotkey combo '{}' -> '{}': {error}",
                        previous_hotkey.combo,
                        next_hotkey.combo
                    );
                }
            }
        }
        None => match build_hotkey_manager(app, state, &next_hotkey.combo) {
            Ok(manager) => *hotkey_slot = Some(manager),
            Err(error) => {
                log::warn!(
                    "[hotkey] hotkey enabled but manager missing; failed to recreate '{}': {error}",
                    next_hotkey.combo
                );
            }
        },
    }
}

#[cfg(not(target_os = "windows"))]
fn sync_hotkey_manager(
    _app: &AppHandle,
    _state: &AppState,
    _previous_hotkey: &HotkeyConfig,
    _next_hotkey: &HotkeyConfig,
) {
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

/// Returns the default configuration (for "Reset to Default" in UI).
#[tauri::command]
pub fn get_default_config() -> Config {
    Config::default()
}

/// Saves configuration and hot-reloads engine.
#[tauri::command]
pub fn save_config(config: Config, state: State<AppState>, app: AppHandle) -> Result<(), String> {
    let previous_config = state.config_store.load();

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

    sync_hotkey_manager(&app, &state, &previous_config.hotkey, &config.hotkey);

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

pub(crate) fn toggle_app_filter_entry_in_config(
    config: &mut Config,
    exe_path: String,
) -> Result<ToggleResult, String> {
    let (mode, action) = {
        let app_filter = config.app_filter.as_mut().ok_or_else(|| {
            "App filter not configured. Please choose blacklist/whitelist mode in Settings first."
                .to_string()
        })?;

        let mode = match app_filter.mode {
            crate::config::AppFilterMode::Blacklist => "blacklist",
            crate::config::AppFilterMode::Whitelist => "whitelist",
        }
        .to_string();

        let action = if app_filter
            .list
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&exe_path))
        {
            app_filter
                .list
                .retain(|item| !item.eq_ignore_ascii_case(&exe_path));
            "removed"
        } else {
            app_filter.list.push(exe_path.clone());
            "added"
        }
        .to_string();

        (mode, action)
    };

    config.sanitize();
    let list_count = config
        .app_filter
        .as_ref()
        .map(|app_filter| app_filter.list.len())
        .unwrap_or(0);

    Ok(ToggleResult {
        action,
        exe_path,
        mode,
        list_count,
    })
}

/// Toggles an app filter entry. Returns the resulting action and list state.
#[tauri::command]
pub fn toggle_app_filter_entry(
    exe_path: String,
    state: State<AppState>,
) -> Result<ToggleResult, String> {
    let mut config = state.config_store.load();
    let result = toggle_app_filter_entry_in_config(&mut config, exe_path)?;
    state.config_store.save(&config)?;
    state
        .engine_tx
        .send(EngineCommand::Reload(Box::new(config.clone())))
        .map_err(|e| e.to_string())?;
    Ok(result)
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

/// Shows the confirmation dialog for toggling an app filter entry.
/// Creates the window if it doesn't exist, or closes and recreates it with new params.
#[tauri::command]
pub async fn show_confirm_dialog(
    app: AppHandle,
    exe_path: String,
    app_name: String,
    in_list: bool,
    mode: String,
) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    let label = "confirm-app-filter";
    let url = format!(
        "confirm-app-filter.html?exe_path={}&app_name={}&in_list={}&mode={}",
        urlencoding::encode(&exe_path),
        urlencoding::encode(&app_name),
        in_list,
        mode
    );

    // Close existing window if present (simplest approach for fresh params)
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.close();
    }

    // Create new window with fresh params
    let _window = WebviewWindowBuilder::new(&app, label, WebviewUrl::App(url.into()))
        .title("確認")
        .inner_size(400.0, 220.0)
        .resizable(false)
        .center()
        .always_on_top(true)
        .visible(true)
        .skip_taskbar(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppFilterConfig, AppFilterMode};

    fn config_with_filter(mode: AppFilterMode, list: Vec<&str>) -> Config {
        let mut config = Config::default();
        config.app_filter = Some(AppFilterConfig {
            mode,
            list: list.into_iter().map(String::from).collect(),
        });
        config
    }

    #[test]
    fn toggle_adds_new_entry() {
        let mut config = config_with_filter(AppFilterMode::Blacklist, vec![]);

        let result =
            toggle_app_filter_entry_in_config(&mut config, "C:\\Windows\\calc.exe".to_string())
                .expect("toggle should succeed");

        assert_eq!(result.action, "added");
        assert_eq!(result.exe_path, "C:\\Windows\\calc.exe");
        assert_eq!(result.mode, "blacklist");
        assert_eq!(result.list_count, 1);
        assert_eq!(
            config.app_filter.unwrap().list,
            vec!["C:\\Windows\\calc.exe"]
        );
    }

    #[test]
    fn toggle_removes_existing_entry() {
        let mut config =
            config_with_filter(AppFilterMode::Whitelist, vec!["C:\\Windows\\calc.exe"]);

        let result =
            toggle_app_filter_entry_in_config(&mut config, "C:\\Windows\\calc.exe".to_string())
                .expect("toggle should succeed");

        assert_eq!(result.action, "removed");
        assert_eq!(result.exe_path, "C:\\Windows\\calc.exe");
        assert_eq!(result.mode, "whitelist");
        assert_eq!(result.list_count, 0);
        assert!(config.app_filter.unwrap().list.is_empty());
    }

    #[test]
    fn toggle_matches_case_insensitively() {
        let mut config =
            config_with_filter(AppFilterMode::Blacklist, vec!["c:\\windows\\notepad.exe"]);

        let result =
            toggle_app_filter_entry_in_config(&mut config, "C:\\Windows\\NOTEPAD.EXE".to_string())
                .expect("toggle should succeed");

        assert_eq!(result.action, "removed");
        assert_eq!(result.list_count, 0);
        assert!(config.app_filter.unwrap().list.is_empty());
    }

    #[test]
    fn toggle_errors_when_unconfigured() {
        let mut config = Config::default();

        let err =
            toggle_app_filter_entry_in_config(&mut config, "C:\\Windows\\calc.exe".to_string())
                .expect_err("toggle should fail");

        assert_eq!(
            err,
            "App filter not configured. Please choose blacklist/whitelist mode in Settings first."
        );
    }
}
