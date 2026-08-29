# 10: Result screen and restart

**What to build:** When Victory or Defeat fires (ticket 09), gameplay pauses and a result screen overlay appears indicating which outcome occurred, with a button to reset the level. Resetting restores a completely fresh game: empty Grid (all placed Tower removed), starting Gold, starting Lives, and Wave counter back to before Wave 1 — equivalent to a fresh app launch, without actually restarting the process.

**Blocked by:** 09 (Lives, Leak, and Victory/Defeat)

**Status:** ready-for-agent

- [ ] Reaching Victory shows a result overlay indicating Victory, with a reset button
- [ ] Reaching Defeat shows a result overlay indicating Defeat, with a reset button
- [ ] While the result overlay is showing, no further Grid clicks, Tower placement, or Wave starts have any effect
- [ ] Pressing the reset button clears all placed Tower, restores starting Gold and Lives, and returns the Wave counter to its pre-Wave-1 state
- [ ] After reset, the game is immediately playable again from the same pre-Wave preparation state as a fresh launch
