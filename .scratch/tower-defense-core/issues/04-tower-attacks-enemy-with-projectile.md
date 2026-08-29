# 04: Tower attacks Enemy with Projectile

**What to build:** The (hardcoded Cannon) Tower from ticket 02 fires a Projectile at the Grunt Enemy from ticket 03 whenever it is within Range. The Projectile tracks the Enemy's current position each tick and moves toward it at a fixed speed (ADR-0001: no physics engine, plain distance check). When the distance to the target drops under a small threshold, the hit registers: damage is applied to the Enemy's Health and the Projectile despawns. If the Enemy dies from the hit (or from a prior hit) before the Projectile arrives, the Projectile despawns without effect. An Enemy whose Health reaches zero despawns before reaching the Goal.

**Blocked by:** 03 (A single Enemy walks the Path, reacting to live maze edits)

**Status:** ready-for-agent

- [ ] A placed Cannon Tower fires a visible Projectile at the Enemy once it enters Range
- [ ] The Projectile visibly tracks the Enemy's movement rather than flying in a straight fixed line
- [ ] On hit, the Enemy's Health decreases by the Tower's damage amount and the Projectile despawns
- [ ] An Enemy whose Health reaches zero despawns immediately, before it can reach the Goal
- [ ] A Projectile whose target dies before arrival despawns cleanly with no error and no effect on any other Enemy
- [ ] `simulation` unit tests cover: a hit applies damage and removes the Projectile; Health reaching zero kills the Enemy; a Projectile targeting an already-dead Enemy is a no-op
