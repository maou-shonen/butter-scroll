mod app;
mod autostart;
mod config;
mod detector;
mod detector_win;
mod engine;
mod hook;
mod injector;
mod keyboard_hook;
mod pulse;
mod resolve;
mod resolve_win;
mod threshold;
mod traits;
mod tray;
mod util;

#[cfg(target_os = "windows")]
use crate::app::AppCommand;
#[cfg(target_os = "windows")]
use crate::autostart::{AutoStartService, WindowsAutoStart};
#[cfg(target_os = "windows")]
use crate::config::{ConfigStore, FileConfigStore};
#[cfg(target_os = "windows")]
use crate::detector_win::WindowsScrollDetector;
#[cfg(target_os = "windows")]
use crate::engine::ScrollEngine;
#[cfg(target_os = "windows")]
use crate::hook::MouseHook;
#[cfg(target_os = "windows")]
use crate::injector::WindowsScrollOutput;
#[cfg(target_os = "windows")]
use crate::keyboard_hook::KeyboardHook;
#[cfg(target_os = "windows")]
use crate::resolve_win::WindowsProcessResolver;
#[cfg(target_os = "windows")]
use crate::traits::{EngineCommand, SystemClock};
#[cfg(target_os = "windows")]
use crossbeam_channel::unbounded;
#[cfg(target_os = "windows")]
use std::sync::Arc;

#[cfg(target_os = "windows")]
fn main() {
    if let Err(e) = run_windows() {
        eprintln!("[butter-scroll] fatal error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("butter-scroll 目前僅支援 Windows。測試可在非 Windows 平台執行。\n");
}

#[cfg(target_os = "windows")]
fn run_windows() -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
    };

    // 1) Load config
    let store = Arc::new(FileConfigStore::new(FileConfigStore::default_path()));
    eprintln!("[butter-scroll] config: {}", store.path().display());
    let mut config = store.load();

    // 2) Setup core channels + engine
    let (engine_tx, engine_rx) = unbounded::<EngineCommand>();
    let (app_tx, app_rx) = unbounded::<AppCommand>();

    let output = Arc::new(WindowsScrollOutput::new());
    let clock = Arc::new(SystemClock::new());
    let resolver = Arc::new(WindowsProcessResolver::new());
    let detector = Box::new(WindowsScrollDetector::new());

    // Load threshold cache from disk (alongside config file)
    let cache_path = store.path().with_file_name("threshold_cache.json");
    let threshold_cache = crate::threshold::AppThresholdCache::load(&cache_path);
    let threshold_cache = std::sync::Arc::new(std::sync::Mutex::new(threshold_cache));

    let mut engine = ScrollEngine::new(
        clock,
        output,
        resolver,
        detector,
        config.clone(),
        engine_tx.clone(),
        engine_rx,
    );
    engine.set_threshold_cache(threshold_cache.clone());
    let cache_path_for_thread = cache_path.clone();
    let cache_for_save = threshold_cache.clone();
    let engine_thread = std::thread::spawn(move || {
        engine.run();
        // Save cache on engine shutdown
        if let Ok(cache) = cache_for_save.lock() {
            let _ = cache.save(&cache_path_for_thread);
        }
    });

    // 3) Install low-level hooks
    let _mouse_hook = MouseHook::install(engine_tx.clone())?;
    let _keyboard_hook = KeyboardHook::install(engine_tx.clone(), config.keyboard.clone())?;
    // Ensure keyboard hook respects global enabled state at startup
    // (e.g. config has general.enabled=false + keyboard.enabled=true).
    sync_keyboard_hook(&config);

    // 4) Create system tray
    let _tray = tray::TrayIcon::create(app_tx)?;

    // 5) Sync autostart with config
    let autostart = WindowsAutoStart::new("SmoothScroll")?;
    if config.general.autostart != autostart.is_enabled() {
        let _ = autostart.set_enabled(config.general.autostart);
    }

    // 6) Main message loop
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        // SAFETY: standard Win32 message loop.
        let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        if ret <= 0 {
            break;
        }

        // SAFETY: `msg` initialized by GetMessageW.
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        while let Ok(cmd) = app_rx.try_recv() {
            match cmd {
                AppCommand::ToggleEnabled => {
                    config.general.enabled = !config.general.enabled;
                    let _ = engine_tx.send(EngineCommand::SetEnabled(config.general.enabled));
                    // When globally disabled, also pause keyboard hook so
                    // keys pass through natively (not converted to wheel).
                    sync_keyboard_hook(&config);
                    let _ = store.save(&config);
                }
                AppCommand::ToggleKeyboard => {
                    config.keyboard.enabled = !config.keyboard.enabled;
                    sync_keyboard_hook(&config);
                    let _ = store.save(&config);
                }
                AppCommand::ReloadConfig => {
                    config = store.load();
                    let _ = engine_tx.send(EngineCommand::Reload(Box::new(config.clone())));
                    sync_keyboard_hook(&config);
                }
                AppCommand::ToggleAutostart => {
                    let next = !autostart.is_enabled();
                    if autostart.set_enabled(next).is_ok() {
                        config.general.autostart = next;
                        let _ = store.save(&config);
                    }
                }
                AppCommand::Exit => {
                    // SAFETY: asks message loop to terminate.
                    unsafe { PostQuitMessage(0) };
                }
            }
        }
    }

    let _ = engine_tx.send(EngineCommand::Stop);
    let _ = engine_thread.join();

    Ok(())
}

/// Push the effective keyboard config to the hook, accounting for the
/// global `general.enabled` master switch.  When the app is globally
/// disabled, the keyboard hook must also stop intercepting keys so they
/// pass through with their native behaviour (not converted to wheel events).
#[cfg(target_os = "windows")]
fn sync_keyboard_hook(config: &crate::config::Config) {
    if config.general.enabled {
        KeyboardHook::update_config(&config.keyboard);
    } else {
        let mut paused = config.keyboard.clone();
        paused.enabled = false;
        KeyboardHook::update_config(&paused);
    }
}
