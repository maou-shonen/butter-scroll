<script lang="ts">
  import type { KeyboardConfig, KeyboardMode, KeyboardGroupConfig } from "./types";

  export let config: KeyboardConfig;

  const modeOptions: { value: KeyboardMode | undefined; label: string }[] = [
    { value: undefined, label: "繼承預設" },
    { value: "off", label: "停用" },
    { value: "always", label: "永遠攔截" },
    { value: "win32_scrollbar", label: "僅 Win32 捲動條" },
  ];

  // For the parent mode (no undefined option)
  const parentModeOptions: { value: KeyboardMode; label: string }[] = [
    { value: "off", label: "停用" },
    { value: "always", label: "永遠攔截" },
    { value: "win32_scrollbar", label: "僅 Win32 捲動條" },
  ];

  function handleGroupModeChange(group: KeyboardGroupConfig, value: string) {
    if (value === "" || value === "inherit") {
      group.mode = undefined;
    } else {
      group.mode = value as KeyboardMode;
    }
    // Trigger Svelte reactivity — in-place mutation of nested object
    // doesn't cause reassignment, so isDirty in App.svelte won't detect the change.
    config = config;
  }

  function getGroupModeValue(group: KeyboardGroupConfig): string {
    return group.mode ?? "";
  }
</script>

<section>
  <h2>鍵盤平滑捲動</h2>

  <div class="field toggle-field">
    <label>
      <span>啟用鍵盤平滑捲動</span>
      <input type="checkbox" bind:checked={config.enabled} />
    </label>
  </div>

  {#if config.enabled}
    <div class="field">
      <label><span>預設模式</span></label>
      <select bind:value={config.mode}>
        {#each parentModeOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <p class="hint">作為各按鍵群組的預設值</p>
    </div>

    <h3>按鍵群組設定</h3>

    <div class="field">
      <label><span>PageUp / PageDown</span></label>
      <select
        value={getGroupModeValue(config.page_up_down)}
        on:change={(e) => handleGroupModeChange(config.page_up_down, (e.target as HTMLSelectElement).value)}
      >
        {#each modeOptions as opt}
          <option value={opt.value ?? ""}>{opt.label}</option>
        {/each}
      </select>
      <p class="hint">低風險：鮮少與其他功能衝突</p>
    </div>

    <div class="field">
      <label><span>方向鍵（↑↓）</span></label>
      <select
        value={getGroupModeValue(config.arrow_keys)}
        on:change={(e) => handleGroupModeChange(config.arrow_keys, (e.target as HTMLSelectElement).value)}
      >
        {#each modeOptions as opt}
          <option value={opt.value ?? ""}>{opt.label}</option>
        {/each}
      </select>
      <p class="hint">高衝突風險：可能干擾文字編輯</p>
    </div>

    <div class="field">
      <label><span>空白鍵 / Shift+空白鍵</span></label>
      <select
        value={getGroupModeValue(config.space)}
        on:change={(e) => handleGroupModeChange(config.space, (e.target as HTMLSelectElement).value)}
      >
        {#each modeOptions as opt}
          <option value={opt.value ?? ""}>{opt.label}</option>
        {/each}
      </select>
      <p class="hint">中等風險：瀏覽器適用，文字輸入不適用</p>
    </div>
  {/if}
</section>

<style>
  section {
    padding: 0.5rem 0;
  }

  h2 {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
    color: #1a1a1a;
  }

  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem;
    color: #333;
  }

  .field {
    margin-bottom: 0.6rem;
  }

  .field > label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.3rem;
  }

  .field > label > span {
    font-weight: 500;
    color: #333;
  }

  .toggle-field label {
    cursor: pointer;
  }

  .toggle-field input[type="checkbox"] {
    margin: 0;
  }

  select {
    width: 100%;
    padding: 0.4rem 0.5rem;
    font-size: 0.85rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    background: white;
    cursor: pointer;
  }

  select:focus {
    outline: none;
    border-color: #999;
  }

  .hint {
    font-size: 0.75rem;
    color: #666;
    margin: 0.2rem 0 0;
  }
</style>
