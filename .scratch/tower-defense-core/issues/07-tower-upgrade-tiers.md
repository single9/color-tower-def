# 07: Tower upgrade Tiers

**What to build:** Clicking an already-placed Tower opens a sidebar panel showing its Tower Kind, current Tier, and stats, with Upgrade and Sell buttons (Sell here replaces the free-removal behavior from ticket 02 with the Gold-refund behavior from ticket 06). Upgrading is only available below Tier 3: it costs 80% of the Tower's original purchase price, deducted from Gold, and increases the Tower's primary stat (damage for Cannon/Gatling, the chosen stat for Frost — see spec's Further Notes) by 30% over the previous Tier. The sell refund now accounts for total Gold spent (purchase price plus all upgrade costs paid), still refunding 70% of that total.

**Blocked by:** 06 (Gold economy)

**Status:** done

- [x] Clicking a placed Tower opens an info panel showing its Tower Kind, Tier, and current stats
- [x] The Upgrade button is disabled/hidden at Tier 3
- [x] Upgrading below Tier 3 deducts 80% of the Tower's original purchase price from Gold and increases its primary stat by 30% over its current value
- [x] Attempting to upgrade without enough Gold is rejected — no Tier change, no Gold deducted
- [x] Selling a Tower that has been upgraded refunds 70% of (purchase price + all upgrade costs paid), not just the base purchase price
- [x] `simulation` unit tests cover: upgrade cost and stat increase at each Tier transition; upgrade blocked at Tier 3; upgrade blocked when unaffordable; sell refund correctly sums purchase + upgrade spend before applying 70%

## Verification

- `cargo test --workspace`: 29/29 `simulation` unit tests pass (4 new: upgrading increases damage 30% per Tier and deducts a flat cost; upgrade is blocked at Tier 3; upgrade is blocked when unaffordable and state is unchanged; selling an upgraded Tower refunds 70% of purchase plus upgrade spend).
- Decided and documented in `CONTEXT.md`'s Tier entry: Frost's primary stat (the one Tiers scale) is Range, since it has no damage; Cannon/Gatling's primary stat is damage.
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): placing a Cannon then clicking it again opens the sidebar info panel ("Cannon (Tier 1)", "Damage: 50", Upgrade (80g), Sell) instead of instantly selling it; clicking Upgrade moves it to Tier 2 with Damage 65 (50 × 1.3) and Gold dropping by 80; clicking Upgrade again while unaffordable (Gold 20 < 80) is a silent no-op — still Tier 2, Gold unchanged; clicking Sell refunds 70% of (100 + 80) = 126 Gold, despawns the Tower sprite, and closes the panel.

## Comments

- UI follow-up (2026-08-30): upgrading and selling now require an explicit confirmation dialog before committing. The verification notes above describe the earlier direct-click behavior; the info panel upgrades/sells via a Confirm/Cancel dialog in the game layer.
