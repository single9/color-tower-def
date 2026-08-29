# 08: Wave spawning and scaling

**What to build:** Replace the single manually-spawned Enemy from tickets 03–05 with a real Wave system. The sidebar shows the current Wave number and a "Start Next Wave" button. Pressing it spawns Wave *n*'s Enemy one at a time at the Spawn Cell, 0.8s apart, with `5 + n` Enemy total (mixed across the three Enemy Kind — exact mix left to the implementer, document the chosen distribution), each with Health scaled by `base_health * (1 + n * 0.1)`. The button is inert while a Wave is still in progress.

**Blocked by:** 05 (All Tower Kind and Enemy Kind)

**Status:** ready-for-agent

- [ ] Sidebar shows the current Wave number, starting at 1 before any Wave has been started
- [ ] "Start Next Wave" spawns that Wave's Enemy one at a time at 0.8s intervals rather than all at once
- [ ] Wave *n* spawns exactly `5 + n` Enemy
- [ ] Each Enemy's Health in Wave *n* equals its base Health multiplied by `(1 + n * 0.1)`
- [ ] "Start Next Wave" has no effect while the current Wave's Enemy are still spawning or alive
- [ ] `simulation` unit tests cover: Enemy count and Health formulas for at least Wave 1, a middle Wave, and Wave 15; the next-Wave trigger is rejected while a Wave is in progress
