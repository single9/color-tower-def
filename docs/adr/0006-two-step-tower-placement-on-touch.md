# 觸控放置 Tower 改為兩段式,滑鼠維持單擊

在手機(觸控)上放置 Tower 原本和滑鼠一樣是「一下就放」:手指碰到哪個 Cell 就直接扣 Gold 放下 Tower。這在滑鼠上沒問題,因為滑鼠有 hover——游標移到 Cell 上時就已經看得到 Path 預覽與 Range 圓圈,點擊只是確認早就看見的結果。手指沒有 hover 這個階段:玩家第一次「看到」預覽的瞬間,Tower 已經放下去了,而且手指本身還遮住那個 Cell,誤觸的代價是白花 Gold(以及可能整條 Path 被改道)。

決定把觸控的放置拆成兩段,滑鼠維持原本的單擊不變:

- **第一下**:只決定位置,不放置。該 Cell 顯示一個半透明的 ghost Tower(該 Tower Kind 的顏色,alpha 0.4),連同放置後的 Path 預覽與 Tier 1 的 Range 圓圈。透明度是刻意壓低的——玩家要能一眼看出「這只是在確認位置」,而不是已經放好的 Tower。
- **第二下(同一個 Cell)**:才真的放置並扣 Gold。
- 點到別的 Buildable Cell 就是把待確認的位置移過去(仍然不放置);點到已有 Tower 的 Cell 是選取那座 Tower(照舊開啟資訊面板);點到不可建的 Cell 則取消待確認狀態。點在 Grid 以外(例如側欄)則不影響待確認的位置,所以放置前還能切換 Tower Kind,ghost 會直接換色。

待確認的 Cell 存在 `PendingPlacement` 這個 Resource,滑鼠路徑不會寫入它,因此桌機的行為完全沒變。它同時取代 hover 成為預覽的來源(`PreviewTarget`):手指抬起後 `Touches` 就沒有位置了,若仍以 hover 為準,Path 與 Range 預覽會在手指離開的瞬間跟著消失,玩家等於沒有東西可以確認。

另外,觸控裝置上的瀏覽器會在每次 tap 之後補送一組合成的 mouse 事件(mousedown/mouseup/click)。若照單全收,第一下 tap 剛把 Cell 設為待確認,緊接著的合成 click 就會直接把 Tower 放下去,兩段式等於沒做。因此記錄最後一次觸控的時間(`LastTouchAt`),在其後 1 秒內忽略滑鼠輸入;純滑鼠裝置永遠不會進入這個狀態,混合裝置(觸控筆電)也只是在剛碰過螢幕的 1 秒內滑鼠不作用。

同時修掉一個既有的漏點:Upgrade/Sell 的確認對話框是覆蓋在 Grid 上的,原本按下對話框中央的 Confirm/Cancel 會連帶觸發底下那個 Cell 的放置。改為對話框開著時 Grid 完全不接受點擊/觸控。
