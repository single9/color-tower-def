Status: ready-for-agent

# Tower Defense Core Gameplay (MVP)

## Problem Statement

The user wants a playable, self-contained maze-style tower defense game with a deliberately minimal color-block visual style, built in Rust with Bevy. Right now the project has no code at all — only the design decisions captured in `CONTEXT.md` and the two ADRs. There is no way to actually place a Tower, watch a Wave of Enemy come through, or win/lose a game.

## Solution

Build the full MVP game loop described in `CONTEXT.md`: a single fixed 25x25 Grid on which the player places Tower to block the Path of Enemy waves, across 15 Wave, ending in Victory or Defeat. All game rules live in a Bevy-independent `simulation` module (see Implementation Decisions); Bevy is a thin input/render shell around it.

## User Stories

1. As a player, I want to see a 25x25 Grid rendered as 20px color-block Cell on screen, so that I can visually parse the play area.
2. As a player, I want the Spawn Cell and Goal Cell to be visually distinguishable from Buildable Cell, so that I understand where Enemy enter and what I'm defending.
3. As a player, I want to select a Tower Kind (Cannon, Gatling, or Frost) from the sidebar before placing, so that I can choose my strategy.
4. As a player, I want to click a Buildable Cell to place the currently-selected Tower Kind there, so that I can build my defense.
5. As a player, I want tower placement to be rejected if it would leave no Path from Spawn to Goal, so that I can never accidentally seal off the map.
6. As a player, I want to see a live preview of the resulting Path before I confirm a placement, so that I can judge the effect of a Tower before committing Gold to it.
7. As a player, I want each in-flight Enemy to keep moving smoothly (no teleporting) even as I edit the maze around it, so that the game feels fair and readable.
8. As a player, I want Cannon towers to fire slow, high-damage Projectile at a single target, so that I have an option for high burst damage.
9. As a player, I want Gatling towers to fire fast, low-damage Projectile at a single target, so that I have an option for sustained single-target DPS.
10. As a player, I want Frost towers to continuously slow every Enemy inside their Range without firing Projectile, so that I have an area-control option.
11. As a player, I want a Frost tower's Slow Aura to stop affecting an Enemy the instant it leaves Range, so that the effect is predictable and non-sticky.
12. As a player, I want Projectile fired by Cannon/Gatling to track their target and always hit (barring the target dying first), so that I'm never frustrated by wasted shots.
13. As a player, I want to click an already-placed Tower to open a panel showing its Tower Kind, Tier, and stats, so that I can make informed upgrade/sell decisions.
14. As a player, I want to upgrade a selected Tower up to Tier 3, each Tier costing Gold and increasing its main stat by 30%, so that I have a mid-game power progression.
15. As a player, I want to sell a selected Tower for 70% of its total Gold spent (purchase + upgrades), so that I can rework my maze without losing all my investment.
16. As a player, I want to start with a fixed amount of Gold, so that early-game strategy is constrained.
17. As a player, I want to earn Gold when an Enemy dies, with the amount depending on Enemy Kind, so that harder kills feel more rewarding.
18. As a player, I want three Enemy Kind — Grunt (medium Health, medium speed), Runner (low Health, high speed), Tank (high Health, low speed) — so that different Tower Kind have distinct counters.
19. As a player, I want to manually trigger the start of the next Wave via a button, so that I control the pacing and can prepare my maze first.
20. As a player, I want each Wave's Enemy to spawn one at a time with a fixed interval between them, so that I can see and react to individual threats.
21. As a player, I want the number and Health of Enemy in a Wave to scale up with the Wave number, so that the game gets harder as I progress.
22. As a player, I want to lose 1 Lives every time an Enemy reaches the Goal (a Leak), so that letting enemies through has a clear, understandable cost.
23. As a player, I want to see my current Gold, Lives, and Wave number at all times in the sidebar, so that I always know my status.
24. As a player, I want to reach Victory after clearing all 15 Wave while Lives is still above zero, so that there's a clear win condition.
25. As a player, I want to reach Defeat the moment my Lives hits zero, so that there's a clear loss condition.
26. As a player, I want a result screen on Victory or Defeat with a button to reset the level and try again, so that I can immediately replay without restarting the app.
27. As a player, I want the game to open directly into the level's pre-Wave preparation state with no main menu, so that I can start playing immediately.
28. As a developer, I want the core game rules (Grid, Path, Tower, Enemy, Wave, Gold, Lives, Projectile, Slow Aura) implemented as a Bevy-independent `simulation` module, so that I can unit test the entire ruleset without booting a Bevy `App`.

