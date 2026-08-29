# Tower 的 Range 以 Gizmos 圓圈視覺化,而非只靠文字面板

原本只有選中 Frost Tower 時,資訊面板會以文字顯示數字化的 Range(`Range: 12.0`);Cannon/Gatling 完全不顯示 Range,任何 Tower Kind 在放置前(hover 階段)也都沒有任何 Range 相關提示。評估後認為這不夠——Range 直接決定放置決策,尤其 Frost 的 Slow Aura 是離開 Range 立即解除、不具持續時間(見 CONTEXT.md 的 Slow Aura 定義),玩家很難從一個數字換算出它在 Grid 座標系裡實際涵蓋哪些 Cell。

決定改為所有 Tower Kind 都在 Grid 上疊加一個半透明白色 Range 圓圈,涵蓋兩種情境:

- 選中一座已放置的 Tower(`SelectedTower`)時,顯示它目前 Tier 的實際 Range。
- 尚未選中任何 Tower、且滑鼠正 hover 在可放置的 Buildable Cell 上時,依側欄目前選中的 Tower Kind,預覽該 Kind 在 Tier 1 的 Range(放置後的起始 Range,而非最終 Tier 3)。

圓圈用 Bevy 的 `Gizmos::circle_2d` 每幀直接畫,而不是像 `PathPreviewTile`/Enemy/Projectile 那樣用常駐 Sprite entity 搭配「每幀 despawn 再重新 spawn」。因為 Gizmos 本來就是為了這種每幀重繪的疊加圖層設計的,不需要额外的 Marker Component 與 despawn 迴圈,比照既有 Sprite-overlay 模式反而是多餘的樣板碼。

為了讓放置前的 Tier 1 Range 預覽可以查詢,把原本私有的 `TowerKind::range` 方法改為 `pub`(`simulation` crate),讓 Bevy 層可以在還沒有 `TowerStats`(即尚未放置)的情況下查到基礎 Range。

資訊面板的文字內容維持原樣不變(Frost 顯示 `Range`,Cannon/Gatling 顯示 `Damage`)——圓圈已經涵蓋所有 Kind 的 Range 視覺化需求,沒有必要再讓面板重複顯示對 Cannon/Gatling 而言只是次要數值的 Range 數字。
