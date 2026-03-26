<script lang="ts">
  import type { OutputConfig, ThresholdSetting } from "./types";

  export let config: OutputConfig;

  // Determine if using "auto" or fixed mode
  $: isAuto = config.inject_threshold === "auto";
  $: fixedValue = isAuto ? 40 : (config.inject_threshold as number);

  function setAutoMode() {
    config.inject_threshold = "auto";
  }

  function setFixedMode() {
    config.inject_threshold = 40;
  }

  function updateFixedValue(e: Event) {
    const target = e.target as HTMLInputElement;
    const v = parseInt(target.value);
    config.inject_threshold = Math.max(1, Math.min(120, v));
  }

  // App overrides management
  let newExePath = "";
  let newThreshold = 40;

  function addOverride() {
    if (newExePath.trim()) {
      config.app_overrides[newExePath.trim()] = newThreshold;
      config.app_overrides = { ...config.app_overrides }; // trigger reactivity
      newExePath = "";
      newThreshold = 40;
    }
  }

  function removeOverride(key: string) {
    const { [key]: _, ...rest } = config.app_overrides;
    config.app_overrides = rest;
  }
</script>

<section>
  <h2>輸出設定</h2>

  <div class="field">
    <label>
      <span>注入閾值模式</span>
    </label>
    <div class="threshold-mode">
      <label class="radio">
        <input type="radio" checked={isAuto} on:change={setAutoMode} />
        <span>自動偵測（推薦）</span>
      </label>
      <label class="radio">
        <input type="radio" checked={!isAuto} on:change={setFixedMode} />
        <span>固定值</span>
      </label>
    </div>
    {#if !isAuto}
      <div class="fixed-threshold">
        <label>
          <span>閾值</span>
          <span class="value">{fixedValue}</span>
        </label>
        <input
          type="range"
          min="1"
          max="120"
          step="1"
          value={fixedValue}
          on:input={updateFixedValue}
        />
        <p class="hint">120 = 最相容，1 = 最平滑（僅適用現代 App）</p>
      </div>
    {/if}
    {#if isAuto}
      <p class="hint">自動偵測每個 App 的適合閾值，WPF 等舊式 App 會使用較高閾值</p>
    {/if}
  </div>

  <div class="field">
    <h3>個別 App 設定</h3>
    <p class="hint">為特定 App 設定固定閾值（覆蓋自動偵測）</p>

    {#each Object.entries(config.app_overrides) as [path, threshold] (path)}
      <div class="override-item">
        <span class="exe-path" title={path}>{path}</span>
        <span class="threshold-badge">{threshold}</span>
        <button type="button" on:click={() => removeOverride(path)}>移除</button>
      </div>
    {/each}

    <div class="add-override">
      <input
        type="text"
        placeholder="C:\Program Files\App\app.exe"
        bind:value={newExePath}
      />
      <input type="number" min="1" max="120" bind:value={newThreshold} />
      <button type="button" on:click={addOverride}>新增</button>
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
    color: #1a1a1a;
  }

  h3 {
    font-size: 0.85rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem;
    color: #333;
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
    color: #333;
    min-width: 100px;
  }

  .threshold-mode {
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
    color: #444;
  }

  .radio input[type="radio"] {
    margin: 0;
  }

  .fixed-threshold {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: #f8f8f8;
    border-radius: 4px;
  }

  .fixed-threshold label {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.35rem;
    font-size: 0.85rem;
  }

  .fixed-threshold .value {
    font-weight: 600;
    color: #1a1a1a;
  }

  .fixed-threshold input[type="range"] {
    width: 100%;
  }

  .hint {
    font-size: 0.75rem;
    color: #666;
    margin: 0.25rem 0 0;
  }

  .override-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid #eee;
  }

  .exe-path {
    flex: 1;
    font-size: 0.8rem;
    color: #444;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .threshold-badge {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.15rem 0.4rem;
    background: #e5e5e5;
    border-radius: 3px;
    color: #333;
  }

  .override-item button {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    background: transparent;
    border: 1px solid #ccc;
    border-radius: 3px;
    cursor: pointer;
    color: #666;
  }

  .override-item button:hover {
    background: #f5f5f5;
    border-color: #999;
  }

  .add-override {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }

  .add-override input[type="text"] {
    flex: 1;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    border: 1px solid #ddd;
    border-radius: 3px;
  }

  .add-override input[type="number"] {
    width: 60px;
    padding: 0.35rem 0.4rem;
    font-size: 0.8rem;
    border: 1px solid #ddd;
    border-radius: 3px;
    text-align: center;
  }

  .add-override button {
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
    background: #1a1a1a;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .add-override button:hover {
    background: #333;
  }
</style>
