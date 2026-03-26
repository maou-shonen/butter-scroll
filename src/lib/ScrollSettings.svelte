<script lang="ts">
  import type { ScrollConfig } from "./types";
  import { EASING_OPTIONS } from "./types";

  export let config: ScrollConfig;
</script>

<section>
  <h2>捲動設定</h2>

  <div class="field">
    <label>
      <span>動畫幀率</span>
      <span class="value">{config.frame_rate} Hz</span>
    </label>
    <input type="range" min="30" max="1000" step="1" bind:value={config.frame_rate} />
    <p class="hint">較高值 = 更平滑，但 CPU 使用更多</p>
  </div>

  <div class="field">
    <label>
      <span>動畫持續時間</span>
      <span class="value">{config.animation_time} ms</span>
    </label>
    <input type="range" min="1" max="5000" step="10" bind:value={config.animation_time} />
    <p class="hint">較長 = 更漸進；較短 = 更快速</p>
  </div>

  <div class="field">
    <label>
      <span>捲動距離</span>
      <span class="value">{config.step_size}</span>
    </label>
    <input type="range" min="1" max="2000" step="1" bind:value={config.step_size} />
    <p class="hint">每個滾輪刻度的捲動距離（100 = 預設）</p>
  </div>

  <div class="field">
    <label for="easing-select">緩動曲線</label>
    <select id="easing-select" bind:value={config.easing}>
      {#each EASING_OPTIONS as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
    <p class="hint">控制捲動動畫的加速與減速曲線（預覽功能）</p>
  </div>

  {#if config.easing === "pulse"}
    <div class="field">
      <label>
        <span>Pulse 強度</span>
        <span class="value">{config.pulse_scale.toFixed(1)}</span>
      </label>
      <input
        type="range"
        min="0.1"
        max="20"
        step="0.1"
        bind:value={config.pulse_scale}
      />
    </div>

    <div class="field">
      <label>
        <span>Pulse 標準化</span>
        <span class="value">{config.pulse_normalize.toFixed(1)}</span>
      </label>
      <input
        type="range"
        min="0.1"
        max="10"
        step="0.1"
        bind:value={config.pulse_normalize}
      />
      <p class="hint">保持 1.0 以自動標準化</p>
    </div>
  {/if}

  <div class="field toggle-field">
    <label>
      <span>反向捲動方向</span>
      <input type="checkbox" bind:checked={config.inverted} />
    </label>
    <p class="hint">macOS 風格「自然滾動」</p>
  </div>
</section>

<style>
  section {
    padding: 0.75rem 0;
  }
  h2 {
    font-size: 0.85rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #666;
    margin-bottom: 0.75rem;
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
  }
  .value {
    font-size: 0.85rem;
    color: #666;
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    width: 100%;
    margin: 0.25rem 0;
  }
  select {
    width: 100%;
    padding: 0.4rem 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 0.9rem;
    background: #fff;
    margin-top: 0.25rem;
  }
  select:focus {
    outline: none;
    border-color: #4a9eff;
    box-shadow: 0 0 0 2px rgba(74, 158, 255, 0.2);
  }
  .toggle-field label {
    cursor: pointer;
  }
  .hint {
    font-size: 0.75rem;
    color: #888;
    margin-top: 0.1rem;
  }
</style>
