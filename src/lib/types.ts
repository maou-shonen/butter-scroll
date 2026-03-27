// Mirror of Rust Config structs (snake_case preserved — Tauri IPC uses serde snake_case)

export interface ScrollConfig {
  frame_rate: number; // 30–1000 Hz
  animation_time: number; // 1–5000 ms
  step_size: number; // 1–2000
  pulse_scale: number; // 0.1–20
  pulse_normalize: number; // 0.1–10
  inverted: boolean;
}

export interface AccelerationConfig {
  delta_ms: number; // 1–500 ms
  max: number; // 1–20
}

// ThresholdSetting: either "auto" string or a number (1–120)
export type ThresholdSetting = "auto" | number;

export interface OutputConfig {
  inject_threshold: ThresholdSetting;
  app_overrides: Record<string, number>; // exe path → threshold (1–120)
}

export type KeyboardMode = "off" | "always" | "win32_scrollbar";

export interface KeyboardGroupConfig {
  mode?: KeyboardMode; // undefined = inherit from parent
}

export interface KeyboardConfig {
  enabled: boolean;
  mode: KeyboardMode;
  page_up_down: KeyboardGroupConfig;
  arrow_keys: KeyboardGroupConfig;
  space: KeyboardGroupConfig;
}

export interface GeneralConfig {
  autostart: boolean;
  enabled: boolean;
}

export type AppFilterMode = "blacklist" | "whitelist";

export interface AppFilterConfig {
  mode: AppFilterMode;
  list: string[]; // executable paths
}

export interface Config {
  scroll: ScrollConfig;
  acceleration: AccelerationConfig;
  output: OutputConfig;
  general: GeneralConfig;
  keyboard: KeyboardConfig;
  app_filter: AppFilterConfig | null;
}

export interface AppStatus {
  enabled: boolean;
  keyboard_enabled: boolean;
  autostart_enabled: boolean;
}
