import { invoke } from "@tauri-apps/api/core";
import type { Config, AppStatus, ToggleResult } from "./types";

/** Returns the current configuration from the Rust backend. */
export async function getConfig(): Promise<Config> {
  return await invoke<Config>("get_config");
}

/** Returns the default configuration (for resetting). */
export async function getDefaultConfig(): Promise<Config> {
  return await invoke<Config>("get_default_config");
}

/** Saves the configuration and hot-reloads the engine. */
export async function saveConfig(config: Config): Promise<void> {
  return await invoke<void>("save_config", { config });
}

/** Toggles scroll smoothing. Returns new enabled state. */
export async function toggleEnabled(): Promise<boolean> {
  return await invoke<boolean>("toggle_enabled");
}

/** Toggles an app filter entry. Returns the resulting action and list state. */
export async function toggleAppFilterEntry(exePath: string): Promise<ToggleResult> {
  return await invoke<ToggleResult>("toggle_app_filter_entry", { exePath });
}

/** Toggles keyboard smoothing. Returns new enabled state. */
export async function toggleKeyboard(): Promise<boolean> {
  return await invoke<boolean>("toggle_keyboard");
}

/** Toggles autostart. Returns new enabled state. */
export async function toggleAutostart(): Promise<boolean> {
  return await invoke<boolean>("toggle_autostart");
}

/** Returns current app status. */
export async function getStatus(): Promise<AppStatus> {
  return await invoke<AppStatus>("get_status");
}

/** Checks for available updates. Returns true if update available. */
export async function checkForUpdates(): Promise<boolean> {
  return await invoke<boolean>("check_for_updates");
}
