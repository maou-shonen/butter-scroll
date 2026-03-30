<script lang="ts">
  import type { HotkeyConfig } from "./types";

  export let config: HotkeyConfig;

  const comboOptions: { value: string; label: string }[] = [
    { value: "ctrl+shift+b", label: "Ctrl + Shift + B" },
    { value: "ctrl+alt+b", label: "Ctrl + Alt + B" },
    { value: "ctrl+shift+alt+b", label: "Ctrl + Shift + Alt + B" },
    { value: "ctrl+shift+h", label: "Ctrl + Shift + H" },
  ];
</script>

<section>
  <h2>快速鍵設定</h2>

  <div class="field toggle-field">
    <label>
      <span>啟用全域快速鍵</span>
      <input type="checkbox" bind:checked={config.enabled} />
    </label>
    <p class="hint">按下快速鍵可切換當前應用程式的平滑捲動</p>
  </div>

  <div class="field">
    <label for="hotkey-combo">快速鍵組合</label>
    <select id="hotkey-combo" bind:value={config.combo} disabled={!config.enabled}>
      {#each comboOptions as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
    <p class="hint">選擇切換平滑捲動的快速鍵組合</p>
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
    color: var(--text-secondary);
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
    border: 1px solid var(--input-border);
    border-radius: 4px;
    background: var(--input-bg);
    color: var(--text-secondary);
    cursor: pointer;
  }

  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  select:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin: 0.2rem 0 0;
  }
</style>
