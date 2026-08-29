# Tower Defense

一款以色塊呈現的迷宮式塔防遊戲(Rust + Bevy)。玩家在 25x25 的 Grid 上放置 Tower 阻擋 Enemy 的行進路線,依序通過 3 個 Level、每個 Level 15 個 Wave,守住 Base 不被攻破。

## Language

### 地圖與路徑

**Grid**:
25x25 個 Cell 組成的遊戲場地,是 Tower 放置與 Enemy 移動的唯一空間。
_Avoid_: Board, Map(Map 泛指整個關卡,Grid 專指格子本身)

**Cell**:
Grid 上的單一格子,是放置 Tower 或 Enemy 通行的最小單位。每個 Cell 有一個 CellKind:Buildable(可建塔空地)、Spawn(出生點)、Goal(終點)、Obstacle(該 Level 固定的永久障礙,見下)。放置 Tower 後的 Cell 視為被佔用,無法再放置或通行。
_Avoid_: Tile, Square

**Spawn**:
Grid 上唯一的 Enemy 生成位置,每個 Wave 的 Enemy 都從這裡依序出現。
_Avoid_: Start, Entry

**Goal**:
Grid 上唯一的終點,即玩家的 Base 所在位置。Enemy 抵達 Goal 視為一次 Leak。
_Avoid_: End, Base(Base 是概念上的目標物,Goal 是它在 Grid 上的座標)

**Path**:
單一 Enemy 從自己「當前所在 Cell」到 Goal 的最短路徑,由尋路演算法即時計算,繞過所有已放置的 Tower。每隻 Enemy 各自持有自己的 Path,並在抵達下一個 Cell 中心點時重新計算——因此同時間場上不同 Enemy 的 Path 可能不同。
_Avoid_: Route(單數即可,不需另立新詞)

**Blocking Rule**:
放置 Tower 時的限制:若該次放置會導致 Spawn 到 Goal 之間完全沒有可通行的 Path,則該次放置被拒絕。
_Avoid_: Maze rule

**Obstacle**:
Level 地圖中固定烘焙、永久性的不可建塔 Cell,用來讓 Path 依 Level 產生轉折,和玩家放置的 Tower 是不同概念——Obstacle 不能被賣出或升級,也不佔用 Gold。
_Avoid_: Wall(Wall 只是視覺呈現,Obstacle 才是規則層級的名詞)

**Level**:
遊戲目前所在的地圖關卡,決定 Grid 上 Obstacle 的佈局(進而影響 Path 的走向),但 Spawn/Goal 位置在每個 Level 間固定不變。共 3 個 Level,依序遞增;在最後一個 Level 之前,通過該 Level 全部 15 個 Wave 即進入下一個 Level(重置 Wave 計數、賣出場上所有 Tower 並退還 70%,Gold 與 Lives 沿用);只有在最後一個 Level 通過全部 Wave 才會觸發 Victory。
_Avoid_: Stage, Map(Map 是 Level 在 Grid 上的具體呈現,Level 才是遊戲進度的計數名詞)

### 防禦塔

**Tower**:
玩家花費 Gold 放置在 Buildable Cell 上的防禦單位,會攻擊進入其 Range 的 Enemy,同時也是阻擋 Path 的障礙物。
_Avoid_: Turret, Defender

**Tower Kind**:
Tower 的類型,決定其攻擊方式與數值傾向。目前有三種:

- **Cannon**:單體攻擊、高傷害、低射速(對應紅色)
- **Gatling**:單體攻擊、高射速、低傷害(對應綠色)
- **Frost**:持續套用 Slow Aura,不主動攻擊(對應藍色)

_Avoid_: Red Tower / Green Tower / Blue Tower(顏色是 Tower Kind 的視覺呈現,不是名稱本身)

**Range**:
Tower 的有效作用半徑。Cannon 與 Gatling 在 Range 內鎖定 Enemy 發射 Projectile;Frost 對 Range 內所有 Enemy 持續套用 Slow Aura。

**Projectile**:
Cannon 或 Gatling 發射後鎖定 Enemy 並跟隨其移動的攻擊實體,命中即扣除 Enemy 的 Health,保證命中。
_Avoid_: Bullet(Projectile 涵蓋所有攻擊性拋射物,不限定外觀)

**Slow Aura**:
Frost 對 Range 內所有 Enemy 持續套用的降速效果;Enemy 離開 Range 後效果立即解除,不具持續時間。
_Avoid_: Slow effect, Debuff

**Tier**:
Tower 的升級等級,共三級:基礎(Tier 1)、Tier 2、Tier 3。每升一級,主要數值提升 30%(較上一 Tier,非疊加自 Tier 1 累乘外的額外加成——三級數值為 Tier 1 的 1.3 的平方倍);升級花費固定為原始購買價的 80%,兩次升級(Tier 1→2、Tier 2→3)費用相同,不隨 Tier 遞增。主要數值依 Tower Kind 而定:Cannon/Gatling 是 Damage;Frost 沒有 Damage,主要數值是 Range(實作於 ticket 07 時決定——Frost 的 Slow Aura 強度维持固定,Range 隨 Tier 擴大)。
_Avoid_: Level(Level 是地圖關卡的計數,Tier 專指 Tower 的升級等級,兩者互不相關)

### 敵人與波次

**Enemy**:
沿著自己的 Path 從 Spawn 走向 Goal 的移動單位,抵達 Goal 視為一次 Leak。
_Avoid_: Monster, Creep

**Enemy Kind**:
Enemy 的類型,決定 Health 與移動速度的傾向。目前有三種:

- **Grunt**:中 Health、中速度
- **Runner**:低 Health、高速度
- **Tank**:高 Health、低速度

_Avoid_: Mob type

**Health**:
Enemy 目前剩餘的生命值,歸零即死亡並掉落 Gold。
_Avoid_: HP(與 Lives 都可能被稱為「生命」,用 Health 專指 Enemy,用 Lives 專指玩家,避免混淆)

**Wave**:
玩家手動觸發的一次敵人生成事件,包含固定數量與強度的 Enemy,依序間隔(0.8 秒)出現於 Spawn。每個 Level 內共 15 個 Wave,全數通過即完成該 Level(見上方 Level)。第 n 個 Wave 生成 `5 + n` 隻 Enemy,每隻 Health 為基礎值乘上 `1 + n * 0.1`;Enemy Kind 依 Grunt → Runner → Tank 固定順序循環分配(實作於 ticket 08 時決定,任意但均勻的組成)。Wave 進行中(尚有 Enemy 待生成或存活)無法觸發下一個 Wave;每個新 Level 開始時 Wave 計數重置為 1。
_Avoid_: Round(Level 現在是獨立於 Wave 的地圖關卡概念,見上方定義,不再互相避免)

**Leak**:
一隻 Enemy 抵達 Goal 的事件,會使玩家扣除 1 點 Lives。
_Avoid_: Escape, Breach

### 玩家狀態與勝負

**Lives**:
玩家目前剩餘的生命值,每次 Leak 扣 1 點,歸零即 Defeat。
_Avoid_: Health(專指 Enemy 的生命值,見上)、HP

**Gold**:
玩家的貨幣,用於放置與升級 Tower;擊殺 Enemy 依 Enemy Kind 獲得對應數量,賣出 Tower 退還其花費的 70%。
_Avoid_: Money, Currency, Coins

**Victory**:
玩家通過最後一個 Level 的全部 15 個 Wave 且 Lives 未歸零時達成的勝利狀態;在最後一個 Level 之前通過全部 Wave 只會進入下一個 Level,不算 Victory。

**Defeat**:
玩家 Lives 歸零時達成的落敗狀態。
