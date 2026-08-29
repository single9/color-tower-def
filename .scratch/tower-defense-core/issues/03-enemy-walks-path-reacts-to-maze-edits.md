# 03: A single Enemy walks the Path, reacting to live maze edits

**What to build:** One Enemy (hardcoded to Grunt stats, spawned manually — e.g. on app start or a debug trigger, no Wave system yet) moves smoothly from Spawn toward Goal, cell-center to cell-center. It computes its own shortest remaining Path via BFS. Per ADR-0002, it only recomputes that Path when it reaches the center of its current Cell — placing or selling a Tower mid-transit does not make it teleport or snap; it keeps heading to the Cell it already committed to, then picks up the new Path at the next center. When it reaches the Goal Cell it despawns (Lives/Leak bookkeeping comes in ticket 09).

**Blocked by:** 02 (Place and sell a single Tower Kind, with the Blocking Rule enforced)

**Status:** ready-for-agent

- [ ] A Grunt-stat Enemy spawns at the Spawn Cell and visibly moves cell-by-cell toward the Goal Cell
- [ ] Placing or selling a Tower while the Enemy is mid-Cell does not change its current heading; the change is only reflected once it reaches the next Cell center
- [ ] Placing a Tower that changes the shortest route causes the Enemy to take the new route starting from its next Cell-center recalculation
- [ ] Reaching the Goal Cell despawns the Enemy
- [ ] `simulation` unit tests cover: an Enemy mid-Cell keeps its stored Path across a Grid mutation; an Enemy that has just reached a Cell center recomputes and picks up a changed Grid
