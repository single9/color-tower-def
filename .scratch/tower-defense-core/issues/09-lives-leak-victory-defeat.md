# 09: Lives, Leak, and Victory/Defeat

**What to build:** The player starts with a fixed amount of Lives, shown live in the sidebar. Each Enemy that reaches the Goal (a Leak) decrements Lives by 1 instead of just despawning silently. Defeat triggers the instant Lives reaches 0, even mid-Wave — this overrides everything else in progress. Victory triggers once Wave 15's Enemy have all been cleared (dead or leaked) while Lives is still above 0.

**Blocked by:** 08 (Wave spawning and scaling)

**Status:** ready-for-agent

- [ ] Sidebar shows current Lives, updating live
- [ ] An Enemy reaching the Goal decrements Lives by 1 and despawns
- [ ] Lives reaching 0 immediately ends the game in Defeat, even if a Wave is still mid-spawn or has live Enemy on the board
- [ ] Clearing all of Wave 15's Enemy while Lives > 0 ends the game in Victory
- [ ] Clearing a Wave before 15 does not trigger Victory — it simply allows "Start Next Wave" again
- [ ] `simulation` unit tests cover: a Leak decrements Lives and emits a Leak event; Lives hitting 0 emits Defeat immediately regardless of remaining Enemy; Wave 15 clear with Lives > 0 emits Victory; a non-final Wave clear emits neither
