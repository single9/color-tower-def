//! Drives the real `Simulation` engine (real ticks, real Path travel
//! time, real Tower range/cooldown) with a greedy "always buy the
//! cheapest Cannon you can afford, spread coverage across several
//! chokepoints along the Path" bot. This is the realistic counterpart
//! to a closed-form best-case model: it captures the chicken-and-egg
//! of needing kills to afford Towers, and how far a single Tower's
//! Range circle actually reaches over a moving Enemy queue — details a
//! formula comparing aggregate Wave duration to aggregate DPS misses
//! entirely.
use simulation::{CellPos, GameOutcome, GRID_SIZE, SimEvent, Simulation, TowerKind};

/// Placement order a reasonably strong player would use: several
/// chokepoints spread along the whole Path (not one cluster near
/// Spawn), each anchored so its full Range circle sits over reachable
/// Path, handed out round-robin as Gold allows so coverage extends the
/// entire Path as the economy grows — leaving no long undefended
/// stretch once later Waves need it.
fn candidate_positions() -> Vec<CellPos> {
    let mid = GRID_SIZE as f32 / 2.0;
    let anchors = [3.0f32, 9.0, 15.0, 21.0];
    let mut per_anchor: Vec<Vec<CellPos>> = anchors
        .iter()
        .map(|&ax| {
            let mut cells: Vec<CellPos> = (0..GRID_SIZE)
                .flat_map(|x| (0..GRID_SIZE).map(move |y| CellPos::new(x, y)))
                .filter(|p| p.y != GRID_SIZE / 2)
                .collect();
            cells.sort_by(|a, b| {
                let da = ((a.x as f32 - ax).powi(2) + (a.y as f32 - mid).powi(2)).sqrt();
                let db = ((b.x as f32 - ax).powi(2) + (b.y as f32 - mid).powi(2)).sqrt();
                da.partial_cmp(&db).unwrap()
            });
            cells
        })
        .collect();

    let mut out = Vec::new();
    let mut idx = vec![0usize; anchors.len()];
    loop {
        let mut any = false;
        for (a, cells) in per_anchor.iter_mut().enumerate() {
            if idx[a] < cells.len() {
                out.push(cells[idx[a]]);
                idx[a] += 1;
                any = true;
            }
        }
        if !any {
            break;
        }
    }
    out
}

/// Spends down to the cheapest Cannon affordable, buying the next
/// candidate position each time, then ticks `dt` once. Returns the
/// `SimEvent` this tick produced.
fn buy_then_tick(
    sim: &mut Simulation,
    candidates: &[CellPos],
    next_candidate: &mut usize,
    dt: f32,
) -> Vec<SimEvent> {
    while *next_candidate < candidates.len() && sim.gold() >= sim.tower_price(TowerKind::Cannon) {
        let pos = candidates[*next_candidate];
        let _ = sim.place_tower(pos, TowerKind::Cannon);
        *next_candidate += 1;
    }
    sim.tick(dt)
}

/// Under optimal play (zero wasted Gold, always buying the cheapest
/// Cannon, coverage spread across the whole Path), Wave 1 should be
/// clearable using only `STARTING_GOLD` — no Enemy should reach Goal.
/// This is the real-engine ground truth the ad hoc best-case formula
/// (aggregate Wave duration vs. aggregate DPS) got wrong: it ignored
/// that Enemy traverse the Path over time, giving even a lone,
/// well-placed Tower multiple shots per Enemy, and it ignored that
/// spreading Gold's first two (equally-priced, since
/// `fibonacci(1) == fibonacci(2) == 1`) Towers across two chokepoints
/// beats stacking both on one.
#[test]
fn optimal_play_clears_wave_one_with_no_leaks() {
    let mut sim = Simulation::new();
    let candidates = candidate_positions();
    let mut next_candidate = 0usize;
    let starting_lives = sim.lives();

    sim.start_next_wave().expect("Wave 1 should start");
    let mut ticks = 0;
    while sim.wave_in_progress() && ticks < 3000 {
        let events = buy_then_tick(&mut sim, &candidates, &mut next_candidate, 0.05);
        assert!(
            !events.contains(&SimEvent::Leak),
            "Wave 1 leaked an Enemy under optimal play"
        );
        assert_ne!(sim.outcome(), Some(GameOutcome::Defeat));
        ticks += 1;
    }

    assert!(!sim.wave_in_progress(), "Wave 1 never cleared within the time budget");
    assert_eq!(sim.wave_number(), 2, "Wave 1 should have advanced to Wave 2");
    assert_eq!(sim.lives(), starting_lives, "no Lives should have been lost clearing Wave 1");
}

/// Broader regression guard alongside the Wave 1 invariant above: the
/// same optimal-play bot should be able to carry that early-game
/// footing through every Wave on Level 1 (`TOTAL_WAVES`) without the
/// run ever stalling out (Defeat) partway through — the failure mode
/// the original closed-form model predicted for Waves 1-5. Stops at
/// `TOTAL_WAVES` rather than chasing full Victory: clearing it also
/// triggers `LevelCleared` (Level 2 of `LEVEL_COUNT`, a separate
/// meta-progression layer this test isn't about), which resets
/// `wave_number` back to 1 on a fresh map.
#[test]
fn optimal_play_survives_every_wave_on_level_one() {
    let mut sim = Simulation::new();
    let candidates = candidate_positions();
    let mut next_candidate = 0usize;

    for wave in 1..=simulation::TOTAL_WAVES {
        sim.start_next_wave()
            .unwrap_or_else(|e| panic!("Wave {wave} should start: {e:?}"));
        let mut ticks = 0;
        while sim.wave_in_progress() && ticks < 3000 {
            buy_then_tick(&mut sim, &candidates, &mut next_candidate, 0.05);
            assert_ne!(
                sim.outcome(),
                Some(GameOutcome::Defeat),
                "run should not lose all Lives before clearing every Wave"
            );
            ticks += 1;
        }
        assert!(!sim.wave_in_progress(), "Wave {wave} never cleared within the time budget");
    }

    assert_ne!(sim.outcome(), Some(GameOutcome::Defeat));
}
