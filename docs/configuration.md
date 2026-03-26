# Configuration Reference

butter-scroll stores its configuration at:

```
%APPDATA%\com.butter-scroll.app\config.toml
```

Delete the file to regenerate defaults on next launch.

Below is a complete reference of all settings. Default values are shown inline.

---

## `[scroll]` — Scroll Animation

| Key | Default | Range | Description |
|-----|---------|-------|-------------|
| `frame_rate` | `150` | 30–1000 | Animation updates per second (Hz). Higher = smoother animation but more CPU. 150 is a good balance; rarely needs changing |
| `animation_time` | `400` | 1–5000 | Duration of one scroll animation (ms). Shorter (~200) = snappier response; longer (~600) = more gradual deceleration |
| `step_size` | `100.0` | 1–2000 | Scroll distance per wheel notch. 100 ≈ Windows default 3-line scroll. Increase to cover more content per scroll |
| `pulse_scale` | `4.0` | 0.1–20 | Pulse easing intensity — controls "fast start, gradual stop" strength. Higher = more scroll front-loaded into early animation. 2.0 = gentle, 4.0 = default, 8.0 = aggressive |
| `pulse_normalize` | `1.0` | 0.1–10 | Pulse curve output amplitude. 1.0 = auto-normalized (animation completes fully). Usually no need to change |
| `inverted` | `false` | — | Invert scroll direction — macOS-style "natural scrolling" where wheel-down moves content up |

## `[acceleration]` — Rapid Scroll Boost

| Key | Default | Range | Description |
|-----|---------|-------|-------------|
| `delta_ms` | `50` | 1–500 | Time window (ms) for detecting consecutive scrolls |
| `max` | `3.0` | 1–20 | Maximum acceleration multiplier. 1.0 = disabled |

## `[output]` — Event Injection

| Key | Default | Range | Description |
|-----|---------|-------|-------------|
| `inject_threshold` | `"auto"` | `"auto"` or 1–120 | Controls granularity of injected wheel events. `"auto"` = adaptive per-app detection. `120` = most compatible. `1` = smoothest (modern apps only) |

### `[output.app_overrides]`

Per-app threshold overrides. Keys are full executable paths:

```toml
[output.app_overrides]
"C:\\Windows\\System32\\notepad.exe" = 120.0
"C:\\Program Files\\App\\modern.exe" = 1.0
```

## `[general]`

| Key | Default | Description |
|-----|---------|-------------|
| `autostart` | `false` | Start on Windows login |
| `enabled` | `true` | Master switch. `false` = all wheel events pass through unmodified |

## `[keyboard]` — Keyboard Smooth Scrolling

Intercepts keyboard scroll keys and replaces them with smoothed wheel events using the same animation engine.

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `true` | Master switch for keyboard smooth scrolling |
| `mode` | `"always"` | Default activation mode for all key groups (can be overridden per-group) |

### Modes

| Mode | Behavior |
|------|----------|
| `"off"` | Never intercept — key passes through unchanged |
| `"always"` | Always intercept and convert to smooth scroll |
| `"win32_scrollbar"` | Only intercept when the focused window has a standard Win32 scrollbar (`WS_VSCROLL`). Modern apps (Electron, browsers, WPF) use custom-drawn scrollbars and won't be detected |

### Key Groups

Each group inherits `mode` from `[keyboard].mode` unless explicitly overridden.

#### `[keyboard.page_up_down]`

Page Up / Page Down. Low conflict risk — these keys are almost exclusively used for scrolling.

#### `[keyboard.arrow_keys]`

Arrow Up / Down. **High conflict risk** — also used for cursor movement in editors, game input, etc. `Shift+Arrow` is always passed through for text selection.

Default: `mode = "off"`

#### `[keyboard.space]`

Space / Shift+Space. **Medium conflict risk** — Space scrolls in browsers but is character input elsewhere.

Default: `mode = "off"`

---

## Example Configuration

```toml
[scroll]
frame_rate = 150
animation_time = 400
step_size = 100.0
pulse_scale = 4.0

[acceleration]
delta_ms = 50
max = 3.0

[output]
inject_threshold = "auto"

[general]
autostart = true
enabled = true

[keyboard]
enabled = true
mode = "always"

[keyboard.page_up_down]
# inherits mode = "always"

[keyboard.arrow_keys]
mode = "off"

[keyboard.space]
mode = "off"
```
