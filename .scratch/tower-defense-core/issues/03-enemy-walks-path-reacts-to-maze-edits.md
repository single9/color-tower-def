# 03: A single Enemy walks the Path, reacting to live maze edits

**What to build:** One Enemy (hardcoded to Grunt stats, spawned manually — e.g. on app start or a debug trigger, no Wave system yet) moves smoothly from Spawn toward Goal, cell-center to cell-center. It computes its own shortest remaining Path via BFS. Per ADR-0002, it only recomputes that Path when it reaches the center of its current Cell — placing or selling a Tower mid-transit does not make it teleport or snap; it keeps heading to the Cell it already committed to, then picks up the new Path at the next center. When it reaches the Goal Cell it despawns (Lives/Leak bookkeeping comes in ticket 09).

**Blocked by:** 02 (Place and sell a single Tower Kind, with the Blocking Rule enforced)

**Status:** done

- [x] A Grunt-stat Enemy spawns at the Spawn Cell and visibly moves cell-by-cell toward the Goal Cell
- [x] Placing or selling a Tower while the Enemy is mid-Cell does not change its current heading; the change is only reflected once it reaches the next Cell center
- [x] Placing a Tower that changes the shortest route causes the Enemy to take the new route starting from its next Cell-center recalculation
- [x] Reaching the Goal Cell despawns the Enemy
- [x] `simulation` unit tests cover: an Enemy mid-Cell keeps its stored Path across a Grid mutation; an Enemy that has just reached a Cell center recomputes and picks up a changed Grid

## Verification

- `cargo test --workspace`: 12/12 `simulation` unit tests pass (4 new: spawn heading, mid-Cell Path stability, cell-center recompute picking up a new Tower, Goal despawn).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): Enemy spawns and walks the straight row toward Goal; a Tower placed several cells ahead is left alone while the Enemy is still mid-Cell; once it reaches the next Cell center it detours around the Tower; it reaches Goal and the sprite despawns.
