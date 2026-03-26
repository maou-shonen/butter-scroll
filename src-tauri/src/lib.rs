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

mod resolve;

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
            .plugin(
                tauri_plugin_log::Builder::new()
                    .level(log::LevelFilter::Info)
                    .build(),
            )
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .manage(app_state)
            .setup(|app| {
                let _tray = tauri::tray::TrayIconBuilder::new()
                    .tooltip("butter-scroll")
                    .build(app)?;
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
