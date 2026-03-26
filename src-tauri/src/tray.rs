use tauri::{
    menu::{CheckMenuItem, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::state::AppState;
use crate::traits::EngineCommand;

/// Build the system tray icon with a full context menu.
///
/// Menu layout:
/// - 啟用平滑捲動 (checkbox)
/// - 鍵盤平滑捲動 (checkbox)
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
        .item(&settings_item)
        .separator()
        .item(&autostart_item)
        .separator()
        .item(&exit_item)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .tooltip("butter-scroll")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id().as_ref() {
                "enabled" => {
                    let mut config = state.config_store.load();
                    config.general.enabled = !config.general.enabled;
                    let _ = state.config_store.save(&config);
                    let _ = state
                        .engine_tx
                        .send(EngineCommand::SetEnabled(config.general.enabled));
                }
                "keyboard" => {
                    let mut config = state.config_store.load();
                    config.keyboard.enabled = !config.keyboard.enabled;
                    let _ = state.config_store.save(&config);
                    let _ = state.engine_tx.send(EngineCommand::Reload(Box::new(config)));
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "autostart" => {
                    let mut config = state.config_store.load();
                    config.general.autostart = !config.general.autostart;
                    let _ = state.config_store.save(&config);
                    // Actual autostart registration handled by tauri-plugin-autostart (T11)
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