## Implementation Decisions

- **Module split**: a `simulation` crate/module with zero Bevy dependency owns all domain state and rules (`Grid`, `Cell`/`CellKind`, `Tower`/`TowerKind`/`Tier`, `Enemy`/`EnemyKind`, `Wave`, `Projectile`, `Path`, `Gold`, `Lives`). It exposes a `Simulation` type with methods such as `select_tower_kind`, `place_tower(pos)`, `sell_tower(pos)`, `upgrade_tower(pos)`, `start_next_wave()`, and `tick(dt)`. Every method returns either the updated read-only state or a rejection reason (e.g. `PlacementRejected::WouldBlockPath`); `tick` additionally returns a list of events (`EnemyKilled`, `Leak`, `WaveCleared`, `Victory`, `Defeat`) for the Bevy layer to react to (spawn/despawn visuals, update sidebar).
- **Bevy layer is a thin adapter**: Bevy systems translate mouse input (cell click, sidebar button click) into `Simulation` method calls, and translate `Simulation` state/events into spawned/despawned/recolored entities and UI text updates. No game rule (damage, Path validity, Gold math, Wave scaling) lives in a Bevy system.
- **CellKind**: `Buildable`, `Spawn`, `Goal`. No fixed obstacle cells (per confirmed design). A Cell holds at most one Tower; a Cell with a Tower is not `Buildable` for further placement.
- **Path per ADR-0002**: each Enemy independently computes its own shortest remaining Path (Spawn/current-cell → Goal) via BFS over `Buildable`-or-currently-occupied-by-self cells, avoiding Cells with a Tower. Recomputation happens only when the Enemy reaches the center of its current Cell — not on every Grid mutation. The pre-placement Path preview (User Story 6) runs the same BFS from the Spawn Cell for display purposes only; it does not mutate any Enemy's stored Path.
- **Blocking Rule**: before committing a `place_tower` call, run BFS from Spawn to Goal treating the candidate Cell as occupied; reject placement if no Path exists. This is the single source of truth also used for the placement preview.
- **Tower Kind stats**: Cannon (single-target, high damage, low fire rate), Gatling (single-target, low damage, high fire rate), Frost (no Projectile; applies a flat slow multiplier to Enemy speed while they are within Range, removed the instant they leave Range — re-evaluated every tick, not a timed buff).
- **Projectile per ADR-0001**: no physics engine. A Projectile stores a target `Enemy` id and moves toward that Enemy's current position each tick at a fixed speed; a hit is registered when the distance to the target falls under a small threshold, at which point it applies its damage and despawns. If the target dies before the Projectile arrives, the Projectile despawns without effect.
- **Tier system**: Tier 1 (base) → Tier 2 → Tier 3. Each upgrade increases the Tower's primary stat (damage for Cannon/Gatling, slow strength or Range for Frost — exact stat TBD by implementer, document the choice in `CONTEXT.md` once decided) by 30% over the previous Tier. Upgrade cost at each step is 80% of the Tower's original purchase price.
- **Economy**: fixed starting Gold (exact number left to implementer/playtesting — record the chosen value as a constant, not a magic number scattered across code). Tower purchase price is fixed per Tower Kind. Selling refunds 70% of total Gold spent on that Tower (purchase price plus any upgrade costs paid).
- **Wave scaling**: 15 Wave total. Wave *n* spawns `5 + n` Enemy. Each Enemy's Health for Wave *n* is `base_health * (1 + n * 0.1)`. Enemies within a Wave spawn one at a time at the Spawn Cell, 0.8s apart.
- **Win/lose**: Victory triggers when Wave 15 is cleared (all its Enemy are dead) and Lives > 0. Defeat triggers the instant Lives reaches 0, even mid-Wave.
- **Screen layout**: 700x500 window — 500x500 Grid area on the left, 200px sidebar on the right showing Gold, Lives, Wave number, Tower Kind selection buttons, and (when a Tower is selected) its info/upgrade/sell panel.
- **Visual palette**: Grid uses a neutral palette distinct from Tower colors — Buildable = light gray, Spawn = dark purple, Goal = orange, Path preview overlay = translucent yellow. Tower Kind colors: Cannon = red, Gatling = green, Frost = blue.
- **No main menu, no pause, no audio** — confirmed explicitly out of scope for this MVP (see Out of Scope).

