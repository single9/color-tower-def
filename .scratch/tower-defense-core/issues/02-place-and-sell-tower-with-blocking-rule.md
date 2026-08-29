# 02: Place and sell a single Tower Kind, with the Blocking Rule enforced

**What to build:** The player can click a Buildable Cell to place a Tower there (Tower Kind hardcoded to Cannon for now, no Gold cost yet). Before committing, a translucent yellow Path preview shows the route from Spawn to Goal that would result. A placement that would leave no Path from Spawn to Goal is rejected outright — the Cell stays Buildable. Clicking an already-placed Tower removes it (free, no refund logic yet). This is the seam where the `simulation` module's Grid/Cell/Tower types and the BFS-based Blocking Rule get built, independent of Bevy.

**Blocked by:** 01 (Grid renders)

**Status:** done

- [x] Clicking a Buildable Cell places a Tower there and the Cell renders in the Tower's color (red, for Cannon)
- [x] Hovering/selecting a Buildable Cell before confirming shows the resulting Path as a translucent yellow overlay
- [x] Attempting to place a Tower that would leave no Path from Spawn to Goal is rejected — no Tower is placed, the Cell remains Buildable
- [x] Clicking a Cell that already has a Tower removes it, reverting the Cell to Buildable
- [x] `simulation` module unit tests cover: valid placement succeeds, placement that would fully seal the maze is rejected, placement that leaves any path (however narrow) succeeds, sell removes the Tower and frees the Cell

## Verification

- `cargo test --workspace`: 8/8 `simulation` unit tests pass, covering the Blocking Rule (full-seal rejection + narrow-gap success), duplicate/Spawn/Goal placement rejection, and sell freeing the Cell.
- Manually driven via synthetic X11 input against the running Bevy window (screenshots inspected): hover shows the yellow Path preview, a click places a red Tower, clicking it again sells it, and attempting to enclose Spawn's third and last open neighbor is silently rejected (Cell stays Buildable) while the first two succeed.
