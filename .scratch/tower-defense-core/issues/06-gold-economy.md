# 06: Gold economy

**What to build:** The player starts with a fixed amount of Gold, shown live in the sidebar. Placing a Tower deducts its Tower Kind's fixed purchase price; placement is rejected if the player can't afford it (in addition to the existing Blocking Rule check). Killing an Enemy grants Gold, with the amount depending on Enemy Kind. Selling a Tower refunds 70% of the total Gold spent on it (purchase price only for now — upgrade costs get folded into this refund calculation in ticket 07).

**Blocked by:** 05 (All Tower Kind and Enemy Kind)

**Status:** ready-for-agent

- [ ] Sidebar shows current Gold, updating live as it changes
- [ ] Placing a Tower deducts its purchase price from Gold
- [ ] Attempting to place a Tower the player can't afford is rejected — no Gold is deducted, no Tower is placed
- [ ] Killing a Grunt, Runner, and Tank each grant their own distinct Gold amount
- [ ] Selling a Tower refunds 70% of its purchase price, rounded consistently (document the rounding rule)
- [ ] `simulation` unit tests cover: affordable placement deducts correctly; unaffordable placement is rejected and Gold is unchanged; each Enemy Kind's kill reward is correct; sell refund math for a Tower with no upgrades
