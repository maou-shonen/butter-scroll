<script lang="ts">
  import type { GeneralConfig } from "./types";

  export let config: GeneralConfig;
</script>

<section>
  <h2>一般設定</h2>

  <div class="field toggle-field">
    <label>
      <span>啟用平滑捲動</span>
      <input type="checkbox" bind:checked={config.enabled} />
    </label>
    <p class="hint">關閉後，滾輪事件直接傳遞不作修改</p>
  </div>

  <div class="field toggle-field">
    <label>
      <span>開機自動啟動</span>
      <input type="checkbox" bind:checked={config.autostart} />
    </label>
    <p class="hint">登入 Windows 時自動啟動 butter-scroll</p>
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

  .toggle-field label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .toggle-field label > span {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .toggle-field input[type="checkbox"] {
    margin: 0;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin: 0.2rem 0 0 0;
    padding-left: 0;
  }
</style>
