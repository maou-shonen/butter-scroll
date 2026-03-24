mod app;
mod autostart;
mod config;
mod engine;
mod hook;
mod injector;
mod pulse;
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
use crate::engine::ScrollEngine;
#[cfg(target_os = "windows")]
use crate::hook::MouseHook;
#[cfg(target_os = "windows")]
use crate::injector::WindowsScrollOutput;
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

    let mut engine = ScrollEngine::new(clock, output, config.clone(), engine_rx);
    let engine_thread = std::thread::spawn(move || engine.run());

    // 3) Install low-level mouse hook
    let _hook = MouseHook::install(engine_tx.clone())?;

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
                    let _ = store.save(&config);
                }
                AppCommand::ReloadConfig => {
                    config = store.load();
                    let _ = engine_tx.send(EngineCommand::Reload(Box::new(config.clone())));
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
