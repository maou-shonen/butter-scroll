<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Config } from "./lib/types";
  import { getConfig, saveConfig } from "./lib/api";
  import ScrollSettings from "./lib/ScrollSettings.svelte";
  import AccelerationSettings from "./lib/AccelerationSettings.svelte";
  import OutputSettings from "./lib/OutputSettings.svelte";
  import KeyboardSettings from "./lib/KeyboardSettings.svelte";
  import GeneralSettings from "./lib/GeneralSettings.svelte";

  // State
  let config: Config | null = null;
  let savedSnapshot = "";
  let saveStatus = "";
  let isSaving = false;
  let error = "";

  // Cleanup handle for window focus listener
  let unlistenFocus: (() => void) | null = null;

  // Track whether user has unsaved changes
  $: isDirty = config !== null && JSON.stringify(config) !== savedSnapshot;

  // Load config from backend
  async function loadConfig() {
    try {
      config = await getConfig();
      savedSnapshot = JSON.stringify(config);
      error = "";
    } catch (e) {
      error = `無法載入設定: ${e}`;
    }
  }

  onMount(async () => {
    await loadConfig();

    // Re-fetch config when window gains focus — only if no unsaved edits
    const appWindow = getCurrentWindow();
    unlistenFocus = await appWindow.onFocusChanged(async ({ payload: focused }) => {
      if (focused && !isDirty) {
        await loadConfig();
      }
    });
  });

  onDestroy(() => {
    if (unlistenFocus) unlistenFocus();
  });

  async function handleSave() {
    if (!config) return;
    isSaving = true;
    saveStatus = "";
    try {
      await saveConfig(config);
      savedSnapshot = JSON.stringify(config);
      saveStatus = "✓ 已儲存";
      setTimeout(() => (saveStatus = ""), 2000);
    } catch (e) {
      saveStatus = `儲存失敗: ${e}`;
    } finally {
      isSaving = false;
    }
  }
</script>

<main>
  <header>
    <h1>butter-scroll 設定</h1>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {:else if config === null}
    <div class="loading">載入中...</div>
  {:else}
    <div class="settings">
      <GeneralSettings bind:config={config.general} />
      <hr />
      <ScrollSettings bind:config={config.scroll} />
      <hr />
      <AccelerationSettings bind:config={config.acceleration} />
      <hr />
      <OutputSettings bind:config={config.output} />
      <hr />
      <KeyboardSettings bind:config={config.keyboard} />
    </div>
  {/if}

  <footer>
    <div class="save-row">
      {#if saveStatus}
        <span
          class="save-status"
          class:save-error={saveStatus.includes("失敗")}
        >
          {saveStatus}
        </span>
      {/if}
      <button
        type="button"
        class="save-btn"
        disabled={isSaving || config === null}
        on:click={handleSave}
      >
        {isSaving ? "儲存中..." : "儲存設定"}
      </button>
    </div>
  </footer>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 14px;
    background: #fafafa;
  }

  header {
    padding: 1rem 1.25rem 0.5rem;
    border-bottom: 1px solid #e5e5e5;
    background: white;
  }

  h1 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    color: #1a1a1a;
  }

  .settings {
    flex: 1;
    overflow-y: auto;
    padding: 0 1.25rem;
    background: white;
  }

  hr {
    border: none;
    border-top: 1px solid #f0f0f0;
    margin: 0.5rem 0;
  }

  .loading,
  .error {
    padding: 2rem 1.25rem;
    color: #666;
    text-align: center;
  }

  .error {
    color: #dc2626;
  }

  footer {
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #e5e5e5;
    background: #fafafa;
  }

  .save-row {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .save-status {
    font-size: 0.8rem;
    color: #22c55e;
  }

  .save-error {
    color: #dc2626;
  }

  .save-btn {
    padding: 0.4rem 1.2rem;
    background: #1a1a1a;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn:hover:not(:disabled) {
    background: #333;
  }
</style>
