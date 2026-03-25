# Learnings

## [2026-03-25] Session Start

### Codebase Architecture
- 12 source files, ~2200 LOC, 35 tests (all passing), `cargo test`
- Engine is fully DI'd: `TimeSource`, `ScrollOutput` traits with mock implementations
- Engine is platform-agnostic: `engine.rs` has zero `#[cfg(target_os = "windows")]`
- All platform specifics live in `hook.rs`, `keyboard_hook.rs`, `injector.rs`
- Single `inject_threshold` read in exactly ONE place: `engine.rs:228` in `flush_pending()`
- `EngineCommand::Scroll` carries only `{delta: i16, horizontal: bool}` — no window context
- `MSLLHOOKSTRUCT` has `pt: POINT` (cursor coords) available at hook time
- `crossbeam_channel::unbounded()` — non-blocking, safe for LL hook callback
- `ConfigStore` trait already supports DI for testing

### Key Patterns
- Follow `traits.rs:15-18` for new trait definitions (minimal, DI-friendly)
- Follow `keyboard_hook.rs:314-349` for Win32 API usage in hooks
- Follow `engine.rs:440-800` for comprehensive unit tests with mocks
- eprintln!("[engine] ...") pattern for debug logging

### Windows API Notes
- `WindowFromPoint` NOT currently imported — needs `Win32_UI_WindowsAndMessaging`
- `GetWindowThreadProcessId` IS used in `keyboard_hook.rs` but NOT in `hook.rs`
- `GetScrollInfo`/`GetScrollBarInfo` — NOT imported, needs `Win32_UI_Controls` feature
- `QueryFullProcessImageNameW` — NOT imported, needs `Win32_System_Threading` feature
- `GetClassNameW` — needed for WPF detection (HwndWrapper class)

### Design Decisions
- ThresholdMode values: Unknown → 1.0, SmoothOk → 1.0, Legacy120 → 120.0, Detecting → 1.0
- Cache key: normalized exe path + version/mtime (not just exe name — too many collisions)
- WPF detection: class name `HwndWrapper[*]` pattern → Legacy120 (no Win32 scrollbar needed)
- GetScrollInfo probe: only for apps WITH WS_VSCROLL (standard Win32 apps)
- Settle polling: 20ms intervals, 2 consecutive same values, max 200ms
- Overflow detection: actual_delta > 5x expected_delta → Legacy120
- Detector MUST run on separate thread — never block 150Hz engine loop
- Override priority: user config > detected mode > default (1.0)

### [2026-03-25] Config Overrides
- `OutputConfig` now supports `app_overrides: HashMap<String, f64>` with serde defaults.
- `sanitize()` clamps per-app override values to the same 1.0–120.0 range as `inject_threshold`.
- TOML nested tables under `[output.app_overrides]` parse cleanly with Windows executable paths as keys.

### [2026-03-25] Task 2: Threshold Cache Module
- `threshold.rs` is pure Rust — zero platform deps, compiles on Linux without `#[cfg]` gates
- `AppKey` uses `PathBuf` + `Option<u64>` mtime — derives `Hash, Eq, PartialEq, Clone, Debug`
- `start_detecting()` dedup guard: only transitions from `Unknown` (or absent) → `Detecting`; returns false for any other state
- `ThresholdMode::threshold()` method on the enum itself keeps the value mapping co-located with the type
- `lookup_override()` is a stub returning `None` — will be wired to `Config.output.app_overrides` in Task 6
- `mod threshold;` placed alphabetically in `main.rs` between `pulse` and `traits`
- 7 tests covering all modes, dedup guard, stub override, and None key path

### [2026-03-25] Task 4: Scroll target PID plumbing
- `EngineCommand::Scroll` now carries `target_pid` end-to-end, with hook/keyboard callers using `0` as a placeholder for later PID capture.
- `ScrollEngine` caches the latest target PID in `current_target_pid` when handling scroll commands.
- Full `cargo test` stayed green after the signature change.

### [2026-03-25] Task 5: Hook PID capture via WindowFromPoint
- `mouse_proc` now resolves target PID at hook time: `WindowFromPoint(info.pt)` → `GetAncestor(hwnd, GA_ROOT)` → `GetWindowThreadProcessId(hwnd_root, &mut pid)`
- All three APIs (`WindowFromPoint`, `GetAncestor`, `GetWindowThreadProcessId`) imported from `Win32::UI::WindowsAndMessaging` — no new Cargo features needed
- `GA_ROOT` constant also from `WindowsAndMessaging` (value = 2)
- If `WindowFromPoint` returns null (no window under cursor), `GetAncestor` and `GetWindowThreadProcessId` gracefully handle null HWND — pid stays 0 (global fallback)
- `MSLLHOOKSTRUCT.pt` is a `POINT` struct passed directly to `WindowFromPoint` — no conversion needed
- These Win32 calls are fast enough for LL hook callback (no process handle opening, no string queries)
- 45 tests still pass — hook code is `#[cfg(target_os = "windows")]` so Linux test runner compiles the non-Windows stub

### [2026-03-25] Task 6: Engine flush wiring to AppThresholdCache
- `ScrollEngine` now owns `threshold_cache: Arc<Mutex<AppThresholdCache>>` and `pid_to_key: HashMap<u32, AppKey>` for future PID→AppKey resolution.
- `flush_pending()` now routes threshold lookup through `threshold_for_current_pid()`: pid=0, missing pid mapping, or lock failure all fall back to `config.output.inject_threshold`.
- Cache path is active when `pid_to_key` has an entry: `cache.get_threshold(app_key)` drives per-app behavior (`Legacy120`→120, `SmoothOk`→1).
- Added `set_threshold_cache(...)` to support external cache wiring without platform-specific code in `engine.rs`.
- Added tests: `flush_uses_per_app_threshold_legacy`, `flush_uses_per_app_threshold_smooth`, `flush_falls_back_to_global`.
- Validation: `cargo test engine -- --nocapture` and full `cargo test` both pass (48 tests).

