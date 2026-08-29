# 06: Gold economy

**What to build:** The player starts with a fixed amount of Gold, shown live in the sidebar. Placing a Tower deducts its Tower Kind's fixed purchase price; placement is rejected if the player can't afford it (in addition to the existing Blocking Rule check). Killing an Enemy grants Gold, with the amount depending on Enemy Kind. Selling a Tower refunds 70% of the total Gold spent on it (purchase price only for now — upgrade costs get folded into this refund calculation in ticket 07).

**Blocked by:** 05 (All Tower Kind and Enemy Kind)

**Status:** done

- [x] Sidebar shows current Gold, updating live as it changes
- [x] Placing a Tower deducts its purchase price from Gold
- [x] Attempting to place a Tower the player can't afford is rejected — no Gold is deducted, no Tower is placed
- [x] Killing a Grunt, Runner, and Tank each grant their own distinct Gold amount
- [x] Selling a Tower refunds 70% of its purchase price, rounded consistently (document the rounding rule)
- [x] `simulation` unit tests cover: affordable placement deducts correctly; unaffordable placement is rejected and Gold is unchanged; each Enemy Kind's kill reward is correct; sell refund math for a Tower with no upgrades

## Verification

- `cargo test --workspace`: 25/25 `simulation` unit tests pass (5 new: affordable placement deducts the Tower's price; unaffordable placement is rejected and Gold is unchanged; each Enemy Kind grants its own distinct kill reward; killing an Enemy grants its kill reward; selling a Tower with no upgrades refunds 70% of its price, rounded to the nearest whole Gold, .5 away from zero via `f32::round`).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): sidebar starts at "Gold: 200"; placing a Cannon (100 Gold) drops it live to "Gold: 100"; selling that Cannon refunds 70 to "Gold: 170"; placing another Cannon drops it to "Gold: 70"; attempting a third Cannon (100 Gold, unaffordable) is silently rejected — Gold stays at "Gold: 70" and no second Tower sprite appears.
