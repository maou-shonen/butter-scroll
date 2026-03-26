[English](README.md)

# butter-scroll

> [!NOTE]
> 本專案完全由 coding agent（AI）編寫，維護者負責審核邏輯與方向，不直接撰寫 Rust 程式碼。

如奶油般滑順的 Windows 滑鼠滾輪體驗。

butter-scroll 是一個輕量的系統匣工具，在作業系統層級攔截滑鼠滾輪與鍵盤捲動事件，套用緩動動畫後重新注入平滑化的事件 — 讓每個應用程式的捲動都像奶油一樣滑順。

因為找不到免費且堪用的替代方案，所以自己做了一個。

## 功能

- **平滑捲動** — 可選緩動曲線（Pulse、OutCubic、OutQuint、OutExpo、OutCirc、OutBack）取代預設的頓挫滾輪行為
- **鍵盤捲動** — 可選的 Page Up/Down、方向鍵、Space 平滑捲動
- **加速度** — 快速連續滾動時自動加速
- **逐應用自適應輸出** — 自動偵測每個應用程式最佳的注入閾值
- **系統匣** — 安靜地在背景執行，透過匣圖示開啟設定介面
- **自動更新** — 內建更新檢查，透過 GitHub Releases 發佈
- **TOML 設定檔** — 所有設定集中在一個人類可讀的設定檔中

## 安裝

從 [Releases](https://github.com/maou-shonen/butter-scroll/releases) 下載最新的安裝程式或可攜版 ZIP。

| 套件 | 說明 |
|---|---|
| `*-setup.exe` | NSIS 安裝程式 — 安裝至使用者目錄，含自動更新 |
| `*-portable.zip` | 可攜版 — 解壓縮至任意位置，免安裝 |

> [!NOTE]
> 可攜版需要 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)（Windows 10 22H2+ 及 Windows 11 已內建）。可攜版不支援自動更新。

## 快速入門

1. 執行安裝程式（或解壓縮可攜版 ZIP）— butter-scroll 會啟動在系統匣
2. 右鍵點擊匣圖示 → **設定** 開啟設定面板
3. 調整捲動手感（緩動曲線、動畫時間、步進大小）後點擊 **儲存設定**
4. 若要開機自動啟動，在「一般」設定中啟用 **開機自動啟動**

設定檔位於 `%APPDATA%\com.butter-scroll.app\config.toml`。  
可攜版的設定檔位於執行檔旁。  
刪除該檔案即可在下次啟動時還原為預設值。

## 開發

前置需求：[mise](https://mise.jdx.dev/)（管理 Rust 與 Node.js 版本）、[pnpm](https://pnpm.io/)

```sh
pnpm install
mise run dev       # 啟動開發伺服器（Tauri + Vite HMR）
mise run test      # 執行 Rust 測試
mise run check     # Cargo check
mise run clippy    # Lint 檢查
mise run build     # 建置 NSIS 安裝程式 + release exe
```

詳細設定選項請參閱 [docs/configuration.md](docs/configuration.md)。

## 致謝

預設捲動動畫算法（Pulse）基於 [@gblazex](https://github.com/gblazex) 的 [SmoothScroll](https://github.com/gblazex/smoothscroll) — 一個為瀏覽器帶來平滑捲動的 Chrome 擴充功能。butter-scroll 將其脈衝緩動曲線（Michael Herf 算法）移植到 Windows 系統層級，並提供多種標準 Penner 緩動函數作為替代選項。

## 授權

MIT
