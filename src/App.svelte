<script lang="ts">
  import type { Config } from "./lib/types";
  import ScrollSettings from "./lib/ScrollSettings.svelte";
  import AccelerationSettings from "./lib/AccelerationSettings.svelte";
  import OutputSettings from "./lib/OutputSettings.svelte";
  import KeyboardSettings from "./lib/KeyboardSettings.svelte";
  import GeneralSettings from "./lib/GeneralSettings.svelte";

  // Config will be loaded from Tauri in T14 — use defaults for now
  let config: Config = {
    scroll: {
      frame_rate: 150,
      animation_time: 400,
      step_size: 100,
      pulse_algorithm: true,
      pulse_scale: 4.0,
      pulse_normalize: 1.0,
      inverted: false,
    },
    acceleration: {
      delta_ms: 50,
      max: 3.0,
    },
    output: {
      inject_threshold: "auto",
      app_overrides: {},
    },
    general: {
      autostart: false,
      enabled: true,
    },
    keyboard: {
      enabled: true,
      mode: "always",
      page_up_down: { mode: undefined },
      arrow_keys: { mode: "off" },
      space: { mode: "off" },
    },
  };

  let saveStatus = "";
  let isSaving = false;

  async function handleSave() {
    // IPC will be wired in T14
    saveStatus = "已儲存（IPC 待實作）";
    setTimeout(() => (saveStatus = ""), 2000);
  }
</script>

<main>
  <header>
    <h1>butter-scroll 設定</h1>
  </header>

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

  <footer>
    <div class="save-row">
      {#if saveStatus}
        <span class="save-status">{saveStatus}</span>
      {/if}
      <button
        type="button"
        class="save-btn"
        disabled={isSaving}
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
