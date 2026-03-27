<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { AppFilterMode } from "./types";

  const dispatch = createEventDispatcher<{ confirm: { mode: AppFilterMode } }>();

  let selectedMode: AppFilterMode | null = null;

  function selectMode(mode: AppFilterMode) {
    selectedMode = mode;
  }

  function handleConfirm() {
    if (selectedMode) {
      dispatch("confirm", { mode: selectedMode });
    }
  }
</script>

<div class="setup-container">
  <div class="setup-content">
    <h1>選擇過濾模式</h1>
    <p class="subtitle">決定哪些應用程式會套用平滑捲動</p>

    <div class="options">
      <button
        type="button"
        class="option"
        class:selected={selectedMode === "blacklist"}
        on:click={() => selectMode("blacklist")}
      >
        <div class="option-header">
          <span class="option-title">黑名單</span>
        </div>
        <p class="option-desc">平滑所有應用程式的捲動，排除名單中的應用程式</p>
      </button>

      <button
        type="button"
        class="option"
        class:selected={selectedMode === "whitelist"}
        on:click={() => selectMode("whitelist")}
      >
        <div class="option-header">
          <span class="option-title">白名單</span>
        </div>
        <p class="option-desc">僅平滑名單中的應用程式</p>
      </button>
    </div>

    <button
      type="button"
      class="confirm-btn"
      disabled={!selectedMode}
      on:click={handleConfirm}
    >
      確認
    </button>
  </div>
</div>

<style>
  .setup-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem 1rem;
    background: var(--bg-secondary);
  }

  .setup-content {
    width: 100%;
    max-width: 420px;
  }

  h1 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem;
    color: var(--text-primary);
    text-align: center;
  }

  .subtitle {
    font-size: 0.85rem;
    color: var(--text-hint);
    margin: 0 0 1.5rem;
    text-align: center;
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .option {
    display: block;
    padding: 1rem;
    background: var(--bg-primary);
    border: 2px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .option:hover {
    border-color: var(--text-hint);
  }

  .option.selected {
    border-color: var(--accent-color);
    background: var(--bg-tertiary);
  }

  .option-header {
    margin-bottom: 0.35rem;
  }

  .option-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .option-desc {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
  }

  .confirm-btn {
    display: block;
    width: 100%;
    padding: 0.6rem 1.2rem;
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: background-color 0.15s ease;
  }

  .confirm-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .confirm-btn:hover:not(:disabled) {
    background: var(--btn-primary-hover);
  }
</style>
