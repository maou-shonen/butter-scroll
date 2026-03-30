#![allow(dead_code)]

mod app;
mod config;
mod detector;
mod engine;
pub mod foreground;
mod injector;
mod pulse;
mod state;
mod threshold;
mod traits;
mod util;

#[cfg(target_os = "windows")]
mod commands;
mod detector_win;
#[cfg(target_os = "windows")]
mod hook;
#[cfg(target_os = "windows")]
pub mod hotkey;
#[cfg(all(test, not(target_os = "windows")))]
#[path = "hotkey.rs"]
mod hotkey_test_module;
#[cfg(target_os = "windows")]
mod keyboard_hook;
#[cfg(target_os = "windows")]
mod resolve_win;
#[cfg(target_os = "windows")]
mod tray;

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

/// Check if running in portable mode.
///
/// Portable mode is detected by a `.portable` marker file next to the executable.
/// In portable mode, all data (config, cache) is stored next to the exe instead
/// of `%APPDATA%`, and the NSIS auto-updater is skipped.
fn is_portable() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(".portable").exists()))
        .unwrap_or(false)
}

/// Resolve the exe-relative directory (used for portable mode and as fallback).
fn exe_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn run() {
    #[cfg(target_os = "windows")]
    {
        use std::sync::{Arc, Mutex};

        use tauri::Manager;

        use crate::config::ConfigStore;

        let portable = is_portable();

        // Portable mode: store everything next to the exe.
        // Installed mode: use %APPDATA%\com.butter-scroll.app\
        let config_dir = if portable {
            log::info!("[config] portable mode detected");
            exe_dir()
        } else {
            std::env::var("APPDATA")
                .map(|p| std::path::PathBuf::from(p).join("com.butter-scroll.app"))
                .unwrap_or_else(|_| exe_dir())
        };

        let config_path = config_dir.join("config.toml");

        // Migrate old exe-relative config → %APPDATA% (installed mode only)
        if !portable && !config_path.exists() {
            let old_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("config.toml")));
            if let Some(old) = old_path {
                if old.exists() {
                    if let Err(e) = std::fs::create_dir_all(&config_dir) {
                        log::warn!("[config] failed to create app data dir: {e}");
                    } else {
                        match std::fs::copy(&old, &config_path) {
                            Ok(_) => log::info!(
                                "[config] migrated config from {:?} to {:?}",
                                old,
                                config_path
                            ),
                            Err(e) => log::warn!("[config] failed to migrate config: {e}"),
                        }
                    }
                }
            }
        }
        let _ = std::fs::create_dir_all(&config_dir);

        let config_store = Arc::new(crate::config::FileConfigStore::new(config_path));
        let config = config_store.load();

        let cache_path = config_dir.join("threshold_cache.json");

        // Migrate old threshold cache → %APPDATA% (installed mode only)
        if !portable && !cache_path.exists() {
            let old_cache = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("threshold_cache.json")));
            if let Some(old) = old_cache {
                if old.exists() {
                    if let Err(e) = std::fs::copy(&old, &cache_path) {
                        log::warn!("[config] failed to migrate threshold cache: {e}");
                    }
                }
            }
        }
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
        let _keyboard_hook =
            crate::keyboard_hook::KeyboardHook::install(engine_tx.clone(), config.keyboard.clone())
                .expect("Failed to install keyboard hook");

        let cache_for_save = Arc::clone(&threshold_cache);
        let cache_path_for_save = config_dir.join("threshold_cache.json");
        let engine_thread = std::thread::spawn(move || {
            engine.run();
            // Save threshold cache when engine stops
            if let Ok(cache) = cache_for_save.lock() {
                let _ = cache.save(&cache_path_for_save);
                log::info!("[engine] threshold cache saved on shutdown");
            }
        });

        let app_state = state::AppState {
            engine_tx: engine_tx.clone(),
            config_store: Arc::clone(&config_store) as Arc<dyn ConfigStore>,
            threshold_cache: Arc::clone(&threshold_cache),
            portable,
        };

        tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
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
                commands::get_default_config,
                commands::save_config,
                commands::toggle_enabled,
                commands::toggle_app_filter_entry,
                commands::toggle_keyboard,
                commands::toggle_autostart,
                commands::get_status,
                commands::check_for_updates,
                commands::show_confirm_dialog,
            ])
            .manage(app_state)
            .setup(move |app| {
                cleanup_old_autostart();
                tray::setup_tray(app.handle())?;

                // Delayed startup update check (installed mode only).
                // The NSIS-based updater is not compatible with portable installs.
                if !portable {
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
                }

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

        // Clean shutdown: stop engine and save threshold cache
        let _ = engine_tx.send(crate::traits::EngineCommand::Stop);
        let _ = engine_thread.join();
    }

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("butter-scroll 目前僅支援 Windows。測試可在非 Windows 平台執行。\n");
    }
}
