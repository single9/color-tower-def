# 09: Lives, Leak, and Victory/Defeat

**What to build:** The player starts with a fixed amount of Lives, shown live in the sidebar. Each Enemy that reaches the Goal (a Leak) decrements Lives by 1 instead of just despawning silently. Defeat triggers the instant Lives reaches 0, even mid-Wave — this overrides everything else in progress. Victory triggers once Wave 15's Enemy have all been cleared (dead or leaked) while Lives is still above 0.

**Blocked by:** 08 (Wave spawning and scaling)

**Status:** done

- [x] Sidebar shows current Lives, updating live
- [x] An Enemy reaching the Goal decrements Lives by 1 and despawns
- [x] Lives reaching 0 immediately ends the game in Defeat, even if a Wave is still mid-spawn or has live Enemy on the board
- [x] Clearing all of Wave 15's Enemy while Lives > 0 ends the game in Victory
- [x] Clearing a Wave before 15 does not trigger Victory — it simply allows "Start Next Wave" again
- [x] `simulation` unit tests cover: a Leak decrements Lives and emits a Leak event; Lives hitting 0 emits Defeat immediately regardless of remaining Enemy; Wave 15 clear with Lives > 0 emits Victory; a non-final Wave clear emits neither

## Verification

- Introduced `Simulation::tick`'s event system from the original spec: `tick` now returns `Vec<SimEvent>` (`EnemyKilled`, `Leak`, `WaveCleared`, `Victory`, `Defeat`) instead of `()`. Once `outcome()` is `Some`, `tick` is a permanent no-op — Defeat (checked immediately after Enemy movement, before Towers/Projectiles/Wave-completion run that same tick) and Victory both freeze the Simulation exactly where it stood.
- `cargo test --workspace`: 35/35 `simulation` unit tests pass (4 new: a Leak decrements Lives and emits a Leak event; Lives hitting 0 emits Defeat immediately regardless of remaining Enemy — including that the still-alive Tank is left untouched that tick and that all further ticks become no-ops; Wave 15 clear with Lives > 0 emits Victory; a non-final Wave clear emits WaveCleared but neither Victory nor Defeat).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected): sidebar starts at "Lives: 20"; starting Wave 1 with no Towers placed lets every Enemy walk unhindered to Goal — the faster Runner leaks first, dropping the live sidebar to "Lives: 19", then further Enemy leak in sequence down to "Lives: 17", confirming the Leak → Lives decrement pipeline end-to-end.
