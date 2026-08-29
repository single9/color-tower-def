# 10: Result screen and restart

**What to build:** When Victory or Defeat fires (ticket 09), gameplay pauses and a result screen overlay appears indicating which outcome occurred, with a button to reset the level. Resetting restores a completely fresh game: empty Grid (all placed Tower removed), starting Gold, starting Lives, and Wave counter back to before Wave 1 — equivalent to a fresh app launch, without actually restarting the process.

**Blocked by:** 09 (Lives, Leak, and Victory/Defeat)

**Status:** done

- [x] Reaching Victory shows a result overlay indicating Victory, with a reset button
- [x] Reaching Defeat shows a result overlay indicating Defeat, with a reset button
- [x] While the result overlay is showing, no further Grid clicks, Tower placement, or Wave starts have any effect
- [x] Pressing the reset button clears all placed Tower, restores starting Gold and Lives, and returns the Wave counter to its pre-Wave-1 state
- [x] After reset, the game is immediately playable again from the same pre-Wave preparation state as a fresh launch

## Verification

- `simulation` gained hard guards enforcing "no effect while the game has ended" at the rules layer itself, not just the UI: `place_tower`/`can_place`, `upgrade_tower`, and `start_next_wave` all now return a new `GameOver` rejection once `outcome()` is set, and `sell_tower` becomes a no-op returning `false`. `interact_with_grid` additionally short-circuits Grid clicks entirely once `outcome()` is set, so hovering/clicking do nothing at the Bevy layer either.
- `cargo test --workspace`: 36/36 `simulation` unit tests pass (1 new: once the game has ended, placement, upgrade, sell, and start-next-wave all reject via the new `GameOver` variants, and sell leaves the existing Tower untouched).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected), using a temporary in-session fast-forward cheat (auto-starting Waves and ticking with a large dt, since no Tower were ever placed) to reach Defeat in well under a second of wall time instead of several real minutes: a dimming full-window overlay appeared showing "Defeat" in red and a "Reset" button; a Grid click and a "Start Next Wave" click while it was showing both had zero effect (Gold/Lives/Wave/board all unchanged); clicking Reset immediately cleared the overlay and restored Gold: 200, Lives: 20, Wave: 1, and an empty board — identical to a fresh launch. Victory shares the exact same overlay-rendering code path (differing only in the label/color match arm), and its state transition is already covered by `simulation`'s own Wave-15-clear unit test, so it was not separately re-verified visually.
