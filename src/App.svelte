<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Config } from "./lib/types";
  import { getConfig, getDefaultConfig, saveConfig } from "./lib/api";
  import ScrollSettings from "./lib/ScrollSettings.svelte";
  import AccelerationSettings from "./lib/AccelerationSettings.svelte";
  import OutputSettings from "./lib/OutputSettings.svelte";
  import KeyboardSettings from "./lib/KeyboardSettings.svelte";
  import GeneralSettings from "./lib/GeneralSettings.svelte";

  // 標籤定義
  const tabs = [
    { id: "scroll", label: "捲動" },
    { id: "acceleration", label: "加速" },
    { id: "output", label: "輸出" },
    { id: "keyboard", label: "鍵盤" },
    { id: "general", label: "一般" },
  ] as const;

  // State
  let config: Config | null = null;
  let activeTab = "scroll";
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

  // 重設當前標籤的設定為預設值
  async function resetCurrentTab() {
    if (!config) return;
    try {
      const defaults = await getDefaultConfig();
      switch (activeTab) {
        case "scroll":
          config.scroll = defaults.scroll;
          break;
        case "acceleration":
          config.acceleration = defaults.acceleration;
          break;
        case "output":
          config.output = defaults.output;
          break;
        case "keyboard":
          config.keyboard = defaults.keyboard;
          break;
        case "general":
          config.general = defaults.general;
          break;
      }
      // 觸發響應式更新
      config = config;
    } catch (e) {
      console.error("重設失敗:", e);
    }
  }
</script>

<main>
  <header>
    <h1>butter-scroll 設定</h1>
    <nav class="tab-bar">
      {#each tabs as tab}
        <button
          type="button"
          class="tab"
          class:active={activeTab === tab.id}
          on:click={() => (activeTab = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </nav>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {:else if config === null}
    <div class="loading">載入中...</div>
  {:else}
    <div class="content">
      <div class="tab-content">
        {#if activeTab === "scroll"}
          <ScrollSettings bind:config={config.scroll} />
        {:else if activeTab === "acceleration"}
          <AccelerationSettings bind:config={config.acceleration} />
        {:else if activeTab === "output"}
          <OutputSettings bind:config={config.output} />
        {:else if activeTab === "keyboard"}
          <KeyboardSettings bind:config={config.keyboard} />
        {:else if activeTab === "general"}
          <GeneralSettings bind:config={config.general} />
        {/if}
      </div>
      <div class="reset-row">
        <button
          type="button"
          class="reset-btn"
          on:click={resetCurrentTab}
        >
          重設為預設值
        </button>
      </div>
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
    background: var(--bg-secondary);
  }

  header {
    background: var(--bg-primary);
  }

  h1 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    padding: 0.75rem 1rem 0.5rem;
    color: var(--text-primary);
  }

  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border-color);
    padding: 0 0.5rem;
  }

  .tab {
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--tab-text);
    background: var(--tab-bg);
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 0.15s ease, background-color 0.15s ease, border-color 0.15s ease;
  }

  .tab:hover {
    background: var(--tab-hover-bg);
  }

  .tab.active {
    color: var(--tab-active-text);
    border-bottom-color: var(--tab-active-border);
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 0 1rem;
    background: var(--bg-primary);
  }

  .reset-row {
    padding: 0.5rem 1rem;
    background: var(--bg-primary);
    border-top: 1px solid var(--border-color-light);
    display: flex;
    justify-content: flex-end;
  }

  .reset-btn {
    padding: 0.3rem 0.75rem;
    font-size: 0.8rem;
    color: var(--btn-secondary-text);
    background: var(--btn-secondary-bg);
    border: 1px solid var(--btn-secondary-border);
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .reset-btn:hover {
    background: var(--btn-secondary-hover-bg);
    border-color: var(--btn-secondary-hover-border);
  }

  .loading,
  .error {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1.25rem;
    color: var(--text-hint);
    text-align: center;
  }

  .error {
    color: var(--error-color);
  }

  footer {
    padding: 0.75rem 1rem;
    border-top: 1px solid var(--border-color);
    background: var(--bg-secondary);
  }

  .save-row {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .save-status {
    font-size: 0.8rem;
    color: var(--success-color);
  }

  .save-error {
    color: var(--error-color);
  }

  .save-btn {
    padding: 0.4rem 1.2rem;
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: background-color 0.15s ease;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--btn-primary-hover);
  }
</style>
