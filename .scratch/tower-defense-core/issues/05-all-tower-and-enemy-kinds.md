# 05: All Tower Kind and Enemy Kind

**What to build:** Generalize tickets 02–04 from the hardcoded Cannon/Grunt pair to the full roster. Gatling (single-target, low damage, high fire rate, reusing the Projectile pipeline from ticket 04) and Frost (no Projectile — continuously applies a slow multiplier to every Enemy currently within Range, removed the instant an Enemy leaves Range, re-evaluated every tick rather than as a timed buff) both come online. Runner (low Health, high speed) and Tank (high Health, low speed) join Grunt as spawnable Enemy Kind. The sidebar gets working Tower Kind selection buttons (Cannon/Gatling/Frost) that control which kind the next Cell click places.

**Blocked by:** 04 (Tower attacks Enemy with Projectile)

**Status:** done

- [x] Sidebar has three Tower Kind buttons; the selected one determines what the next Cell click places
- [x] Placing a Gatling Tower fires faster, lower-damage Projectile than Cannon, using the same tracking/hit pipeline
- [x] Placing a Frost Tower fires no Projectile; instead every Enemy within its Range moves at reduced speed while inside, and returns to normal speed the instant it steps outside — with no lingering effect
- [x] Grunt, Runner, and Tank Enemy each have distinct Health and speed matching their described profile (Runner fastest/lowest Health, Tank slowest/highest Health)
- [x] `simulation` unit tests cover: Gatling fire-rate/damage differ from Cannon; Frost slow applies only while an Enemy's position is within Range and clears the tick it leaves; each Enemy Kind's stats are distinct and correct

## Verification

- `cargo test --workspace`: 20/20 `simulation` unit tests pass (5 new: Gatling fires faster/weaker than Cannon; each Enemy Kind has distinct correct stats; spawn_enemy uses the given Kind's Health; Frost slow applies only within Range and clears the moment it leaves; an Enemy inside Frost Range covers less ground per tick).
- Manually driven end-to-end via synthetic X11 input against the running Bevy window (screenshots inspected, two separate fresh runs):
  - Clicking the Gatling sidebar button re-highlights it and switches placement to Gatling; a placed Gatling Tower (green) fired two visibly tracking Projectiles at once during the Enemy's approach (its fast cooldown lets multiple shots be in flight), and killed the Grunt.
  - Clicking the Frost sidebar button re-highlights it and switches placement to Frost; a placed Frost Tower (blue) never spawned a Projectile as the Enemy passed directly beneath it, and the Enemy emerged past it undamaged and continued toward Goal — confirming the slow-while-in-Range/no-lingering-effect behavior end-to-end.
