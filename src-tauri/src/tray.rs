use tauri::{
    menu::{CheckMenuItem, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use tauri_plugin_autostart::ManagerExt;

use crate::commands::show_confirm_dialog;
use crate::config::{Config, ConfigStore};
use crate::foreground::{capture_foreground_app, ForegroundApp};
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

/// Build the system tray icon with a full context menu.
///
/// Menu layout:
/// - 啟用平滑捲動 (checkbox)
/// - 鍵盤平滑捲動 (checkbox)
/// - ─────────────
/// - 切換當前應用程式
/// - ─────────────
/// - 開啟設定
/// - ─────────────
/// - 開機自動啟動 (checkbox)
/// - ─────────────
/// - 離開
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let config = state.config_store.load();

    // Checkable items — initial state from config
    let enabled_item = CheckMenuItem::with_id(
        app,
        "enabled",
        "啟用平滑捲動",
        true,
        config.general.enabled,
        None::<&str>,
    )?;

    let keyboard_item = CheckMenuItem::with_id(
        app,
        "keyboard",
        "鍵盤平滑捲動",
        true,
        config.keyboard.enabled,
        None::<&str>,
    )?;

    let toggle_current_app_item = MenuItem::with_id(
        app,
        "toggle_current_app",
        "切換當前應用程式",
        true,
        None::<&str>,
    )?;

    let settings_item = MenuItem::with_id(app, "settings", "開啟設定", true, None::<&str>)?;

    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "開機自動啟動",
        true,
        config.general.autostart,
        None::<&str>,
    )?;

    let exit_item = MenuItem::with_id(app, "exit", "離開", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .items(&[&enabled_item, &keyboard_item])
        .separator()
        .item(&toggle_current_app_item)
        .separator()
        .item(&settings_item)
        .separator()
        .item(&autostart_item)
        .separator()
        .item(&exit_item)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(tauri::include_image!("icons/icon.png"))
        .tooltip("butter-scroll")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id().as_ref() {
                "enabled" => {
                    let mut config = state.config_store.load();
                    config.general.enabled = !config.general.enabled;
                    if let Err(e) = state.config_store.save(&config) {
                        log::error!("[tray] failed to save config: {e}");
                        return;
                    }
                    if let Err(e) = state
                        .engine_tx
                        .send(EngineCommand::SetEnabled(config.general.enabled))
                    {
                        log::error!("[tray] failed to send engine command: {e}");
                    }
                    // Sync keyboard hook — pause it when globally disabled
                    sync_keyboard_hook(&config);
                }
                "keyboard" => {
                    let mut config = state.config_store.load();
                    config.keyboard.enabled = !config.keyboard.enabled;
                    if let Err(e) = state.config_store.save(&config) {
                        log::error!("[tray] failed to save config: {e}");
                        return;
                    }
                    // Engine reload first, then hook update (matches commands.rs ordering)
                    if let Err(e) = state
                        .engine_tx
                        .send(EngineCommand::Reload(Box::new(config.clone())))
                    {
                        log::error!("[tray] failed to send engine command: {e}");
                    }
                    sync_keyboard_hook(&config);
                }
                "toggle_current_app" => {
                    let Some(app_info) = capture_foreground_app() else {
                        log::warn!("[tray] could not identify foreground app");
                        return;
                    };

                    let config = state.config_store.load();
                    let Some(app_filter) = config.app_filter.as_ref() else {
                        log::warn!("[tray] app filter not configured");
                        return;
                    };

                    let in_list = app_filter
                        .list
                        .iter()
                        .any(|item| item.eq_ignore_ascii_case(&app_info.exe_path));

                    let mode = match app_filter.mode {
                        crate::config::AppFilterMode::Blacklist => "blacklist",
                        crate::config::AppFilterMode::Whitelist => "whitelist",
                    }
                    .to_string();

                    let ForegroundApp { exe_path, app_name } = app_info;

                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) =
                            show_confirm_dialog(app_handle, exe_path, app_name, in_list, mode).await
                        {
                            log::warn!("[tray] failed to show confirmation dialog: {e}");
                        }
                    });
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "autostart" => {
                    let autostart_manager = app.autolaunch();
                    let is_enabled = autostart_manager.is_enabled().unwrap_or(false);
                    let toggle_result = if is_enabled {
                        autostart_manager.disable()
                    } else {
                        autostart_manager.enable()
                    };
                    if let Err(e) = toggle_result {
                        log::error!("[tray] failed to toggle autostart: {e}");
                        return;
                    }
                    let new_state = !is_enabled;
                    let mut config = state.config_store.load();
                    config.general.autostart = new_state;
                    if let Err(e) = state.config_store.save(&config) {
                        log::error!("[tray] failed to save config, rolling back: {e}");
                        // Rollback OS autostart state
                        if new_state {
                            let _ = autostart_manager.disable();
                        } else {
                            let _ = autostart_manager.enable();
                        }
                    }
                }
                "exit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
