# 05: All Tower Kind and Enemy Kind

**What to build:** Generalize tickets 02–04 from the hardcoded Cannon/Grunt pair to the full roster. Gatling (single-target, low damage, high fire rate, reusing the Projectile pipeline from ticket 04) and Frost (no Projectile — continuously applies a slow multiplier to every Enemy currently within Range, removed the instant an Enemy leaves Range, re-evaluated every tick rather than as a timed buff) both come online. Runner (low Health, high speed) and Tank (high Health, low speed) join Grunt as spawnable Enemy Kind. The sidebar gets working Tower Kind selection buttons (Cannon/Gatling/Frost) that control which kind the next Cell click places.

**Blocked by:** 04 (Tower attacks Enemy with Projectile)

**Status:** ready-for-agent

- [ ] Sidebar has three Tower Kind buttons; the selected one determines what the next Cell click places
- [ ] Placing a Gatling Tower fires faster, lower-damage Projectile than Cannon, using the same tracking/hit pipeline
- [ ] Placing a Frost Tower fires no Projectile; instead every Enemy within its Range moves at reduced speed while inside, and returns to normal speed the instant it steps outside — with no lingering effect
- [ ] Grunt, Runner, and Tank Enemy each have distinct Health and speed matching their described profile (Runner fastest/lowest Health, Tank slowest/highest Health)
- [ ] `simulation` unit tests cover: Gatling fire-rate/damage differ from Cannon; Frost slow applies only while an Enemy's position is within Range and clears the tick it leaves; each Enemy Kind's stats are distinct and correct