## Testing Decisions

- All tests target the `simulation` module directly: construct a `Simulation` (optionally with a fixed RNG/seed if any randomness is introduced later — none is needed for this spec), call its methods, and assert on the returned state/events. No Bevy `App`, no rendering, no window needed for any of these tests.
- Test only externally observable behavior of `Simulation` (state after a call, events emitted, rejection reasons) — not internal representations like the exact BFS implementation, as long as Path correctness holds.
- Priority areas for coverage, since they encode the trickiest rules in this spec:
  - Blocking Rule rejection (placement that would seal the maze must be rejected; placement that leaves any Path must succeed)
  - Per-Enemy Path recalculation timing (an Enemy mid-Cell keeps its old Path; an Enemy that just reached a Cell center picks up Grid changes)
  - Wave scaling formula (Enemy count and Health for a given Wave number)
  - Gold math (purchase, upgrade cost at each Tier, 70% sell refund of total spend)
  - Win/lose transitions (Victory only after Wave 15 clear with Lives > 0; Defeat fires immediately on Lives hitting 0 even mid-Wave)
  - Frost Slow Aura applying/clearing based on live Range membership each tick, not a timer
- There is no prior art in this repo — it's a greenfield project — so these tests establish the convention: plain `#[test]` functions in the `simulation` module, constructing state directly with no mocking (there's no I/O to mock).
- The Bevy adapter layer (input → `Simulation` calls, `Simulation` state/events → rendering) is not unit tested in this spec; verify it manually by running the game via `/run` and walking through placement, a full Wave, upgrade/sell, a Leak, and both Victory and Defeat.

## Out of Scope

- Multiple maps or map/level selection
- A main menu / title screen
- Pause functionality
- Sound effects and music
- Any Tower Kind beyond Cannon/Gatling/Frost, or Enemy Kind beyond Grunt/Runner/Tank
- Upgrades beyond Tier 3
- Keyboard shortcuts for Tower Kind selection (mouse/sidebar buttons only)
- Multiple Spawn or Goal Cells
- Fixed map obstacles (Cells that are neither Buildable, Spawn, nor Goal)
- Save/load, persistence of any kind across app restarts
- Multiplayer, networking
- Touch/gamepad input

## Further Notes

- Domain vocabulary throughout this spec follows `CONTEXT.md` exactly (Grid, Cell, Spawn, Goal, Path, Blocking Rule, Tower, Tower Kind, Range, Projectile, Slow Aura, Tier, Enemy, Enemy Kind, Health, Wave, Leak, Lives, Gold, Victory, Defeat) — implementers should keep code identifiers aligned to these terms rather than inventing synonyms.
- Respects ADR-0001 (no physics engine — self-authored distance-based Projectile collision) and ADR-0002 (per-Enemy Path recalculation at Cell-center boundaries, not a global instant resync).
- Exact numeric constants left as implementer/playtesting decisions in this spec (starting Gold, per-Tower-Kind purchase prices, base damage/fire-rate/Range/speed/Health values, Projectile speed, hit-distance threshold, Frost slow multiplier): pick reasonable values, define them as named constants in one place, and don't scatter magic numbers through the `simulation` module.
