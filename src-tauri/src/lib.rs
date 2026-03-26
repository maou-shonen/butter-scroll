#![allow(dead_code)]

mod app;
mod config;
mod traits;
mod engine;
mod pulse;
mod threshold;
mod detector;
mod injector;
mod util;
mod state;

#[cfg(target_os = "windows")]
mod hook;
#[cfg(target_os = "windows")]
mod keyboard_hook;
mod detector_win;
#[cfg(target_os = "windows")]
mod resolve_win;
#[cfg(target_os = "windows")]
mod tray;
#[cfg(target_os = "windows")]
mod commands;

mod resolve;

#[cfg(target_os = "windows")]
fn cleanup_old_autostart() {
    use crate::util::to_wide;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    let subkey = to_wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let mut hkey = std::ptr::null_mut();

    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
    };

    if result == 0 {
        let value_name = to_wide("SmoothScroll");
        unsafe {
            RegDeleteValueW(hkey, value_name.as_ptr());
            RegCloseKey(hkey);
        }
    }
}

pub fn run() {
    #[cfg(target_os = "windows")]
    {
        use std::sync::{Arc, Mutex};

        use tauri::Manager;

        let config_path = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("config.toml")))
            .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));

        let config_store = Arc::new(crate::config::FileConfigStore::new(config_path));
        let config = config_store.load();

        let cache_path = config_store
            .path()
            .parent()
            .map(|dir| dir.join("threshold_cache.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("threshold_cache.json"));
        let threshold_cache = Arc::new(Mutex::new(crate::threshold::AppThresholdCache::load(
            &cache_path,
        )));

        let (engine_tx, engine_rx) = crossbeam_channel::unbounded::<crate::traits::EngineCommand>();

        let time = Arc::new(crate::traits::SystemClock::new());
        let scroll_output = Arc::new(crate::injector::WindowsScrollOutput::new());
        let process_resolver = Arc::new(crate::resolve_win::WindowsProcessResolver::new());
        let scroll_detector = Box::new(crate::detector_win::WindowsScrollDetector::new());

        let mut engine = crate::engine::ScrollEngine::new(
            time,
            scroll_output,
            process_resolver,
            scroll_detector,
            config.clone(),
            engine_tx.clone(),
            engine_rx,
        );
        engine.set_threshold_cache(Arc::clone(&threshold_cache));

        let _mouse_hook = crate::hook::MouseHook::install(engine_tx.clone())
            .expect("Failed to install mouse hook");
        let _keyboard_hook = crate::keyboard_hook::KeyboardHook::install(
            engine_tx.clone(),
            config.keyboard.clone(),
        )
        .expect("Failed to install keyboard hook");

        let _engine_thread = std::thread::spawn(move || {
            engine.run();
        });

        let app_state = state::AppState {
            engine_tx: engine_tx.clone(),
            config_store: Arc::clone(&config_store) as Arc<dyn crate::config::ConfigStore>,
            threshold_cache: Arc::clone(&threshold_cache),
        };

        tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_focus();
                }
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))
            .plugin(
                tauri_plugin_log::Builder::new()
                    .level(log::LevelFilter::Info)
                    .build(),
            )
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .invoke_handler(tauri::generate_handler![
                commands::get_config,
                commands::save_config,
                commands::toggle_enabled,
                commands::toggle_keyboard,
                commands::toggle_autostart,
                commands::get_status,
                commands::check_for_updates,
            ])
            .manage(app_state)
            .setup(|app| {
                cleanup_old_autostart();
                tray::setup_tray(app.handle())?;

                // Delayed startup update check (5 seconds after launch)
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    tauri::async_runtime::block_on(async {
                        use tauri_plugin_updater::UpdaterExt;
                        if let Ok(updater) = handle.updater() {
                            if let Ok(Some(update)) = updater.check().await {
                                log::info!("[updater] update available: {}", update.version);
                            }
                        }
                    });
                });

                Ok(())
            })
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("butter-scroll 目前僅支援 Windows。測試可在非 Windows 平台執行。\n");
    }
}
