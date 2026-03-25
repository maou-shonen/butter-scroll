# Decisions

## [2026-03-25] Session Start

### Wave 1 (Independent — all 4 run in parallel)
- T1: `quick` — Add app_overrides to OutputConfig
- T2: `unspecified-high` — Create AppThresholdCache module (pure Rust)
- T3: `quick` — Add windows-sys features to Cargo.toml
- T4: `quick` — Extend EngineCommand::Scroll with target_pid

### Guardrails
- Engine.rs must stay cross-platform (NO `#[cfg(windows)]` in engine.rs)
- Hook callback only: WindowFromPoint + GetWindowThreadProcessId (fast, in-process)
- All cross-process calls (OpenProcess, GetScrollInfo) → detector thread only
- Tasks 1-10 keep default threshold=40; Task 11 flips it to 1 (last step)
