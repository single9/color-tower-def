# Tower Defense

一款以色塊呈現的迷宮式塔防遊戲,使用 Rust + [Bevy](https://bevyengine.org/) 開發。玩家在 25x25 的 Grid 上放置 Tower 阻擋 Enemy 的行進路線,依序通過 3 個 Level、每個 Level 15 個 Wave,守住 Base 不被攻破。

完整的遊戲詞彙定義(Grid、Tower、Wave、Boss 等所有名詞的精確定義與邊界)見 [CONTEXT.md](CONTEXT.md);重大架構決策見 [docs/adr](docs/adr)。

## 玩法概觀

- 每個 Level 有固定的 Obstacle 佈局,Enemy 會沿著即時計算的最短 Path 從 Spawn 走到 Goal。放置 Tower 若會完全封死 Path 則會被拒絕。
- 三種 Tower:**Cannon**(高傷害、低射速)、**Gatling**(低傷害、高射速)、**Frost**(不攻擊,持續降速 Range 內的 Enemy)。Tower 可升級至 Tier 3,主要數值逐級提升 30%;升級與賣出前會跳出確認對話框,避免誤觸。
- 四種 Enemy:**Grunt**(中血中速)、**Runner**(低血高速)、**Tank**(高血低速),以及每 5 個 Wave 額外出現一次的 **Boss**(極高血量,目前無特殊機制)。
- 每個 Level 15 個 Wave,通過即進入下個 Level(Tower 自動賣出退還 70%,Gold/Lives 沿用);通過最後一個 Level 的全部 Wave 即 Victory,Lives 歸零則 Defeat。

## 開發用命令列(Command Palette)

按 `` ` ``(Backquote / `~`)鍵開啟測試用命令列,直接輸入指令後按 Enter 執行、Esc 關閉:

| 指令 | 說明 |
| --- | --- |
| `level <1..3>` | 直接跳到指定 Level(退還場上所有 Tower) |
| `gold <amount>` | 設定目前 Gold |
| `skipwave` | 瞬間清空進行中的 Wave(方便直接測試下一波) |

此為開發/測試輔助工具,不會影響正式遊玩。

## 開發環境需求

- [Rust](https://www.rust-lang.org/tools/install)(edition 2021)
- 若要以瀏覽器執行(wasm target):[trunk](https://trunkrs.dev/)

  ```sh
  rustup target add wasm32-unknown-unknown
  cargo install trunk
  ```

## 常用指令

專案提供 `Makefile` 包裝常用指令:

| 指令 | 說明 |
| --- | --- |
| `make run` / `make game` | 以原生視窗執行遊戲(`cargo run -p game`) |
| `make web` | 以瀏覽器開發模式執行(`trunk serve`,預設 port 8787) |
| `make web-build` | 建置瀏覽器版正式檔案(`trunk build --release`) |
| `make build` | 編譯整個 workspace |
| `make test` | 執行 workspace 全部測試 |
| `make fmt` | `cargo fmt --all` |
| `make lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `make clean` | 清除編譯產物與瀏覽器建置輸出 |

## 專案結構

```
simulation/   純遊戲邏輯(Grid、Path、Tower、Enemy、Wave 規則),不依賴 Bevy 的渲染層,可獨立測試
game/         Bevy 應用,負責渲染、輸入與把 simulation 的狀態呈現到畫面上
docs/adr/     架構決策紀錄(Architecture Decision Record)
CONTEXT.md    專案詞彙表,定義每個遊戲名詞的精確意涵與應避免的同義詞
```

`simulation` crate 刻意保持與 Bevy 無關,方便直接寫單元測試(見 `simulation/src/lib.rs` 內的 `tests` 模組),遊戲規則的變更應優先在這裡驗證。
