<script lang="ts">
  import type { AppFilterConfig, AppFilterMode } from "./types";

  export let config: AppFilterConfig;

  let newExePath = "";

  function setMode(mode: AppFilterMode) {
    config.mode = mode;
  }

  function addPath() {
    const path = newExePath.trim();
    if (!path) return;
    if (!config.list.includes(path)) {
      config.list = [...config.list, path];
    }
    newExePath = "";
  }

  function removePath(path: string) {
    config.list = config.list.filter((p) => p !== path);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      addPath();
    }
  }
</script>

<section>
  <h2>應用程式過濾</h2>

  <div class="field">
    <label>
      <span>過濾模式</span>
    </label>
    <div class="mode-options">
      <label class="radio">
        <input
          type="radio"
          checked={config.mode === "blacklist"}
          on:change={() => setMode("blacklist")}
        />
        <span>黑名單</span>
      </label>
      <label class="radio">
        <input
          type="radio"
          checked={config.mode === "whitelist"}
          on:change={() => setMode("whitelist")}
        />
        <span>白名單</span>
      </label>
    </div>
    <p class="hint">
      {#if config.mode === "blacklist"}
        平滑所有應用程式的捲動，排除名單中的應用程式
      {:else}
        僅平滑名單中的應用程式
      {/if}
    </p>
  </div>

  <div class="field">
    <h3>應用程式清單</h3>
    <p class="hint">輸入應用程式的執行檔路徑</p>

    {#each config.list as path (path)}
      <div class="list-item">
        <span class="exe-path" title={path}>{path}</span>
        <button type="button" on:click={() => removePath(path)}>移除</button>
      </div>
    {/each}

    {#if config.list.length === 0}
      <p class="empty-hint">尚未加入任何應用程式</p>
    {/if}

    <div class="add-path">
      <input
        type="text"
        placeholder="C:\Program Files\App\app.exe"
        bind:value={newExePath}
        on:keydown={handleKeydown}
      />
      <button type="button" on:click={addPath}>新增</button>
    </div>
  </div>
</section>

<style>
  section {
    padding: 0.5rem 0;
  }

  h2 {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
    color: var(--text-primary);
  }

  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem;
    color: var(--text-secondary);
  }

  .field {
    margin-bottom: 0.75rem;
  }

  .field > label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
  }

  .field > label > span:first-child {
    font-weight: 500;
    color: var(--text-secondary);
    min-width: 100px;
  }

  .mode-options {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .radio input[type="radio"] {
    margin: 0;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin: 0.25rem 0 0;
  }

  .empty-hint {
    font-size: 0.8rem;
    color: var(--text-hint);
    font-style: italic;
    margin: 0.5rem 0;
  }

  .list-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid var(--border-color-light);
  }

  .exe-path {
    flex: 1;
    font-size: 0.8rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list-item button {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    background: var(--btn-secondary-bg);
    border: 1px solid var(--btn-secondary-border);
    border-radius: 3px;
    cursor: pointer;
    color: var(--btn-secondary-text);
  }

  .list-item button:hover {
    background: var(--btn-secondary-hover-bg);
    border-color: var(--btn-secondary-hover-border);
  }

  .add-path {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }

  .add-path input[type="text"] {
    flex: 1;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    border: 1px solid var(--input-border);
    border-radius: 3px;
    background: var(--input-bg);
    color: var(--text-secondary);
  }

  .add-path input[type="text"]:focus {
    outline: none;
    border-color: var(--input-border-focus);
  }

  .add-path button {
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .add-path button:hover {
    background: var(--btn-primary-hover);
  }
</style>