### [2026-03-25] Task 7: PID Resolution & Process Resolver
- `ProcessResolver` trait in `resolve.rs` follows same minimal DI pattern as `TimeSource`/`ScrollOutput` in `traits.rs`.
- `MockProcessResolver` is `#[cfg(test)]` only — returns a configurable `Option<AppKey>`.
- `WindowsProcessResolver` in `resolve_win.rs` uses `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` — both from `Win32_System_Threading` (already in Cargo.toml features).
- Non-Windows stub returns `None` — same pattern as `injector.rs`.
- Engine constructor now takes `Arc<dyn ProcessResolver>` — all call sites (main.rs + test helpers) updated.
- PID resolution happens in `handle_command` Scroll arm, after setting `current_target_pid` but before `handle_scroll()`.
- Resolution is skipped for `pid == 0` and already-resolved PIDs (`pid_to_key.contains_key`).
- User overrides from `config.output.app_overrides` take precedence: value >= 100.0 → `Legacy120`, else → `SmoothOk`, set directly in cache.
- `OpenProcess` failure → `None` → no entry in `pid_to_key` → engine uses global `inject_threshold` fallback.
- `PROCESS_NAME_WIN32` flag (value 0) used with `QueryFullProcessImageNameW` for Win32 path format.
- `CloseHandle` is called unconditionally after `OpenProcess` succeeds, regardless of query result.
- 5 new tests: `pid_resolution_populates_key_map`, `failed_resolution_uses_global_default`, `pid_resolution_applies_user_override_legacy`, `pid_resolution_applies_user_override_smooth`, `pid_resolution_skips_already_resolved`.
- Total: 53 tests passing (was 48).

### [2026-03-25] Task 8: Scroll Detector (WPF heuristic + GetScrollInfo probe)
- 新增 `ScrollDetector` trait（`src/detector.rs`）與 `WindowsScrollDetector`（`src/detector_win.rs`），維持 `traits.rs` 的極簡 DI 風格。
- WPF 快速判斷採 `GetClassNameW` 前綴比對：class 以 `HwndWrapper` 開頭即直接視為 `Legacy120`，避免多餘 Win32 捲軸探測。
- Win32 探測流程：`GetWindowLongW(GWL_STYLE)` 檢查 `WS_VSCROLL`；無捲軸且非 WPF 直接 `SmoothOk`。
- 有 `WS_VSCROLL` 才進入 `GetScrollInfo(SIF_POS)` 探測：取 `before_pos`，最多 250ms、每 20ms 輪詢直到位置變化或超時。
- 判斷規則：位置無變化（邊界/無有效捲動）回傳 `Unknown`；若 `actual_delta > 5 * expected_delta` 則 `Legacy120`，否則 `SmoothOk`。
- 失敗保守策略：`hwnd=0`、`GetClassNameW` 失敗、`GetScrollInfo` 失敗等路徑一律回 `Unknown`。
- 非 Windows stub 維持安全預設：`WindowsScrollDetector::detect()` 回傳 `SmoothOk`。
- `src/detector.rs` 加入 3 個測試（mock class/style/scroll 輸入）涵蓋：WPF 偵測、現代 App 無捲軸、失敗路徑 `Unknown`。
- 驗證結果：`cargo test detector`（輸出已存 `.sisyphus/evidence/task-8-wpf-heuristic.txt`）與全量 `cargo test` 均通過，總測試數提升為 56。

### [2026-03-25] Task 9: Engine 非阻塞偵測串接
- `EngineCommand` 新增 `DetectResult { app_key, mode }`；另新增 `DetectRequest` struct（非 enum variant）給 engine→detector 專用 channel。
- `ScrollEngine::new(...)` 新增 `detector: Box<dyn ScrollDetector>` 與 `tx: Sender<EngineCommand>` 參數，建構時建立 `detect_tx/detect_rx` 並啟動獨立 detector thread。
- Detector thread 以 `recv()` 阻塞等待請求，呼叫 `detector.detect(hwnd, expected_delta)` 後透過既有 engine channel 回送 `EngineCommand::DetectResult`，不阻塞 150Hz 動畫迴圈。
- `handle_command(Scroll)` 在 PID resolve 後，若已有 `app_key` 則呼叫 `cache.start_detecting(app_key.clone())`；僅回傳 `true` 時才送出 `DetectRequest`（`hwnd=0` placeholder）。
- `handle_command(DetectResult)` 會在 `drain_commands` 流程中落地 `cache.set_mode(app_key, mode)`，完成 cache 狀態轉移。
- 去重保證依賴 `start_detecting()`：`Detecting/已解析 mode` 皆不會重複送 detect request。
- `main.rs` 已注入 `WindowsScrollDetector`，並把 `engine_tx.clone()` 傳入 engine 建構函式供 detector thread 回傳結果。
- 新增 2 個 engine 測試：`detection_triggers_for_unknown_app`、`no_duplicate_detection_for_detecting_app`。
- 驗證結果：`cargo test engine` 輸出已存 `.sisyphus/evidence/task-9-detection-trigger.txt`；全量 `cargo test` 通過，總測試數 58。
