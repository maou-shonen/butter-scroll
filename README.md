[繁體中文](README.zh.md)

# butter-scroll

Buttery smooth mouse wheel scrolling for Windows.

butter-scroll is a lightweight system tray utility that intercepts mouse wheel and keyboard scroll events at the OS level, applies pulse easing animation, and re-injects smoothed events — making every app scroll like butter.

## Features

- **Smooth scrolling** — Pulse easing curve (ease-out feel) replaces the default jerky wheel behavior
- **Keyboard scrolling** — Optional smooth scrolling for Page Up/Down, Arrow keys, Space
- **Acceleration** — Rapid consecutive scrolls get a speed boost
- **Per-app adaptive output** — Automatically detects the best injection threshold for each app
- **System tray** — Runs quietly in the background; settings UI via tray icon
- **Auto-update** — Built-in update checker via GitHub Releases
- **TOML config** — All settings in a single human-readable config file

## Install

Download the latest `.exe` installer from [Releases](https://github.com/maou-shonen/butter-scroll/releases).

## Quick Start

1. Run the installer — butter-scroll starts in the system tray
2. Right-click the tray icon → **Settings** to open the configuration panel
3. Adjust scroll feel (animation time, step size, pulse intensity) and click **Save**
4. To start on login, enable **Autostart** in General settings

Configuration is stored in `%APPDATA%\com.butter-scroll.app\config.toml`.  
Delete the file to reset to defaults.

## Development

Prerequisites: [mise](https://mise.jdx.dev/) (manages Rust and Node.js), [pnpm](https://pnpm.io/)

```sh
pnpm install
mise run dev       # Start dev server (Tauri + Vite HMR)
mise run test      # Run Rust tests
mise run check     # Cargo check
mise run clippy    # Lint
mise run build     # Build NSIS installer
```

For detailed configuration options, see [docs/configuration.md](docs/configuration.md).

> [!NOTE]
> This project is entirely written by coding agents (AI). The maintainer reviews logic and direction but does not write Rust code directly.

## Acknowledgments

The scroll animation algorithm is based on [SmoothScroll](https://github.com/gblazex/smoothscroll) by [@gblazex](https://github.com/gblazex), a Chrome extension that brings smooth scrolling to browsers. butter-scroll ports its pulse easing curve (Michael Herf algorithm) to the Windows system level.

## License

MIT
