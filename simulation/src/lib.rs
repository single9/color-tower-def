//! Bevy-independent core game rules for the tower defense game.

use std::collections::{HashMap, HashSet, VecDeque};

pub const GRID_SIZE: usize = 25;
pub const CELL_SIZE_PX: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPos {
    pub x: usize,
    pub y: usize,
}

impl CellPos {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Buildable,
    Spawn,
    Goal,
}

#[derive(Debug, Clone)]
pub struct Grid {
    spawn: CellPos,
    goal: CellPos,
}

impl Grid {
    /// A single fixed 25x25 map: one Spawn Cell on the left edge, one
    /// Goal Cell on the right edge, everything else Buildable.
    pub fn new() -> Self {
        Self {
            spawn: CellPos::new(0, GRID_SIZE / 2),
            goal: CellPos::new(GRID_SIZE - 1, GRID_SIZE / 2),
        }
    }

    pub fn spawn(&self) -> CellPos {
        self.spawn
    }

    pub fn goal(&self) -> CellPos {
        self.goal
    }

    pub fn kind_at(&self, pos: CellPos) -> CellKind {
        if pos == self.spawn {
            CellKind::Spawn
        } else if pos == self.goal {
            CellKind::Goal
        } else {
            CellKind::Buildable
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = CellPos> {
        (0..GRID_SIZE).flat_map(|y| (0..GRID_SIZE).map(move |x| CellPos::new(x, y)))
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

fn neighbors(pos: CellPos) -> impl Iterator<Item = CellPos> {
    let mut out = Vec::with_capacity(4);
    if pos.x > 0 {
        out.push(CellPos::new(pos.x - 1, pos.y));
    }
    if pos.x + 1 < GRID_SIZE {
        out.push(CellPos::new(pos.x + 1, pos.y));
    }
    if pos.y > 0 {
        out.push(CellPos::new(pos.x, pos.y - 1));
    }
    if pos.y + 1 < GRID_SIZE {
        out.push(CellPos::new(pos.x, pos.y + 1));
    }
    out.into_iter()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    /// The target Cell is Spawn, Goal, or otherwise not Buildable.
    NotBuildable,
    /// The target Cell already has a Tower on it.
    AlreadyOccupied,
    /// The player does not have enough Gold for this Tower Kind's price.
    InsufficientGold,
    /// Placing here would leave no Path from Spawn to Goal (the Blocking Rule).
    WouldBlockPath,
}

/// Balance numbers pending playtesting. Tier-scaling (ticket 07) and
/// Wave-scaling (ticket 08) land later; these are the base values.
const GRUNT_HEALTH: f32 = 100.0;
const GRUNT_SPEED_CELLS_PER_SEC: f32 = 2.0;
const RUNNER_HEALTH: f32 = 50.0;
const RUNNER_SPEED_CELLS_PER_SEC: f32 = 3.5;
const TANK_HEALTH: f32 = 220.0;
const TANK_SPEED_CELLS_PER_SEC: f32 = 1.0;

const CANNON_DAMAGE: f32 = 50.0;
const CANNON_RANGE_CELLS: f32 = 5.0;
const CANNON_COOLDOWN_SECONDS: f32 = 1.0;
const GATLING_DAMAGE: f32 = 15.0;
const GATLING_RANGE_CELLS: f32 = 4.0;
const GATLING_COOLDOWN_SECONDS: f32 = 0.25;
const FROST_RANGE_CELLS: f32 = 3.5;
/// Fraction of normal speed an Enemy moves at while inside a Frost
/// Tower's Range (re-evaluated every tick; no lingering effect).
const FROST_SLOW_MULTIPLIER: f32 = 0.5;

const PROJECTILE_SPEED_CELLS_PER_SEC: f32 = 8.0;
const PROJECTILE_HIT_DISTANCE_CELLS: f32 = 0.25;

/// Gold economy balance numbers pending playtesting.
const STARTING_GOLD: i32 = 200;
const CANNON_PRICE: i32 = 100;
const GATLING_PRICE: i32 = 80;
const FROST_PRICE: i32 = 120;
const GRUNT_GOLD_REWARD: i32 = 10;
const RUNNER_GOLD_REWARD: i32 = 6;
const TANK_GOLD_REWARD: i32 = 20;
/// Fraction of the Gold spent on a Tower refunded on sale.
const SELL_REFUND_FRACTION: f32 = 0.7;

/// The three Enemy Kind, each with distinct Health/speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Grunt,
    Runner,
    Tank,
}

impl EnemyKind {
    fn health(self) -> f32 {
        match self {
            EnemyKind::Grunt => GRUNT_HEALTH,
            EnemyKind::Runner => RUNNER_HEALTH,
            EnemyKind::Tank => TANK_HEALTH,
        }
    }

    fn speed(self) -> f32 {
        match self {
            EnemyKind::Grunt => GRUNT_SPEED_CELLS_PER_SEC,
            EnemyKind::Runner => RUNNER_SPEED_CELLS_PER_SEC,
            EnemyKind::Tank => TANK_SPEED_CELLS_PER_SEC,
        }
    }

    /// Gold granted to the player for killing this Enemy Kind.
    fn gold_reward(self) -> i32 {
        match self {
            EnemyKind::Grunt => GRUNT_GOLD_REWARD,
            EnemyKind::Runner => RUNNER_GOLD_REWARD,
            EnemyKind::Tank => TANK_GOLD_REWARD,
        }
    }
}

/// The three Tower Kind. Cannon and Gatling both fire tracking
/// Projectiles (ADR-0001) at different damage/fire-rate tradeoffs;
/// Frost fires none, instead continuously slowing every Enemy inside
/// its Range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerKind {
    Cannon,
    Gatling,
    Frost,
}

impl TowerKind {
    fn damage(self) -> f32 {
        match self {
            TowerKind::Cannon => CANNON_DAMAGE,
            TowerKind::Gatling => GATLING_DAMAGE,
            TowerKind::Frost => 0.0,
        }
    }

    fn range(self) -> f32 {
        match self {
            TowerKind::Cannon => CANNON_RANGE_CELLS,
            TowerKind::Gatling => GATLING_RANGE_CELLS,
            TowerKind::Frost => FROST_RANGE_CELLS,
        }
    }

    fn cooldown(self) -> f32 {
        match self {
            TowerKind::Cannon => CANNON_COOLDOWN_SECONDS,
            TowerKind::Gatling => GATLING_COOLDOWN_SECONDS,
            TowerKind::Frost => 0.0,
        }
    }

    fn fires_projectiles(self) -> bool {
        !matches!(self, TowerKind::Frost)
    }

    /// Gold cost to place a fresh Tower of this Kind.
    fn price(self) -> i32 {
        match self {
            TowerKind::Cannon => CANNON_PRICE,
            TowerKind::Gatling => GATLING_PRICE,
            TowerKind::Frost => FROST_PRICE,
        }
    }
}

/// A single Enemy in transit between two Cell centers. `target` is
/// `None` only in the rare case its onward Path vanished entirely
/// (see `Simulation::tick_enemy_movement`).
#[derive(Debug, Clone, Copy)]
struct Enemy {
    at: CellPos,
    target: Option<CellPos>,
    progress: f32,
    health: f32,
    kind: EnemyKind,
}

/// Per-Tower runtime state. Tier lands in ticket 07 and will fold
/// upgrade spend into `gold_spent` for the sell refund.
#[derive(Debug, Clone, Copy)]
struct TowerRuntime {
    kind: TowerKind,
    cooldown_remaining: f32,
    gold_spent: i32,
}

/// A shot in flight, tracking the live Enemy's position every tick
/// (ADR-0001: plain distance check, no physics engine). Position is
/// in Cell units, not pixels — the Bevy layer converts. `damage`
/// carries the firing Tower Kind's damage so Gatling and Cannon
/// shots resolve differently on hit.
#[derive(Debug, Clone, Copy)]
struct Projectile {
    pos: (f32, f32),
    damage: f32,
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// Where an Enemy currently is, for the Bevy layer to render: it sits
/// somewhere between `from` and `to`, `progress` of the way there
/// (0.0 = at `from`'s center, 1.0 = at `to`'s center).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyTransit {
    pub from: CellPos,
    pub to: CellPos,
    pub progress: f32,
}

/// Owns the Grid and every placed Tower, and enforces the Blocking Rule.
///
/// Ticket 02 only needs a single Tower Kind, so a Tower is represented
/// as bare occupancy (a `CellPos` with nothing else attached) for now;
/// Tower Kind/Tier land in later tickets.
#[derive(Debug, Clone)]
pub struct Simulation {
    grid: Grid,
    towers: HashMap<CellPos, TowerRuntime>,
    enemy: Option<Enemy>,
    projectiles: Vec<Projectile>,
    gold: i32,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            grid: Grid::new(),
            towers: HashMap::new(),
            enemy: None,
            projectiles: Vec::new(),
            gold: STARTING_GOLD,
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn gold(&self) -> i32 {
        self.gold
    }

    pub fn has_tower(&self, pos: CellPos) -> bool {
        self.towers.contains_key(&pos)
    }

    /// The Tower Kind placed at `pos`, if any — for the Bevy layer to
    /// pick a sprite color.
    pub fn tower_kind_at(&self, pos: CellPos) -> Option<TowerKind> {
        self.towers.get(&pos).map(|runtime| runtime.kind)
    }

    /// The current shortest Path from Spawn to Goal around all placed
    /// Tower, or `None` if none exists.
    pub fn current_path(&self) -> Option<Vec<CellPos>> {
        self.shortest_path(self.grid.spawn(), None)
    }

    /// What the shortest Path from Spawn to Goal would be if a Tower
    /// were additionally placed at `pos`. Used for the pre-placement
    /// preview; does not mutate any state.
    pub fn preview_path_if_placed(&self, pos: CellPos) -> Option<Vec<CellPos>> {
        self.shortest_path(self.grid.spawn(), Some(pos))
    }

    /// Whether a Tower of the given Kind could be placed at `pos` right
    /// now, and if not, why.
    pub fn can_place(&self, pos: CellPos, kind: TowerKind) -> Result<(), PlacementError> {
        if self.grid.kind_at(pos) != CellKind::Buildable {
            return Err(PlacementError::NotBuildable);
        }
        if self.towers.contains_key(&pos) {
            return Err(PlacementError::AlreadyOccupied);
        }
        if self.gold < kind.price() {
            return Err(PlacementError::InsufficientGold);
        }
        if self.shortest_path(self.grid.spawn(), Some(pos)).is_none() {
            return Err(PlacementError::WouldBlockPath);
        }
        Ok(())
    }

    pub fn place_tower(&mut self, pos: CellPos, kind: TowerKind) -> Result<(), PlacementError> {
        self.can_place(pos, kind)?;
        let price = kind.price();
        self.gold -= price;
        self.towers.insert(
            pos,
            TowerRuntime {
                kind,
                cooldown_remaining: 0.0,
                gold_spent: price,
            },
        );
        Ok(())
    }

    /// Removes the Tower at `pos`, if any, refunding `SELL_REFUND_FRACTION`
    /// of the Gold spent on it (purchase price only for now; upgrade
    /// spend folds in from ticket 07). The refund rounds to the
    /// nearest whole Gold, .5 rounding away from zero. Returns whether
    /// a Tower was there.
    pub fn sell_tower(&mut self, pos: CellPos) -> bool {
        let Some(runtime) = self.towers.remove(&pos) else {
            return false;
        };
        self.gold += (runtime.gold_spent as f32 * SELL_REFUND_FRACTION).round() as i32;
        true
    }

    /// Spawns one Enemy of the given Kind at Spawn, replacing any
    /// Enemy already present. Ticket 05 only needs a single Enemy on
    /// screen; Wave spawning of many at once lands in ticket 08.
    pub fn spawn_enemy(&mut self, kind: EnemyKind) {
        let spawn = self.grid.spawn();
        let path = self.shortest_path(spawn, None);
        self.enemy = Some(Enemy {
            at: spawn,
            target: path.and_then(|p| p.get(1).copied()),
            progress: 0.0,
            health: kind.health(),
            kind,
        });
    }

    pub fn enemy_alive(&self) -> bool {
        self.enemy.is_some()
    }

    pub fn enemy_health(&self) -> Option<f32> {
        self.enemy.as_ref().map(|e| e.health)
    }

    pub fn projectile_count(&self) -> usize {
        self.projectiles.len()
    }

    /// Every Projectile's current position, in Cell units, for the
    /// Bevy layer to render.
    pub fn projectile_positions(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        self.projectiles.iter().map(|p| p.pos)
    }

    /// The live Enemy's current transit segment (where it's coming
    /// from, where it's headed, how far along it is), for the Bevy
    /// layer to interpolate a world position from. `None` once the
    /// Enemy has reached Goal and despawned.
    pub fn enemy_transit(&self) -> Option<EnemyTransit> {
        self.enemy.as_ref().map(|enemy| EnemyTransit {
            from: enemy.at,
            to: enemy.target.unwrap_or(enemy.at),
            progress: if enemy.target.is_some() { enemy.progress } else { 0.0 },
        })
    }

    /// Advances the whole Simulation by `dt` seconds: Enemy movement
    /// (at whatever speed the current Frost coverage allows), then
    /// Tower firing, then Projectile flight/impact.
    pub fn tick(&mut self, dt: f32) {
        let slow_multiplier = self.frost_slow_multiplier();
        self.tick_enemy_movement(dt, slow_multiplier);
        self.tick_towers(dt);
        self.tick_projectiles(dt);
    }

    /// Whether the live Enemy's current position falls within any
    /// Frost Tower's Range right now. Re-evaluated fresh every tick —
    /// no lingering effect once the Enemy steps back outside.
    fn frost_slow_multiplier(&self) -> f32 {
        let Some(enemy_pos) = self.enemy_position_cells() else {
            return 1.0;
        };
        let in_frost = self.towers.iter().any(|(pos, runtime)| {
            runtime.kind == TowerKind::Frost
                && distance((pos.x as f32, pos.y as f32), enemy_pos) <= runtime.kind.range()
        });
        if in_frost {
            FROST_SLOW_MULTIPLIER
        } else {
            1.0
        }
    }

    /// Advances the live Enemy by `dt` seconds at its Kind's base
    /// speed times `speed_multiplier` (Frost slow). Per ADR-0002, the
    /// Enemy's Path is only ever recomputed the instant it reaches a
    /// Cell center — never mid-transit, no matter how the Grid changes
    /// underneath it in the meantime.
    fn tick_enemy_movement(&mut self, dt: f32, speed_multiplier: f32) {
        let Some((at, target, mut progress, kind)) = self
            .enemy
            .as_ref()
            .map(|enemy| (enemy.at, enemy.target, enemy.progress, enemy.kind))
        else {
            return;
        };

        let Some(target) = target else {
            // Stuck at a Cell whose onward Path vanished; try again
            // every tick in case the Grid opens back up.
            let new_target = self.shortest_path(at, None).and_then(|p| p.get(1).copied());
            self.enemy.as_mut().unwrap().target = new_target;
            return;
        };

        progress += dt * kind.speed() * speed_multiplier;
        if progress < 1.0 {
            self.enemy.as_mut().unwrap().progress = progress;
            return;
        }

        if target == self.grid.goal() {
            self.enemy = None;
            return;
        }

        // Just reached a Cell center: recompute the remaining Path
        // from here, picking up whatever the Grid looks like *now*.
        let new_target = self.shortest_path(target, None).and_then(|p| p.get(1).copied());
        let enemy = self.enemy.as_mut().unwrap();
        enemy.at = target;
        enemy.progress = 0.0;
        enemy.target = new_target;
    }

    /// Where the live Enemy currently sits, in Cell units, interpolated
    /// between its current transit segment's two Cell centers.
    fn enemy_position_cells(&self) -> Option<(f32, f32)> {
        self.enemy_transit().map(|t| {
            let (fx, fy) = (t.from.x as f32, t.from.y as f32);
            let (tx, ty) = (t.to.x as f32, t.to.y as f32);
            (fx + (tx - fx) * t.progress, fy + (ty - fy) * t.progress)
        })
    }

    /// Ticks down every projectile-firing Tower's cooldown and fires a
    /// Projectile from any that's ready and has the Enemy within
    /// Range. Frost Towers never fire — their slow is applied directly
    /// in `tick`, not through this pipeline.
    fn tick_towers(&mut self, dt: f32) {
        let Some(enemy_pos) = self.enemy_position_cells() else {
            return;
        };

        let positions: Vec<CellPos> = self.towers.keys().copied().collect();
        for pos in positions {
            let runtime = self.towers.get_mut(&pos).unwrap();
            if !runtime.kind.fires_projectiles() {
                continue;
            }
            runtime.cooldown_remaining -= dt;
            if runtime.cooldown_remaining > 0.0 {
                continue;
            }

            let tower_pos = (pos.x as f32, pos.y as f32);
            if distance(tower_pos, enemy_pos) <= runtime.kind.range() {
                runtime.cooldown_remaining = runtime.kind.cooldown();
                self.projectiles.push(Projectile {
                    pos: tower_pos,
                    damage: runtime.kind.damage(),
                });
            }
        }
    }

    /// Moves every Projectile toward the live Enemy's current position
    /// and resolves hits (ADR-0001: plain distance check). A
    /// Projectile whose target has died — from this hit or an earlier
    /// one this same tick — despawns without effect.
    fn tick_projectiles(&mut self, dt: f32) {
        let Some(enemy_pos) = self.enemy_position_cells() else {
            self.projectiles.clear();
            return;
        };

        let in_flight = std::mem::take(&mut self.projectiles);
        let mut remaining = Vec::with_capacity(in_flight.len());
        for mut projectile in in_flight {
            if self.enemy.is_none() {
                continue;
            }

            if distance(projectile.pos, enemy_pos) <= PROJECTILE_HIT_DISTANCE_CELLS {
                if let Some(enemy) = self.enemy.as_mut() {
                    enemy.health -= projectile.damage;
                    if enemy.health <= 0.0 {
                        self.gold += enemy.kind.gold_reward();
                        self.enemy = None;
                    }
                }
                continue;
            }

            let dx = enemy_pos.0 - projectile.pos.0;
            let dy = enemy_pos.1 - projectile.pos.1;
            let dist = (dx * dx + dy * dy).sqrt();
            let step = PROJECTILE_SPEED_CELLS_PER_SEC * dt;
            if dist > f32::EPSILON {
                projectile.pos.0 += dx / dist * step;
                projectile.pos.1 += dy / dist * step;
            }
            remaining.push(projectile);
        }
        self.projectiles = remaining;
    }

    /// BFS from `from` to Goal, treating every placed Tower — plus
    /// `extra_blocked`, if given — as impassable. `from` need not be
    /// Spawn: each Enemy recomputes its own remaining Path from
    /// wherever it currently stands (see ADR-0002).
    fn shortest_path(&self, from: CellPos, extra_blocked: Option<CellPos>) -> Option<Vec<CellPos>> {
        let goal = self.grid.goal();
        let is_blocked = |pos: CellPos| Some(pos) == extra_blocked || self.towers.contains_key(&pos);

        if is_blocked(from) || is_blocked(goal) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut came_from: HashMap<CellPos, CellPos> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if current == goal {
                let mut path = vec![current];
                let mut node = current;
                while let Some(&prev) = came_from.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(path);
            }
            for next in neighbors(current) {
                if visited.contains(&next) || is_blocked(next) {
                    continue;
                }
                visited.insert(next);
                came_from.insert(next, current);
                queue.push_back(next);
            }
        }

        None
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_has_exactly_one_spawn_and_one_goal() {
        let grid = Grid::new();
        let spawn_count = grid
            .cells()
            .filter(|&pos| grid.kind_at(pos) == CellKind::Spawn)
            .count();
        let goal_count = grid
            .cells()
            .filter(|&pos| grid.kind_at(pos) == CellKind::Goal)
            .count();
        assert_eq!(spawn_count, 1);
        assert_eq!(goal_count, 1);
    }

    #[test]
    fn grid_has_625_cells_total() {
        let grid = Grid::new();
        assert_eq!(grid.cells().count(), GRID_SIZE * GRID_SIZE);
    }

    #[test]
    fn every_non_spawn_non_goal_cell_is_buildable() {
        let grid = Grid::new();
        for pos in grid.cells() {
            if pos != grid.spawn() && pos != grid.goal() {
                assert_eq!(grid.kind_at(pos), CellKind::Buildable);
            }
        }
    }

    #[test]
    fn placing_on_an_uncritical_cell_succeeds() {
        let mut sim = Simulation::new();
        assert!(sim.place_tower(CellPos::new(5, 5), TowerKind::Cannon).is_ok());
        assert!(sim.has_tower(CellPos::new(5, 5)));
    }

    #[test]
    fn placing_on_spawn_or_goal_is_rejected() {
        let mut sim = Simulation::new();
        assert_eq!(
            sim.place_tower(sim.grid().spawn(), TowerKind::Cannon),
            Err(PlacementError::NotBuildable)
        );
        assert_eq!(
            sim.place_tower(sim.grid().goal(), TowerKind::Cannon),
            Err(PlacementError::NotBuildable)
        );
    }

    #[test]
    fn placing_on_an_already_occupied_cell_is_rejected() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        assert_eq!(
            sim.place_tower(pos, TowerKind::Cannon),
            Err(PlacementError::AlreadyOccupied)
        );
    }

    #[test]
    fn sealing_the_entire_maze_is_rejected_but_a_single_gap_stays_valid() {
        let mut sim = Simulation::new();
        // This test is about the Blocking Rule, not the Gold economy:
        // give it enough Gold to place two dozen Towers regardless of price.
        sim.gold = 100_000;

        // Wall off the whole column x=1 except one gap at y=24: Spawn
        // (x=0) can only reach the rest of the grid through column 1.
        for y in 0..GRID_SIZE - 1 {
            sim.place_tower(CellPos::new(1, y), TowerKind::Cannon)
                .expect("leaving a gap open should keep placement valid");
        }

        // A narrow path through the single remaining gap must still exist.
        let path = sim.current_path().expect("a narrow path should remain");
        assert!(path.contains(&CellPos::new(1, GRID_SIZE - 1)));

        // Sealing the last gap would cut Spawn off from Goal entirely.
        let last_gap = CellPos::new(1, GRID_SIZE - 1);
        assert_eq!(
            sim.place_tower(last_gap, TowerKind::Cannon),
            Err(PlacementError::WouldBlockPath)
        );
        assert!(!sim.has_tower(last_gap));
    }

    #[test]
    fn selling_a_tower_frees_the_cell() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();

        assert!(sim.sell_tower(pos));
        assert!(!sim.has_tower(pos));

        // Selling an empty cell is a no-op that reports nothing was there.
        assert!(!sim.sell_tower(pos));
    }

