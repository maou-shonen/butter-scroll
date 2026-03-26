[繁體中文](README.zh.md)

# butter-scroll

> [!NOTE]
> This project is entirely written by coding agents (AI). The maintainer reviews logic and direction but does not write Rust code directly.

Buttery smooth mouse wheel scrolling for Windows.

butter-scroll is a lightweight system tray utility that intercepts mouse wheel and keyboard scroll events at the OS level, applies easing animation, and re-injects smoothed events — making every app scroll like butter.

I couldn't find a free app that was good enough — so I built one.

## Features

- **Smooth scrolling** — Selectable easing curves (Pulse, OutCubic, OutQuint, OutExpo, OutCirc, OutBack) replace the default jerky wheel behavior
- **Keyboard scrolling** — Optional smooth scrolling for Page Up/Down, Arrow keys, Space
- **Acceleration** — Rapid consecutive scrolls get a speed boost
- **Per-app adaptive output** — Automatically detects the best injection threshold for each app
- **System tray** — Runs quietly in the background; settings UI via tray icon
- **Auto-update** — Built-in update checker via GitHub Releases
- **TOML config** — All settings in a single human-readable config file

## Install

Download the latest installer or portable ZIP from [Releases](https://github.com/maou-shonen/butter-scroll/releases).

| Package | Description |
|---|---|
| `*-setup.exe` | NSIS installer — installs to user profile, includes auto-update |
| `*-portable.zip` | Portable — extract anywhere, no installation required |

> [!NOTE]
> The portable version requires [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10 22H2+ and Windows 11). Auto-update is not available in portable mode.

## Quick Start

1. Run the installer (or extract the portable ZIP) — butter-scroll starts in the system tray
2. Right-click the tray icon → **Settings** to open the configuration panel
3. Adjust scroll feel (easing curve, animation time, step size) and click **Save**
4. To start on login, enable **Autostart** in General settings

Configuration is stored in `%APPDATA%\com.butter-scroll.app\config.toml`.  
In portable mode, configuration is stored next to the executable.  
Delete the file to reset to defaults.

## Development

Prerequisites: [mise](https://mise.jdx.dev/) (manages Rust and Node.js), [pnpm](https://pnpm.io/)

```sh
pnpm install
mise run dev       # Start dev server (Tauri + Vite HMR)
mise run test      # Run Rust tests
mise run check     # Cargo check
mise run clippy    # Lint
mise run build     # Build NSIS installer + release exe
```

For detailed configuration options, see [docs/configuration.md](docs/configuration.md).

## Acknowledgments

The default scroll animation algorithm (Pulse) is based on [SmoothScroll](https://github.com/gblazex/smoothscroll) by [@gblazex](https://github.com/gblazex), a Chrome extension that brings smooth scrolling to browsers. butter-scroll ports its pulse easing curve (Michael Herf algorithm) to the Windows system level, and also offers several standard Penner easing functions as alternatives.

## License

MIT
