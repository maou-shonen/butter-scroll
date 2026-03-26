// Mirror of Rust Config structs (snake_case preserved — Tauri IPC uses serde snake_case)

export type EasingType =
  | "linear"
  | "pulse"
  | "out_cubic"
  | "out_quint"
  | "out_expo"
  | "out_circ"
  | "out_back";

export const EASING_OPTIONS: { value: EasingType; label: string }[] = [
  { value: "pulse", label: "Pulse (預設)" },
  { value: "out_cubic", label: "OutCubic" },
  { value: "out_quint", label: "OutQuint" },
  { value: "out_expo", label: "OutExpo" },
  { value: "out_circ", label: "OutCirc" },
  { value: "out_back", label: "OutBack (有回彈)" },
  { value: "linear", label: "Linear (無緩動)" },
];

export interface ScrollConfig {
  frame_rate: number; // 30–1000 Hz
  animation_time: number; // 1–5000 ms
  step_size: number; // 1–2000
  easing: EasingType;
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

export interface Config {
  scroll: ScrollConfig;
  acceleration: AccelerationConfig;
  output: OutputConfig;
  general: GeneralConfig;
  keyboard: KeyboardConfig;
}

export interface AppStatus {
  enabled: boolean;
  keyboard_enabled: boolean;
  autostart_enabled: boolean;
}
