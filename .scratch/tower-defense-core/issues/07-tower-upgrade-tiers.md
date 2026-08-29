# 07: Tower upgrade Tiers

**What to build:** Clicking an already-placed Tower opens a sidebar panel showing its Tower Kind, current Tier, and stats, with Upgrade and Sell buttons (Sell here replaces the free-removal behavior from ticket 02 with the Gold-refund behavior from ticket 06). Upgrading is only available below Tier 3: it costs 80% of the Tower's original purchase price, deducted from Gold, and increases the Tower's primary stat (damage for Cannon/Gatling, the chosen stat for Frost — see spec's Further Notes) by 30% over the previous Tier. The sell refund now accounts for total Gold spent (purchase price plus all upgrade costs paid), still refunding 70% of that total.

**Blocked by:** 06 (Gold economy)

**Status:** ready-for-agent

- [ ] Clicking a placed Tower opens an info panel showing its Tower Kind, Tier, and current stats
- [ ] The Upgrade button is disabled/hidden at Tier 3
- [ ] Upgrading below Tier 3 deducts 80% of the Tower's original purchase price from Gold and increases its primary stat by 30% over its current value
- [ ] Attempting to upgrade without enough Gold is rejected — no Tier change, no Gold deducted
- [ ] Selling a Tower that has been upgraded refunds 70% of (purchase price + all upgrade costs paid), not just the base purchase price
- [ ] `simulation` unit tests cover: upgrade cost and stat increase at each Tier transition; upgrade blocked at Tier 3; upgrade blocked when unaffordable; sell refund correctly sums purchase + upgrade spend before applying 70%
