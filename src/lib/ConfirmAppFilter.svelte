<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { toggleAppFilterEntry } from "./api";

  // Parse URL query params
  const params = new URLSearchParams(window.location.search);
  const exePath = params.get("exe_path") || "";
  const appName = params.get("app_name") || "Unknown App";
  const inList = params.get("in_list") === "true";
  const mode = params.get("mode") || "blacklist";

  let isProcessing = false;
  let error = "";

  const modeLabel = mode === "blacklist" ? "黑名單" : "白名單";
  const actionText = inList ? "從名單移除" : "加入名單";
  const statusText = inList ? `目前在 ${modeLabel} 中` : `不在 ${modeLabel} 中`;

  async function handleConfirm() {
    if (isProcessing) return;
    isProcessing = true;
    error = "";

    try {
      await toggleAppFilterEntry(exePath);
      await getCurrentWindow().close();
    } catch (e) {
      error = `操作失敗: ${e}`;
      isProcessing = false;
    }
  }

  async function handleCancel() {
    await getCurrentWindow().close();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      handleConfirm();
    } else if (event.key === "Escape") {
      handleCancel();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<svelte:head>
  <title>確認 - butter-scroll</title>
</svelte:head>

<main>
  <div class="content">
    <h1>{appName}</h1>
    <p class="path" title={exePath}>{exePath}</p>
    <p class="status">{statusText}</p>

    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <div class="actions">
    <button
      type="button"
      class="btn btn-secondary"
      on:click={handleCancel}
      disabled={isProcessing}
    >
      取消
    </button>
    <button
      type="button"
      class="btn btn-primary"
      on:click={handleConfirm}
      disabled={isProcessing}
    >
      {isProcessing ? "處理中..." : actionText}
    </button>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    padding: 1rem;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 14px;
    background: var(--bg-primary);
    color: var(--text-secondary);
  }

  .content {
    flex: 1;
  }

  h1 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0 0 0.25rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin: 0 0 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    font-size: 0.85rem;
    color: var(--text-tertiary);
    margin: 0;
  }

  .error {
    font-size: 0.8rem;
    color: var(--error-color);
    margin: 0.75rem 0 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color-light);
    margin-top: 1rem;
  }

  .btn {
    padding: 0.4rem 1rem;
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    border: none;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--btn-primary-hover);
  }

  .btn-secondary {
    background: var(--btn-secondary-bg);
    color: var(--btn-secondary-text);
    border: 1px solid var(--btn-secondary-border);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--btn-secondary-hover-bg);
    border-color: var(--btn-secondary-hover-border);
  }
</style>