    #[test]
    fn enemy_spawns_at_spawn_heading_toward_goal() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Grunt);
        let transit = sim.enemy_transit().expect("Enemy should be alive right after spawning");
        assert_eq!(transit.from, sim.grid().spawn());
        // With nothing blocking, Spawn (0,12) -> Goal (24,12) is a
        // straight row, so the first step is (1,12).
        assert_eq!(transit.to, CellPos::new(1, sim.grid().spawn().y));
        assert_eq!(transit.progress, 0.0);
    }

    #[test]
    fn enemy_mid_cell_keeps_its_stored_path_across_a_grid_mutation() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Grunt);

        // Advance partway into the first Cell — not far enough to reach its center.
        sim.tick(0.1);
        let before = sim.enemy_transit().unwrap();
        assert_eq!(before.to, CellPos::new(1, 12));
        assert!(before.progress > 0.0 && before.progress < 1.0);

        // A Grid mutation happening mid-transit must not retarget the Enemy.
        sim.place_tower(CellPos::new(5, 12), TowerKind::Cannon)
            .expect("placing off to the side of Spawn should stay legal");
        let after = sim.enemy_transit().unwrap();
        assert_eq!(after.to, before.to);
        assert_eq!(after.progress, before.progress);
    }

    #[test]
    fn enemy_recomputes_and_picks_up_a_changed_grid_on_reaching_a_cell_center() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Grunt);

        // Cross the full first Cell: Spawn -> (1,12), recomputing there
        // onto the still-open straight row, so target becomes (2,12).
        sim.tick(1.0 / EnemyKind::Grunt.speed());
        let at_first_center = sim.enemy_transit().unwrap();
        assert_eq!(at_first_center.from, CellPos::new(1, 12));
        assert_eq!(at_first_center.to, CellPos::new(2, 12));

        // Block the cell the Enemy was about to walk into next, *after*
        // it already committed to heading toward (2,12).
        sim.place_tower(CellPos::new(3, 12), TowerKind::Cannon)
            .expect("blocking one cell should still leave a detour");

        // Cross into (2,12): this is the recompute point.
        sim.tick(1.0 / EnemyKind::Grunt.speed());
        let at_second_center = sim.enemy_transit().unwrap();
        assert_eq!(at_second_center.from, CellPos::new(2, 12));
        assert_ne!(
            at_second_center.to,
            CellPos::new(3, 12),
            "recompute at the Cell center should route around the new Tower"
        );
    }

    #[test]
    fn enemy_reaching_goal_despawns() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Grunt);

        let steps_to_goal = sim.grid().goal().x - sim.grid().spawn().x;
        for _ in 0..steps_to_goal {
            sim.tick(1.0 / EnemyKind::Grunt.speed());
        }

        assert!(!sim.enemy_alive());
        assert!(sim.enemy_transit().is_none());
    }

    /// Places a Cannon one Cell above Spawn (never on the straight
    /// row-12 Path, so it never diverts the Enemy) and within Range of
    /// it from the very first tick, so hit timing doesn't depend on
    /// how far the Enemy has walked.
    fn sim_with_enemy_and_adjacent_tower() -> Simulation {
        let mut sim = Simulation::new();
        let spawn = sim.grid().spawn();
        sim.place_tower(CellPos::new(spawn.x, spawn.y + 1), TowerKind::Cannon)
            .expect("a Tower one Cell above Spawn should be a legal, non-blocking placement");
        sim.spawn_enemy(EnemyKind::Grunt);
        sim
    }

    #[test]
    fn a_hit_applies_damage_and_removes_the_projectile() {
        let mut sim = sim_with_enemy_and_adjacent_tower();

        let mut hit = false;
        for _ in 0..200 {
            sim.tick(0.01);
            if sim.enemy_health() != Some(GRUNT_HEALTH) {
                hit = true;
                break;
            }
        }

        assert!(hit, "the Cannon should land a hit well within 2 seconds");
        assert_eq!(sim.enemy_health(), Some(GRUNT_HEALTH - CANNON_DAMAGE));
        assert_eq!(
            sim.projectile_count(),
            0,
            "the Projectile that just hit should be gone"
        );
    }

    #[test]
    fn health_reaching_zero_kills_the_enemy() {
        let mut sim = sim_with_enemy_and_adjacent_tower();

        for _ in 0..300 {
            sim.tick(0.01);
            if !sim.enemy_alive() {
                break;
            }
        }

        assert!(
            !sim.enemy_alive(),
            "two Cannon hits (100 Health, 50 damage each) should kill the Grunt"
        );
        assert!(sim.enemy_transit().is_none());
    }

    #[test]
    fn a_projectile_targeting_an_already_dead_enemy_is_a_no_op() {
        let mut sim = sim_with_enemy_and_adjacent_tower();

        // One small tick: the Cannon (cooldown starts at 0, Enemy
        // already in Range) fires, but the Projectile hasn't arrived yet.
        sim.tick(0.01);
        assert_eq!(sim.projectile_count(), 1);

        // The Enemy dies from something else entirely before the
        // in-flight Projectile reaches it.
        sim.enemy = None;

        // Advancing further must not panic, must clear the now-orphaned
        // Projectile, and must not resurrect or otherwise affect anything.
        sim.tick(0.5);
        assert_eq!(sim.projectile_count(), 0);
        assert!(!sim.enemy_alive());
    }

    #[test]
    fn gatling_fires_faster_and_weaker_than_cannon() {
        assert!(TowerKind::Gatling.damage() < TowerKind::Cannon.damage());
        assert!(TowerKind::Gatling.cooldown() < TowerKind::Cannon.cooldown());
    }

    #[test]
    fn each_enemy_kind_has_distinct_correct_stats() {
        assert!(EnemyKind::Runner.health() < EnemyKind::Grunt.health());
        assert!(EnemyKind::Grunt.health() < EnemyKind::Tank.health());
        assert!(EnemyKind::Runner.speed() > EnemyKind::Grunt.speed());
        assert!(EnemyKind::Grunt.speed() > EnemyKind::Tank.speed());
    }

    #[test]
    fn spawn_enemy_uses_the_given_kinds_health() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Tank);
        assert_eq!(sim.enemy_health(), Some(EnemyKind::Tank.health()));
    }

    #[test]
    fn frost_slow_applies_only_within_range_and_clears_the_moment_it_leaves() {
        let mut sim = Simulation::new();
        let spawn = sim.grid().spawn();
        sim.place_tower(CellPos::new(spawn.x, spawn.y + 1), TowerKind::Frost)
            .expect("a Frost Tower one Cell above Spawn should be a legal, non-blocking placement");
        sim.spawn_enemy(EnemyKind::Grunt);

        // (3,12) is within the Frost Tower's Range: distance to (0,13)
        // is sqrt(3^2 + 1^2) ~= 3.16, under FROST_RANGE_CELLS (3.5).
        {
            let enemy = sim.enemy.as_mut().unwrap();
            enemy.at = CellPos::new(3, 12);
            enemy.target = Some(CellPos::new(4, 12));
            enemy.progress = 0.0;
        }
        assert_eq!(sim.frost_slow_multiplier(), FROST_SLOW_MULTIPLIER);

        // One Cell further out, (4,12), is just outside Range: distance
        // to (0,13) is sqrt(4^2 + 1^2) ~= 4.12, over FROST_RANGE_CELLS.
        sim.enemy.as_mut().unwrap().at = CellPos::new(4, 12);
        assert_eq!(sim.frost_slow_multiplier(), 1.0);
    }

    #[test]
    fn an_enemy_inside_frost_range_covers_less_ground_per_tick() {
        let mut sim = Simulation::new();
        let spawn = sim.grid().spawn();
        sim.place_tower(CellPos::new(spawn.x, spawn.y + 1), TowerKind::Frost)
            .expect("a Frost Tower one Cell above Spawn should be a legal, non-blocking placement");
        sim.spawn_enemy(EnemyKind::Grunt);

        sim.tick(0.1);
        let slowed_progress = sim.enemy_transit().unwrap().progress;
        let unslowed_progress = 0.1 * EnemyKind::Grunt.speed();
        assert!(
            slowed_progress < unslowed_progress - f32::EPSILON,
            "an Enemy within Frost Range should move slower than its base speed"
        );
    }

    #[test]
    fn affordable_placement_deducts_the_towers_price() {
        let mut sim = Simulation::new();
        let starting_gold = sim.gold();

        sim.place_tower(CellPos::new(5, 5), TowerKind::Cannon)
            .expect("an uncritical Cell should be a legal placement");

        assert_eq!(sim.gold(), starting_gold - TowerKind::Cannon.price());
    }

    #[test]
    fn unaffordable_placement_is_rejected_and_gold_is_unchanged() {
        let mut sim = Simulation::new();
        sim.gold = TowerKind::Cannon.price() - 1;

        assert_eq!(
            sim.place_tower(CellPos::new(5, 5), TowerKind::Cannon),
            Err(PlacementError::InsufficientGold)
        );
        assert_eq!(sim.gold(), TowerKind::Cannon.price() - 1);
        assert!(!sim.has_tower(CellPos::new(5, 5)));
    }

    #[test]
    fn each_enemy_kind_grants_its_own_distinct_kill_reward() {
        assert_ne!(EnemyKind::Grunt.gold_reward(), EnemyKind::Runner.gold_reward());
        assert_ne!(EnemyKind::Grunt.gold_reward(), EnemyKind::Tank.gold_reward());
        assert_ne!(EnemyKind::Runner.gold_reward(), EnemyKind::Tank.gold_reward());
    }

    #[test]
    fn killing_an_enemy_grants_its_kill_reward() {
        let mut sim = sim_with_enemy_and_adjacent_tower();
        let gold_before = sim.gold();

        for _ in 0..300 {
            sim.tick(0.01);
            if !sim.enemy_alive() {
                break;
            }
        }

        assert!(!sim.enemy_alive());
        assert_eq!(sim.gold(), gold_before + EnemyKind::Grunt.gold_reward());
    }

    #[test]
    fn selling_a_tower_with_no_upgrades_refunds_seventy_percent_of_its_price_rounded() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        let gold_after_buying = sim.gold();

        assert!(sim.sell_tower(pos));

        let expected_refund = (TowerKind::Cannon.price() as f32 * SELL_REFUND_FRACTION).round() as i32;
        assert_eq!(sim.gold(), gold_after_buying + expected_refund);
    }
}
