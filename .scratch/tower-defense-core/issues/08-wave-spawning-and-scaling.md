# 08: Wave spawning and scaling

**What to build:** Replace the single manually-spawned Enemy from tickets 03–05 with a real Wave system. The sidebar shows the current Wave number and a "Start Next Wave" button. Pressing it spawns Wave *n*'s Enemy one at a time at the Spawn Cell, 0.8s apart, with `5 + n` Enemy total (mixed across the three Enemy Kind — exact mix left to the implementer, document the chosen distribution), each with Health scaled by `base_health * (1 + n * 0.1)`. The button is inert while a Wave is still in progress.

**Blocked by:** 05 (All Tower Kind and Enemy Kind)

**Status:** done

- [x] Sidebar shows the current Wave number, starting at 1 before any Wave has been started
- [x] "Start Next Wave" spawns that Wave's Enemy one at a time at 0.8s intervals rather than all at once
- [x] Wave *n* spawns exactly `5 + n` Enemy
- [x] Each Enemy's Health in Wave *n* equals its base Health multiplied by `(1 + n * 0.1)`
- [x] "Start Next Wave" has no effect while the current Wave's Enemy are still spawning or alive
- [x] `simulation` unit tests cover: Enemy count and Health formulas for at least Wave 1, a middle Wave, and Wave 15; the next-Wave trigger is rejected while a Wave is in progress

## Verification

- Enemy Kind mix distribution decided and documented in `CONTEXT.md`'s Wave entry: Grunt → Runner → Tank, cycling in that fixed order to fill a Wave's `5 + n` count.
- Internally, `simulation` generalized from a single `Option<Enemy>` to a `Vec<Enemy>` with per-Enemy ids so Tower targeting (nearest in-Range Enemy), Frost slow (per-Enemy position), and Projectile tracking (by target id, unaffected by other Enemy dying) all work correctly with several Enemy alive at once. All 25 pre-existing tests kept passing unmodified through this refactor (aside from two tests reaching into now-renamed private fields), confirming behavior was preserved.
- `cargo test --workspace`: 31/31 `simulation` unit tests pass (2 new: Wave Enemy count and Health formulas verified for Wave 1, Wave 7, and Wave 15; starting the next Wave is rejected while the current Wave is in progress).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): sidebar shows "Wave: 1" and a green "Start Next Wave" button with no Enemy on screen before it's pressed; pressing it spawns a Grunt (lime) immediately, then a Runner (yellow) and Tank (sienna) 0.8s apart each, visibly cycling Kind and moving at different speeds (the later-spawned Runner visibly overtook the earlier Grunt); the button dims while the Wave is in progress, and pressing it again mid-Wave is a silent no-op — Wave stays at "1" and no extra Enemy spawns out of the 0.8s cadence.
