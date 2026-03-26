# AGENTS.md

Windows system-level smooth scrolling utility.  
Intercepts mouse wheel / keyboard scroll events via low-level hooks, applies pulse easing animation, and re-injects smoothed wheel events through `SendInput`.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Tauri app shell  (lib.rs)                          │
│  - Initializes hooks, engine, config, tray, plugins │
│  - Manages Tauri window lifecycle                   │
├──────────────┬──────────────────────────────────────┤
│  Mouse Hook  │  Keyboard Hook                      │
│  (hook.rs)   │  (keyboard_hook.rs)                 │
│  WH_MOUSE_LL │  WH_KEYBOARD_LL                     │
│              │  Per-key-group modes, scrollbar check│
├──────────────┴──────────────────────────────────────┤
│  ScrollEngine  (engine.rs)  — core animation loop   │
│  - Receives EngineCommand via crossbeam channel     │
│  - Pulse easing (pulse.rs) + acceleration           │
│  - Accumulates fractional delta, flushes at threshold│
│  - Adaptive per-app threshold detection (detector/) │
├─────────────────────────────────────────────────────┤
│  Injector  (injector.rs)                            │
│  - SendInput(MOUSEEVENTF_WHEEL) output              │
├─────────────────────────────────────────────────────┤
│  Config  (config.rs + config.default.toml)          │
│  - TOML config with hot-reload via EngineCommand    │
│  - Per-app threshold overrides + cache              │
├─────────────────────────────────────────────────────┤
│  Frontend  (src/ — Svelte 5)                        │
│  - Settings UI: scroll, acceleration, output,       │
│    keyboard, general                                │
│  - Communicates via Tauri IPC commands              │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
src-tauri/                  Rust backend (Tauri app)
  src/
    lib.rs                  App entry — wires hooks, engine, Tauri plugins
    main.rs                 Binary entry point
    engine.rs               Core scroll animation loop (platform-agnostic algorithm)
    pulse.rs                Pulse easing curve (Michael Herf algorithm)
    hook.rs                 WH_MOUSE_LL low-level mouse hook
    keyboard_hook.rs        WH_KEYBOARD_LL low-level keyboard hook
    injector.rs             SendInput wheel event injection
    config.rs               TOML config loading, validation, serialization
    commands.rs             Tauri IPC command handlers (get_config, save_config, etc.)
    tray.rs                 System tray icon and menu setup
    traits.rs               DI traits (TimeSource, ScrollOutput) + EngineCommand enum
    state.rs                Shared Tauri app state
    threshold.rs            Per-app adaptive threshold detection cache
    detector.rs             Scroll detector trait
    detector_win.rs         Windows scroll detector (foreground window analysis)
    resolve.rs              Process resolver trait
    resolve_win.rs          Windows process resolver (PID → exe path)
    util.rs                 Helpers (wide string conversion, etc.)
  Cargo.toml
  tauri.conf.json           Tauri config (window, tray, bundle, updater)

src/                        Frontend (Svelte 5 + TypeScript)
  App.svelte                Root component — settings panel
  main.ts                   Svelte mount
  lib/
    api.ts                  Tauri invoke wrappers
    types.ts                TypeScript config types (mirrors Rust Config)
    ScrollSettings.svelte
    AccelerationSettings.svelte
    OutputSettings.svelte
    KeyboardSettings.svelte
    GeneralSettings.svelte

config.default.toml         Default configuration (embedded + reference)
mise.toml                   Dev tooling tasks (dev, test, check, clippy, build)
.github/workflows/build.yml CI/CD — test on Linux, build NSIS installer + portable ZIP on Windows
```

## Frameworks & Tools

| Tool | Purpose |
|---|---|
| **Tauri v2** | App shell — window, tray, IPC, bundler (NSIS), auto-update |
| **Rust 1.94** | Backend language |
| **windows-sys 0.59** | Win32 API bindings (hooks, SendInput, registry, window queries) |
| **crossbeam-channel** | Lock-free MPSC channel between hook threads and engine |
| **serde + toml** | Config serialization / deserialization |
| **Svelte 5** | Frontend UI framework (settings panel) |
| **Vite 6** | Frontend bundler with HMR |
| **TypeScript 5** | Frontend type safety |
| **pnpm** | Node.js package manager |
| **mise** | Dev tool version manager + task runner |
| **GitHub Actions** | CI (test/lint on Linux) + CD (build/release on Windows) |

## Key Concepts

- **Pulse easing**: Port of the SmoothScroll Chrome extension algorithm (gblazex/smoothscroll). Provides ease-out animation feel.
- **Adaptive threshold**: The engine auto-detects whether each app handles sub-`WHEEL_DELTA` (120) events correctly. Apps that can't (e.g. some WPF apps) get bumped to `threshold=120`. Results cached to disk.
- **Keyboard scroll modes**: `"off"` / `"always"` / `"win32_scrollbar"` — per-key-group (Page Up/Down, Arrow keys, Space). Keyboard events are converted to `ScrollRaw` commands that bypass mouse step_size normalization.
- **Config hot-reload**: Frontend saves config → Tauri command → engine receives `EngineCommand::Reload` → applies immediately without restart.

## Development Commands

```sh
mise run dev       # Tauri dev server with HMR
mise run test      # cargo test --lib
mise run check     # cargo check
mise run clippy    # cargo clippy -- -D warnings
mise run fmt       # cargo fmt
mise run build     # Build NSIS installer + release exe
mise run verify    # Build frontend + run tests
```

## Notes

- Windows-only at runtime (`cfg(target_os = "windows")`). Tests run cross-platform thanks to DI traits.
- The scroll engine (`engine.rs`) is the largest and most complex module (~1300 lines). It is platform-agnostic by design — all Win32 calls go through trait objects.
- Config file location: `%APPDATA%\com.butter-scroll.app\config.toml`
- **Portable mode**: When a `.portable` marker file exists next to the exe, all data (config, threshold cache) is stored next to the exe. The NSIS auto-updater is skipped. CI produces a `*-portable.zip` containing the exe + `.portable` marker.
