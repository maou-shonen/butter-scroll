<script lang="ts">
  import type { AccelerationConfig } from "./types";

  export let config: AccelerationConfig;
</script>

<section>
  <h2>加速設定</h2>

  <div class="field">
    <label>
      <span>加速時間窗口</span>
      <span class="value">{config.delta_ms} ms</span>
    </label>
    <input type="range" min="1" max="500" step="1" bind:value={config.delta_ms} />
    <p class="hint">在此時間內連續滾動將觸發加速</p>
  </div>

  <div class="field">
    <label>
      <span>最大加速倍數</span>
      <span class="value">{config.max.toFixed(1)}×</span>
    </label>
    <input type="range" min="1" max="20" step="0.1" bind:value={config.max} />
    <p class="hint">快速滾動時的最大速度倍增（1.0 = 無加速）</p>
  </div>
</section>

<style>
  section {
    padding: 0.75rem 0;
  }
  h2 {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
    color: var(--text-primary);
  }
  .field {
    margin-bottom: 1rem;
  }
  label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }
  .value {
    font-size: 0.85rem;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    width: 100%;
    margin: 0.25rem 0;
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin-top: 0.1rem;
  }
</style>
