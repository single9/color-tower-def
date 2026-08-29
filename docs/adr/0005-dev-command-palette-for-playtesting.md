# 加入僅供開發用的 Command Palette,取代逐關手動推進來測試 Level

要測試 Level 2、Level 3 的視覺與平衡,原本只能從 Level 1 開始,連續按「Start Next Wave」通過 15 個 Wave 才能自然推進到下一個 Level——沒有任何跳關方式,每次要驗證後面關卡都得重新刷一輪前面的 Wave,非常耗時。

決定加一個僅限開發用的 Command Palette:按 Backquote(`` ` ``)鍵開關,是側欄之外、疊在整個視窗底部的一條輸入列。支援三個指令:

- `level <1..LEVEL_COUNT>`:直接跳到指定 Level(1-indexed,對應側欄顯示的 `Level: X/3`)。內部呼叫 `Simulation::debug_set_level`,行為刻意比照 ADR-0003 描述的自然過關邏輯——把場上所有塔以正常賣出退款比例(70%)退回 Gold、清空 `towers`、重新產生該 Level 的 Grid、Wave 計數重置為 1;差別是這個指令可能在 Wave 進行中被呼叫,所以額外清空了 `spawn_queue` 與存活的 `enemies`(自然過關路徑保證呼叫時這兩者本來就是空的,不需要處理)。
- `gold <amount>`:直接增加 Gold,不用刷波數換取測試用的預算。
- `skipwave`:清空目前 Wave 尚未生成的佇列與所有存活 Enemy(不給擊殺獎勵),讓下一個 tick 的 `tick_wave_completion` 依原本邏輯自然判定該 Wave 已清空,銜接到下一個 Wave 或觸發 Level/Victory 的既有流程——刻意不另外寫一條「強制完成 Wave」的路徑,避免跟正常玩法的過關判定產生兩套邏輯。

這些指令都只在 `game` 這層新增(`simulation` crate 新增對應的 `debug_set_level` / `debug_add_gold` / `debug_skip_wave` 三個 `pub fn`,函式名以 `debug_` 前綴明確標示它們不是正常玩法會呼叫的 API),沒有任何遊戲內建的 UI 入口能觸發,純粹是開發者輸入指令才能用到。

## 連帶修正:Level 切換沒有清掉舊 Tower 的 Sprite

實作 `level` 指令時發現:`simulation` 的 `tick()` 在觸發 `SimEvent::LevelCleared` 時,只會清空自己內部的 `towers` 資料並退款,但 Bevy 層原本完全沒有監聽 `tick()` 回傳的事件——`tick_simulation` 直接把回傳值丟棄。這代表就算是「正常玩法」推滿 15 個 Wave 自然過關,畫面上舊 Level 的 Tower Sprite 也會留在原地不消失,是一個獨立於本次功能、原本就存在的 bug。既然 `level` 指令必須自己處理這個 despawn(直接呼叫時場上沒有事件流可以攔截),就一併讓 `tick_simulation` 也讀取 `tick()` 的回傳事件,遇到 `LevelCleared` 就做同樣的 despawn,兩條路徑(自然過關、`level` 指令跳關)現在共用同一個「清掉 Tower Sprite」的預期行為。
