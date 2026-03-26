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
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("butter-scroll 目前僅支援 Windows。測試可在非 Windows 平台執行。\n");
    }
}
