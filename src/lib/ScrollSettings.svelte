<script lang="ts">
  import type { ScrollConfig } from "./types";

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
    <p class="hint">
      動畫每秒更新的次數。越高越平滑，但 CPU 使用量越多。
      150 Hz 是平滑與效能的良好平衡點；大多數情況下不需要調整
    </p>
  </div>

  <div class="field">
    <label>
      <span>動畫持續時間</span>
      <span class="value">{config.animation_time} ms</span>
    </label>
    <input type="range" min="1" max="5000" step="10" bind:value={config.animation_time} />
    <p class="hint">
      每次滾輪刻度觸發的動畫長度。
      較短（如 200ms）= 反應靈敏，適合快速瀏覽；
      較長（如 600ms）= 滑順漸進，適合閱讀長文
    </p>
  </div>

  <div class="field">
    <label>
      <span>捲動距離</span>
      <span class="value">{config.step_size}</span>
    </label>
    <input type="range" min="1" max="2000" step="1" bind:value={config.step_size} />
    <p class="hint">
      每個滾輪刻度的捲動距離。100 約等於 Windows 預設的 3 行。
      想一次看更多內容可以調高，想精確瀏覽可以調低
    </p>
  </div>

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
    <p class="hint">
      控制「快啟慢停」的強度。數值越高，動畫越集中在前半段完成，
      尾端減速越明顯。4.0 是預設值；2.0 偏平穩，8.0 偏激進
    </p>
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
    <p class="hint">
      控制曲線的輸出幅度。1.0 = 自動標準化（確保動畫完整跑完）。
      通常不需要調整，除非你想微調 Pulse 強度的效果
    </p>
  </div>

  <div class="field toggle-field">
    <label>
      <span>反向捲動方向</span>
      <input type="checkbox" bind:checked={config.inverted} />
    </label>
    <p class="hint">macOS 風格「自然滾動」— 滾輪向下時內容向上移動</p>
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
  .toggle-field label {
    cursor: pointer;
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-hint);
    margin-top: 0.1rem;
  }
</style>
