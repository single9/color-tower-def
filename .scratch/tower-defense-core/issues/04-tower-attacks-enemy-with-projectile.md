# 04: Tower attacks Enemy with Projectile

**What to build:** The (hardcoded Cannon) Tower from ticket 02 fires a Projectile at the Grunt Enemy from ticket 03 whenever it is within Range. The Projectile tracks the Enemy's current position each tick and moves toward it at a fixed speed (ADR-0001: no physics engine, plain distance check). When the distance to the target drops under a small threshold, the hit registers: damage is applied to the Enemy's Health and the Projectile despawns. If the Enemy dies from the hit (or from a prior hit) before the Projectile arrives, the Projectile despawns without effect. An Enemy whose Health reaches zero despawns before reaching the Goal.

**Blocked by:** 03 (A single Enemy walks the Path, reacting to live maze edits)

**Status:** done

- [x] A placed Cannon Tower fires a visible Projectile at the Enemy once it enters Range
- [x] The Projectile visibly tracks the Enemy's movement rather than flying in a straight fixed line
- [x] On hit, the Enemy's Health decreases by the Tower's damage amount and the Projectile despawns
- [x] An Enemy whose Health reaches zero despawns immediately, before it can reach the Goal
- [x] A Projectile whose target dies before arrival despawns cleanly with no error and no effect on any other Enemy
- [x] `simulation` unit tests cover: a hit applies damage and removes the Projectile; Health reaching zero kills the Enemy; a Projectile targeting an already-dead Enemy is a no-op

## Verification

- `cargo test --workspace`: 15/15 `simulation` unit tests pass (3 new: hit applies damage and removes the Projectile, two hits kill the Grunt, a Projectile whose target died mid-flight is cleared without panic or effect).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): a placed Cannon visibly fires a white Projectile that tracks the moving Enemy (not a straight line to a fixed point); a hit removes the Projectile; two hits kill the Enemy, which despawns a couple of Cells from Spawn — well short of Goal.
