# 破關後推進到下一個 Level 而非直接 Victory

原本整場遊戲只有一張固定地圖,通過第 15 個 Wave 就直接 Victory。為了讓 Path 依關卡產生變化(從單調的一直線,變成需要繞路的迷宮),新增了 Level 概念:每個 Level 有各自固定的 Obstacle 佈局,但共用同一組 Spawn/Goal 座標與 Wave 規則。

通過某個 Level 的第 15 個 Wave 時:

- 若不是最後一個 Level,遊戲不觸發 Victory,而是切換到下一個 Level 的 Grid、Wave 計數重置為 1。
- 只有在最後一個 Level 通過第 15 個 Wave,才真正觸發 Victory。

切換 Level 時,場上所有 Tower 會被賣出並依原本的退款比例(70%)退還 Gold——因為新地圖的 Obstacle 佈局不同,舊 Tower 的位置不一定還合法(可能落在新的 Obstacle 上),與其特殊處理每個位置的合法性,不如直接清空重來,讓玩家在新地圖上重新佈局。Gold 與 Lives 則沿用不重置,因為這是同一場遊戲的延續,不是重新開局。

若之後要讓 Tower 在切換 Level 後保留(例如允許玩家跨 Level 保留佈局),需要重新設計 Obstacle 佈局規則,確保新舊 Level 的可建塔區域相容。
