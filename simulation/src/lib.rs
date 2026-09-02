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
    /// A permanent, un-buildable, un-sellable wall baked into the
    /// current Level's map — distinct from a player-placed Tower.
    Obstacle,
}

/// The fixed layout for one Level: Spawn/Goal position plus every
/// permanently-blocked Cell (map walls, not player Towers). Every
/// Level shares the same Spawn/Goal so the Wave/Gold/Lives economy
/// carries over unchanged; only the maze of Obstacle differs.
struct LevelConfig {
    spawn: CellPos,
    goal: CellPos,
    obstacles: Vec<CellPos>,
}

/// How many Level the game cycles through before a Victory. Clearing
/// `TOTAL_WAVES` on any but the last Level advances to the next
/// Level's map instead of ending the game (see
/// `Simulation::tick_wave_completion`).
pub const LEVEL_COUNT: usize = 3;

/// Builds Level `level`'s fixed layout (clamped to the last Level for
/// any out-of-range index). Each entry beyond Level 0 (the original
/// monotonous straight-line map) adds full-column walls with a single
/// gap, alternating the gap between near the top and near the bottom
/// row so the Path zigzags instead of running straight across —
/// mirroring the single-gap-stays-valid Blocking Rule guarantee
/// already proven by `sealing_the_entire_maze_is_rejected...`.
fn level_config(level: usize) -> LevelConfig {
    let spawn = CellPos::new(0, GRID_SIZE / 2);
    let goal = CellPos::new(GRID_SIZE - 1, GRID_SIZE / 2);
    let walls: &[(usize, usize)] = match level.min(LEVEL_COUNT - 1) {
        0 => &[],
        1 => &[(8, 3), (16, GRID_SIZE - 4)],
        _ => &[(6, 3), (12, GRID_SIZE - 4), (18, 3)],
    };
    let obstacles = walls
        .iter()
        .flat_map(|&(x, gap_y)| (0..GRID_SIZE).filter(move |&y| y != gap_y).map(move |y| CellPos::new(x, y)))
        .collect();
    LevelConfig { spawn, goal, obstacles }
}

#[derive(Debug, Clone)]
pub struct Grid {
    spawn: CellPos,
    goal: CellPos,
    obstacles: HashSet<CellPos>,
}

impl Grid {
    /// Level 0's fixed 25x25 map: one Spawn Cell on the left edge, one
    /// Goal Cell on the right edge, everything else Buildable — the
    /// original straight-line layout, kept as the default/no-arg
    /// constructor for backward compatibility.
    pub fn new() -> Self {
        Self::for_level(0)
    }

    /// Builds the fixed map for Level `level` (clamped to the last
    /// Level for any out-of-range index): same Spawn/Goal as every
    /// other Level, plus that Level's permanent Obstacle walls.
    fn for_level(level: usize) -> Self {
        let config = level_config(level);
        Self {
            spawn: config.spawn,
            goal: config.goal,
            obstacles: config.obstacles.into_iter().collect(),
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
        } else if self.obstacles.contains(&pos) {
            CellKind::Obstacle
        } else {
            CellKind::Buildable
        }
    }

    fn is_obstacle(&self, pos: CellPos) -> bool {
        self.obstacles.contains(&pos)
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
    /// The game has already ended in Victory or Defeat.
    GameOver,
}

/// Balance numbers pending playtesting. Tier-scaling (ticket 07) and
/// Wave-scaling (ticket 08) land later; these are the base values.
const GRUNT_HEALTH: f32 = 100.0;
const GRUNT_SPEED_CELLS_PER_SEC: f32 = 2.0;
const RUNNER_HEALTH: f32 = 50.0;
const RUNNER_SPEED_CELLS_PER_SEC: f32 = 3.5;
const TANK_HEALTH: f32 = 220.0;
const TANK_SPEED_CELLS_PER_SEC: f32 = 1.0;
/// Boss base Health, well above Tank's: a plain high-Health MVP with
/// no distinct mechanics (out of scope: anything past that).
const BOSS_HEALTH: f32 = 900.0;
const BOSS_SPEED_CELLS_PER_SEC: f32 = 1.0;

const CANNON_DAMAGE: f32 = 50.0;
const CANNON_RANGE_CELLS: f32 = 5.0;
const CANNON_COOLDOWN_SECONDS: f32 = 1.0;
const GATLING_DAMAGE: f32 = 11.0;
const GATLING_RANGE_CELLS: f32 = 4.0;
const GATLING_COOLDOWN_SECONDS: f32 = 0.25;
const FROST_RANGE_CELLS: f32 = 3.5;
/// Fraction of normal speed an Enemy moves at while inside a Frost
/// Tower's Range (re-evaluated every tick; no lingering effect).
const FROST_SLOW_MULTIPLIER: f32 = 0.5;

const PROJECTILE_SPEED_CELLS_PER_SEC: f32 = 8.0;
const PROJECTILE_HIT_DISTANCE_CELLS: f32 = 0.25;

/// Gold economy balance numbers pending playtesting.
///
/// Raised from 180 after a real-engine (not closed-form) optimal-play
/// simulation showed the original value bought only a single Cannon
/// at Wave 1's start — the second (still un-marked-up, since
/// `fibonacci(1) == fibonacci(2) == 1`) Cannon a real player needs to
/// cover the path's far half stayed unaffordable until a couple of
/// Enemy had already leaked. 200 buys both from the opening whistle:
/// `optimal_play_clears_wave_one_with_no_leaks` locks this in. The
/// bump is small enough to barely register by the late-game economy
/// the total-Tower-count price markup (see `TOWER_COUNT_PRICE_GROWTH_RATE`)
/// already governs.
const STARTING_GOLD: i32 = 200;
const CANNON_PRICE: i32 = 100;
const GATLING_PRICE: i32 = 90;
const FROST_PRICE: i32 = 120;
const GRUNT_GOLD_REWARD: i32 = 10;
const RUNNER_GOLD_REWARD: i32 = 6;
const TANK_GOLD_REWARD: i32 = 20;
const BOSS_GOLD_REWARD: i32 = 60;
/// Fraction of the Gold spent on a Tower refunded on sale.
const SELL_REFUND_FRACTION: f32 = 0.7;
/// Base fraction of a Tower Kind's price each upgrade costs, scaled up
/// per target Tier by `UPGRADE_COST_TIER_GROWTH_RATE` — see
/// `upgrade_cost`.
const UPGRADE_COST_FRACTION: f32 = 0.8;
/// How much pricier each further upgrade step is than the last:
/// reaching Tier `t` costs `UPGRADE_COST_FRACTION * (1 + this * (t -
/// 1))` of the Kind's base price, so Tier 2 costs the base
/// `UPGRADE_COST_FRACTION` fraction and Tier 3 costs that fraction
/// scaled up by `1 + this`. Keeps the *marginal* Gold-per-damage of
/// finishing a Tower to Tier 3 from beating the marginal value of
/// stopping at Tier 2, which a flat upgrade cost does not: `damage`
/// grows by `TIER_STAT_MULTIPLIER` per Tier while a flat cost stays
/// flat, so the last upgrade step would otherwise be the cheapest
/// one — this rate roughly cancels that out instead of removing the
/// upgrade-over-a-fresh-Tower discount entirely.
const UPGRADE_COST_TIER_GROWTH_RATE: f32 = 0.3;
/// How steeply every Tower's price climbs with how many Tower —
/// *any* Kind — are already on the Grid: buying the `k`-th Tower
/// overall (1-indexed — the very first purchase is `k = 1`) costs
/// `kind.price() * (1 + this * ln(fibonacci(k)))`. `fibonacci(1) ==
/// fibonacci(2) == 1`, so the first two Tower placed (of any Kind, in
/// any combination) cost the same, un-marked-up price; from the third
/// on the markup climbs, but the `ln` keeps its *growth itself* from
/// compounding the way raw Fibonacci would — each further Tower still
/// costs more, but the per-Tower increase levels off toward a fixed
/// cap instead of accelerating. Reading total Tower count rather than
/// per-Kind count means switching Kind doesn't dodge the markup, so
/// blanketing the Grid in Tower is discouraged regardless of Kind
/// mix, without ever being hard-capped. Selling one back lowers the
/// price of the next purchase too, since this reads currently-placed
/// count, not lifetime purchases.
const TOWER_COUNT_PRICE_GROWTH_RATE: f32 = 0.25;
/// Multiplier applied to a Tower's primary stat per Tier over Tier 1.
const TIER_STAT_MULTIPLIER: f32 = 1.3;

/// Total Wave count for the MVP (out of scope: anything past Wave 15).
pub const TOTAL_WAVES: u32 = 15;
/// Lives the player starts with; a Leak decrements this by 1.
const STARTING_LIVES: i32 = 15;
/// Seconds between each Enemy spawning within a Wave at Wave 1.
const SPAWN_INTERVAL_SECONDS: f32 = 0.8;
/// Per-Wave reduction in the spawn interval (seconds), so later Waves
/// pressurise the player with faster spawns; clamped at a floor of
/// `SPAWN_INTERVAL_FLOOR_SECONDS`.
const SPAWN_INTERVAL_PER_WAVE_REDUCTION: f32 = 0.02;
/// Floor the Wave-scaled spawn interval never drops below.
const SPAWN_INTERVAL_FLOOR_SECONDS: f32 = 0.3;
/// Health multiplier applied to every Enemy in Wave `n`: `1 + n * this`.
const WAVE_HEALTH_SCALING_PER_WAVE: f32 = 0.15;
/// Enemy count every Wave starts from before Wave growth.
const WAVE_BASE_ENEMY_COUNT: u32 = 5;
/// Quadratic coefficient of Enemy count growth: Wave `n` spawns
/// `WAVE_BASE_ENEMY_COUNT + n + floor(WAVE_ENEMY_COUNT_GROWTH * n * n)`
/// Enemy total, so count grows super-linearly and later Waves spike.
const WAVE_ENEMY_COUNT_GROWTH: f32 = 0.1;
/// A Boss Enemy is appended to every Wave whose number is a multiple
/// of this (Wave 5, 10, 15, ...).
const BOSS_WAVE_INTERVAL: u32 = 5;

/// The four Enemy Kind, each with distinct Health/speed. Boss is a
/// plain high-Health variant for now (no distinct mechanics yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Grunt,
    Runner,
    Tank,
    Boss,
}

impl EnemyKind {
    fn health(self) -> f32 {
        match self {
            EnemyKind::Grunt => GRUNT_HEALTH,
            EnemyKind::Runner => RUNNER_HEALTH,
            EnemyKind::Tank => TANK_HEALTH,
            EnemyKind::Boss => BOSS_HEALTH,
        }
    }

    fn speed(self) -> f32 {
        match self {
            EnemyKind::Grunt => GRUNT_SPEED_CELLS_PER_SEC,
            EnemyKind::Runner => RUNNER_SPEED_CELLS_PER_SEC,
            EnemyKind::Tank => TANK_SPEED_CELLS_PER_SEC,
            EnemyKind::Boss => BOSS_SPEED_CELLS_PER_SEC,
        }
    }

    /// Gold granted to the player for killing this Enemy Kind.
    fn gold_reward(self) -> i32 {
        match self {
            EnemyKind::Grunt => GRUNT_GOLD_REWARD,
            EnemyKind::Runner => RUNNER_GOLD_REWARD,
            EnemyKind::Tank => TANK_GOLD_REWARD,
            EnemyKind::Boss => BOSS_GOLD_REWARD,
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
    /// This Tower Kind's primary stat at Tier 1: damage for Cannon and
    /// Gatling. Frost has no damage — its primary stat is Range (see
    /// `range`) — CONTEXT.md's Tier entry documents this choice.
    fn base_damage(self) -> f32 {
        match self {
            TowerKind::Cannon => CANNON_DAMAGE,
            TowerKind::Gatling => GATLING_DAMAGE,
            TowerKind::Frost => 0.0,
        }
    }

    /// Damage at the given Tier: Cannon/Gatling's primary stat, scaled
    /// `TIER_STAT_MULTIPLIER` per Tier over Tier 1. Always 0 for Frost.
    fn damage(self, tier: TowerTier) -> f32 {
        self.base_damage() * tier.stat_multiplier()
    }

    /// Range at the given Tier. Frost's primary stat, scaled
    /// `TIER_STAT_MULTIPLIER` per Tier over Tier 1; fixed for Cannon
    /// and Gatling, whose primary stat is damage instead. Public so the
    /// Bevy layer can preview a not-yet-placed Tower's Range ring at
    /// Tier 1.
    pub fn range(self, tier: TowerTier) -> f32 {
        match self {
            TowerKind::Cannon => CANNON_RANGE_CELLS,
            TowerKind::Gatling => GATLING_RANGE_CELLS,
            TowerKind::Frost => FROST_RANGE_CELLS * tier.stat_multiplier(),
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

    /// Gold cost to place a fresh Tier 1 Tower of this Kind. Public so
    /// the Bevy layer can label the sidebar's Tower Kind buttons with
    /// their price.
    pub fn price(self) -> i32 {
        match self {
            TowerKind::Cannon => CANNON_PRICE,
            TowerKind::Gatling => GATLING_PRICE,
            TowerKind::Frost => FROST_PRICE,
        }
    }
}

/// A Tower's upgrade level: Tier 1 (base) through Tier 3, the cap.
/// Each step over Tier 1 multiplies the Tower Kind's primary stat by
/// `TIER_STAT_MULTIPLIER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerTier {
    One,
    Two,
    Three,
}

impl TowerTier {
    fn stat_multiplier(self) -> f32 {
        match self {
            TowerTier::One => 1.0,
            TowerTier::Two => TIER_STAT_MULTIPLIER,
            TowerTier::Three => TIER_STAT_MULTIPLIER * TIER_STAT_MULTIPLIER,
        }
    }

    /// The next Tier up, or `None` if already at the Tier 3 cap.
    fn next(self) -> Option<TowerTier> {
        match self {
            TowerTier::One => Some(TowerTier::Two),
            TowerTier::Two => Some(TowerTier::Three),
            TowerTier::Three => None,
        }
    }

    /// This Tier's 1-indexed position (Tier 1 = 1, Tier 2 = 2, Tier 3
    /// = 3) — feeds `upgrade_cost`'s per-Tier scaling.
    fn ordinal(self) -> u32 {
        match self {
            TowerTier::One => 1,
            TowerTier::Two => 2,
            TowerTier::Three => 3,
        }
    }

    pub fn is_max(self) -> bool {
        self == TowerTier::Three
    }
}

/// Why an upgrade attempt failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeError {
    /// There is no Tower at the target Cell.
    NoTowerThere,
    /// The Tower is already at the Tier 3 cap.
    AlreadyMaxTier,
    /// The player does not have enough Gold for this upgrade step.
    InsufficientGold,
    /// The game has already ended in Victory or Defeat.
    GameOver,
}

/// A placed Tower's Kind, Tier, and current stats — for the Bevy
/// layer's info panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TowerStats {
    pub kind: TowerKind,
    pub tier: TowerTier,
    pub damage: f32,
    pub range: f32,
}

/// Why a "Start Next Wave" attempt failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveError {
    /// The current Wave's Enemy are still spawning or alive.
    WaveInProgress,
    /// The game has already ended in Victory or Defeat.
    GameOver,
}

/// How the game ended. Once set, `tick` becomes a no-op — Defeat and
/// Victory both freeze the Simulation exactly where it stood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    Victory,
    Defeat,
}

/// Something noteworthy `tick` did this call, for the Bevy layer to
/// react to (play a cue, update the sidebar, show a result screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimEvent {
    EnemyKilled(EnemyKind),
    /// An Enemy reached the Goal; Lives decremented by 1.
    Leak,
    WaveCleared(u32),
    /// Clearing `TOTAL_WAVES` on a non-final Level (the `u32` is the
    /// Level just cleared, 1-indexed) advanced to the next Level's
    /// map instead of ending the game: Gold/Lives carry over, every
    /// placed Tower is sold at the usual refund, and Wave resets to 1.
    LevelCleared(u32),
    Victory,
    Defeat,
}

/// The Enemy Kind cycled through to build a Wave's spawn queue:
/// Grunt, Runner, Tank, repeating — an even, arbitrary mix (ticket 08
/// leaves the exact distribution to the implementer).
const WAVE_ENEMY_KIND_CYCLE: [EnemyKind; 3] = [EnemyKind::Grunt, EnemyKind::Runner, EnemyKind::Tank];

/// A single Enemy in transit between two Cell centers. `target` is
/// `None` only in the rare case its onward Path vanished entirely
/// (see `Simulation::tick_enemies_movement`). `id` is unique for the
/// Enemy's lifetime, so a Projectile can keep tracking the specific
/// Enemy it was fired at even among several on screen at once.
#[derive(Debug, Clone, Copy)]
struct Enemy {
    id: u32,
    at: CellPos,
    target: Option<CellPos>,
    progress: f32,
    health: f32,
    kind: EnemyKind,
}

impl Enemy {
    /// Current position in Cell units, interpolated between `at` and
    /// `target`'s centers.
    fn position_cells(&self) -> (f32, f32) {
        let to = self.target.unwrap_or(self.at);
        let (fx, fy) = (self.at.x as f32, self.at.y as f32);
        let (tx, ty) = (to.x as f32, to.y as f32);
        (fx + (tx - fx) * self.progress, fy + (ty - fy) * self.progress)
    }
}

/// Per-Tower runtime state. `gold_spent` accumulates the price actually
/// paid to place this Tower (including any total-Tower-count markup —
/// see `Simulation::tower_price`) plus every upgrade paid, and drives
/// the sell refund. Upgrade cost, by contrast, is deliberately *not*
/// derived from `gold_spent`: it always reads `kind.price()`, the flat
/// base price, so a Tower bought at a markup doesn't also upgrade for
/// more — the markup is a placement-time cost only.
#[derive(Debug, Clone, Copy)]
struct TowerRuntime {
    kind: TowerKind,
    tier: TowerTier,
    cooldown_remaining: f32,
    gold_spent: i32,
}

/// A shot in flight, tracking its target Enemy's position every tick
/// by `target_id` (ADR-0001: plain distance check, no physics
/// engine). Position is in Cell units, not pixels — the Bevy layer
/// converts. `damage` carries the firing Tower Kind's damage so
/// Gatling and Cannon shots resolve differently on hit.
#[derive(Debug, Clone, Copy)]
struct Projectile {
    pos: (f32, f32),
    damage: f32,
    target_id: u32,
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

/// The `n`-th Fibonacci number, 1-indexed with `fibonacci(1) ==
/// fibonacci(2) == 1` (so `n` must be at least 1) — feeds
/// `tower_price`'s total-Tower-count markup.
fn fibonacci(n: u32) -> u64 {
    debug_assert!(n >= 1, "fibonacci is 1-indexed");
    let (mut previous, mut current) = (1u64, 1u64);
    for _ in 1..n {
        (previous, current) = (current, previous + current);
    }
    previous
}

/// Gold cost of upgrading to `target_tier`: the Tower Kind's flat base
/// price (`TowerKind::price`, never the total-Tower-count markup a
/// particular Tower may have actually paid) times `UPGRADE_COST_FRACTION`, scaled
/// up per `UPGRADE_COST_TIER_GROWTH_RATE` for how far `target_tier` is
/// past Tier 1 — so Tier 3 costs more to reach than Tier 2 did,
/// rounded to the nearest whole Gold (.5 away from zero) at each step.
fn upgrade_cost(base_price: i32, target_tier: TowerTier) -> i32 {
    let tier_growth = 1.0 + UPGRADE_COST_TIER_GROWTH_RATE * (target_tier.ordinal() as f32 - 1.0);
    (base_price as f32 * UPGRADE_COST_FRACTION * tier_growth).round() as i32
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
    enemies: Vec<Enemy>,
    next_enemy_id: u32,
    projectiles: Vec<Projectile>,
    gold: i32,
    /// The current Level, 0-indexed — see `LEVEL_COUNT`/`level_config`.
    level: usize,
    /// The Wave about to start (or currently in progress), 1-indexed.
    wave_number: u32,
    wave_in_progress: bool,
    /// Enemy Kind still waiting to spawn for the in-progress Wave.
    spawn_queue: VecDeque<EnemyKind>,
    /// Counts down to the next spawn; an Enemy spawns whenever this
    /// reaches zero and the queue isn't empty, then resets.
    spawn_timer: f32,
    lives: i32,
    /// Set the instant Defeat or Victory triggers; once set, `tick`
    /// no-ops forever after, freezing the Simulation in place.
    outcome: Option<GameOutcome>,
}

impl Simulation {
    pub fn new() -> Self {
        Self::new_at_level(0)
    }

    /// Starts a fresh game on Level `level` (0-indexed, clamped to the
    /// last Level for any out-of-range index) instead of always
    /// Level 0. Exists mainly as an authoring/testing seam for the
    /// last-Level Victory condition; real gameplay always starts at
    /// Level 0 via `new` and advances through `tick_wave_completion`.
    fn new_at_level(level: usize) -> Self {
        let level = level.min(LEVEL_COUNT - 1);
        Self {
            grid: Grid::for_level(level),
            towers: HashMap::new(),
            enemies: Vec::new(),
            next_enemy_id: 0,
            projectiles: Vec::new(),
            gold: STARTING_GOLD,
            level,
            wave_number: 1,
            wave_in_progress: false,
            spawn_queue: VecDeque::new(),
            spawn_timer: 0.0,
            lives: STARTING_LIVES,
            outcome: None,
        }
    }

    /// The current Level, 1-indexed, for the Bevy layer's sidebar
    /// (`level` itself is stored 0-indexed internally).
    pub fn level_number(&self) -> u32 {
        self.level as u32 + 1
    }

    pub fn lives(&self) -> i32 {
        self.lives
    }

    pub fn outcome(&self) -> Option<GameOutcome> {
        self.outcome
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

    /// The Kind, Tier, and current stats of the Tower at `pos`, if
    /// any — for the Bevy layer's info panel.
    pub fn tower_stats_at(&self, pos: CellPos) -> Option<TowerStats> {
        self.towers.get(&pos).map(|runtime| TowerStats {
            kind: runtime.kind,
            tier: runtime.tier,
            damage: runtime.kind.damage(runtime.tier),
            range: runtime.kind.range(runtime.tier),
        })
    }

    /// The Gold cost to upgrade the Tower at `pos` one Tier, if any and
    /// not already at the Tier 3 cap — for the Bevy layer's info panel.
    pub fn upgrade_cost_at(&self, pos: CellPos) -> Option<i32> {
        let runtime = self.towers.get(&pos)?;
        let next_tier = runtime.tier.next()?;
        Some(upgrade_cost(runtime.kind.price(), next_tier))
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

    /// How many Tower — of any Kind — are currently on the Grid, the
    /// count `tower_price` scales off. Selling one back lowers this,
    /// and so the price of the next purchase, regardless of Kind.
    fn total_tower_count(&self) -> u32 {
        self.towers.len() as u32
    }

    /// The Gold cost to place a fresh Tower of `kind` right now: its
    /// base `TowerKind::price`, marked up per `TOWER_COUNT_PRICE_GROWTH_RATE`
    /// for how many Tower of *any* Kind are already on the Grid — the
    /// first and second Tower placed always cost exactly `kind.price()`,
    /// since `fibonacci(1) == fibonacci(2) == 1`. Public so the Bevy
    /// layer can label the sidebar's Tower Kind buttons with the price
    /// the next click would pay.
    pub fn tower_price(&self, kind: TowerKind) -> i32 {
        // 1-indexed: placing this one would be the `ordinal`-th Tower
        // overall on the Grid.
        let ordinal = self.total_tower_count() + 1;
        let multiplier = 1.0 + TOWER_COUNT_PRICE_GROWTH_RATE * (fibonacci(ordinal) as f32).ln();
        (kind.price() as f32 * multiplier).round() as i32
    }

    /// Whether a Tower of the given Kind could be placed at `pos` right
    /// now, and if not, why.
    pub fn can_place(&self, pos: CellPos, kind: TowerKind) -> Result<(), PlacementError> {
        if self.outcome.is_some() {
            return Err(PlacementError::GameOver);
        }
        if self.grid.kind_at(pos) != CellKind::Buildable {
            return Err(PlacementError::NotBuildable);
        }
        if self.towers.contains_key(&pos) {
            return Err(PlacementError::AlreadyOccupied);
        }
        if self.gold < self.tower_price(kind) {
            return Err(PlacementError::InsufficientGold);
        }
        if self.shortest_path(self.grid.spawn(), Some(pos)).is_none() {
            return Err(PlacementError::WouldBlockPath);
        }
        Ok(())
    }

    pub fn place_tower(&mut self, pos: CellPos, kind: TowerKind) -> Result<(), PlacementError> {
        self.can_place(pos, kind)?;
        let price = self.tower_price(kind);
        self.gold -= price;
        self.towers.insert(
            pos,
            TowerRuntime {
                kind,
                tier: TowerTier::One,
                cooldown_remaining: 0.0,
                gold_spent: price,
            },
        );
        Ok(())
    }

    /// Upgrades the Tower at `pos` one Tier, if any and not already at
    /// the Tier 3 cap and the player can afford `upgrade_cost_at`.
    pub fn upgrade_tower(&mut self, pos: CellPos) -> Result<(), UpgradeError> {
        if self.outcome.is_some() {
            return Err(UpgradeError::GameOver);
        }
        let Some(runtime) = self.towers.get(&pos) else {
            return Err(UpgradeError::NoTowerThere);
        };
        let Some(next_tier) = runtime.tier.next() else {
            return Err(UpgradeError::AlreadyMaxTier);
        };
        let cost = upgrade_cost(runtime.kind.price(), next_tier);
        if self.gold < cost {
            return Err(UpgradeError::InsufficientGold);
        }

        self.gold -= cost;
        let runtime = self.towers.get_mut(&pos).unwrap();
        runtime.tier = next_tier;
        runtime.gold_spent += cost;
        Ok(())
    }

    /// Removes the Tower at `pos`, if any, refunding `SELL_REFUND_FRACTION`
    /// of the total Gold spent on it (purchase price plus every
    /// upgrade paid). The refund rounds to the nearest whole Gold, .5
    /// rounding away from zero. Returns whether a Tower was there.
    pub fn sell_tower(&mut self, pos: CellPos) -> bool {
        if self.outcome.is_some() {
            return false;
        }
        let Some(runtime) = self.towers.remove(&pos) else {
            return false;
        };
        self.gold += (runtime.gold_spent as f32 * SELL_REFUND_FRACTION).round() as i32;
        true
    }

    /// Spawns one Enemy of the given Kind at Spawn immediately,
    /// outside the Wave system. Kept as a direct testing/authoring
    /// seam for scenarios that only need a single Enemy; real
    /// gameplay spawns exclusively through `start_next_wave`.
    fn spawn_enemy_with_health(&mut self, kind: EnemyKind, health: f32) {
        let spawn = self.grid.spawn();
        let path = self.shortest_path(spawn, None);
        let id = self.next_enemy_id;
        self.next_enemy_id += 1;
        self.enemies.push(Enemy {
            id,
            at: spawn,
            target: path.and_then(|p| p.get(1).copied()),
            progress: 0.0,
            health,
            kind,
        });
    }

    /// Spawns one Enemy of the given Kind at Spawn with its unscaled
    /// base Health. A direct single-Enemy testing/authoring seam;
    /// real gameplay spawns exclusively through `start_next_wave`.
    pub fn spawn_enemy(&mut self, kind: EnemyKind) {
        self.spawn_enemy_with_health(kind, kind.health());
    }

    /// The Wave about to start (or currently in progress), 1-indexed.
    /// Starts at 1 before any Wave has been started.
    pub fn wave_number(&self) -> u32 {
        self.wave_number
    }

    pub fn wave_in_progress(&self) -> bool {
        self.wave_in_progress
    }

    /// Starts Wave `wave_number()`: queues
    /// `WAVE_BASE_ENEMY_COUNT + n` (plus quadratic growth, bounded by
    /// `EnemyKind` cycle) Enemy (mixed Grunt/Runner/Tank, see
    /// `WAVE_ENEMY_KIND_CYCLE`), plus one Boss appended last on every
    /// Wave that's a multiple of `BOSS_WAVE_INTERVAL`, to spawn one at
    /// a time, `SPAWN_INTERVAL_SECONDS` (Wave-scaled) apart, each with
    /// Health scaled by `1 + n * WAVE_HEALTH_SCALING_PER_WAVE`.
    /// Rejected while the current Wave is still spawning or has any
    /// Enemy alive.
    pub fn start_next_wave(&mut self) -> Result<(), WaveError> {
        if self.outcome.is_some() {
            return Err(WaveError::GameOver);
        }
        if self.wave_in_progress {
            return Err(WaveError::WaveInProgress);
        }
        self.spawn_queue = (0..self.wave_enemy_count())
            .map(|i| WAVE_ENEMY_KIND_CYCLE[i as usize % WAVE_ENEMY_KIND_CYCLE.len()])
            .collect();
        if self.wave_number % BOSS_WAVE_INTERVAL == 0 {
            self.spawn_queue.push_back(EnemyKind::Boss);
        }
        self.spawn_timer = 0.0;
        self.wave_in_progress = true;
        Ok(())
    }

    /// Total Enemy count (pre-Boss) the current Wave spawns: base plus
    /// linear and quadratic growth in the Wave number, so it climbs
    /// super-linearly and later Waves spike.
    fn wave_enemy_count(&self) -> u32 {
        let n = self.wave_number as f32;
        let quadratic = (WAVE_ENEMY_COUNT_GROWTH * n * n).floor() as u32;
        WAVE_BASE_ENEMY_COUNT + self.wave_number + quadratic
    }

    /// Seconds between Enemy spawns for the current Wave: base interval
    /// shrinking each Wave, never below `SPAWN_INTERVAL_FLOOR_SECONDS`.
    fn current_spawn_interval(&self) -> f32 {
        (SPAWN_INTERVAL_SECONDS - self.wave_number as f32 * SPAWN_INTERVAL_PER_WAVE_REDUCTION)
            .max(SPAWN_INTERVAL_FLOOR_SECONDS)
    }

    /// Health an Enemy spawned in the current Wave should have: base
    /// Health scaled by `1 + n * WAVE_HEALTH_SCALING_PER_WAVE`.
    fn current_wave_enemy_health(&self, kind: EnemyKind) -> f32 {
        kind.health() * (1.0 + self.wave_number as f32 * WAVE_HEALTH_SCALING_PER_WAVE)
    }

    /// Dev-only: jumps straight to `level` (clamped to
    /// `LEVEL_COUNT - 1`), mirroring the natural Level-clear transition
    /// (see ADR-0003) — refunds every placed Tower at the usual Sell
    /// rate, regenerates the Grid, and resets Wave to 1 — but skips
    /// actually having to clear every Wave first, and (unlike the
    /// natural transition) also clears any Wave in progress since it
    /// can be called mid-Wave. Exists for the game layer's dev command
    /// palette so later Levels can be playtested directly.
    pub fn debug_set_level(&mut self, level: usize) {
        let level = level.min(LEVEL_COUNT - 1);
        let refund: i32 = self
            .towers
            .values()
            .map(|runtime| (runtime.gold_spent as f32 * SELL_REFUND_FRACTION).round() as i32)
            .sum();
        self.gold += refund;
        self.towers.clear();
        self.level = level;
        self.grid = Grid::for_level(self.level);
        self.wave_number = 1;
        self.wave_in_progress = false;
        self.spawn_queue.clear();
        self.spawn_timer = 0.0;
        self.enemies.clear();
    }

    /// Dev-only: grants `amount` Gold outright, for the command
    /// palette's `gold` command — a shortcut around grinding Waves to
    /// afford a specific Tower layout while playtesting.
    pub fn debug_add_gold(&mut self, amount: i32) {
        self.gold += amount;
    }

    /// Dev-only: instantly clears the Wave in progress — despawns
    /// every live Enemy (no Gold reward, unlike a real kill) and drops
    /// the rest of the spawn queue — so the very next `tick` sees an
    /// empty queue and no Enemy left and completes the Wave through
    /// the usual `tick_wave_completion` path (advancing the Wave
    /// number, or the Level/Victory on the last Wave). A no-op if no
    /// Wave is in progress. For the command palette's `skipwave`
    /// command.
    pub fn debug_skip_wave(&mut self) {
        if !self.wave_in_progress {
            return;
        }
        self.spawn_queue.clear();
        self.enemies.clear();
    }

    pub fn enemy_alive(&self) -> bool {
        !self.enemies.is_empty()
    }

    pub fn enemy_health(&self) -> Option<f32> {
        self.enemies.first().map(|e| e.health)
    }

    pub fn projectile_count(&self) -> usize {
        self.projectiles.len()
    }

    /// Every Projectile's current position, in Cell units, for the
    /// Bevy layer to render.
    pub fn projectile_positions(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        self.projectiles.iter().map(|p| p.pos)
    }

    /// The first live Enemy's current transit segment (where it's
    /// coming from, where it's headed, how far along it is), for the
    /// Bevy layer to interpolate a world position from. `None` once no
    /// Enemy is alive. A single-Enemy convenience; `enemies_transits`
    /// covers every Enemy at once for real gameplay.
    pub fn enemy_transit(&self) -> Option<EnemyTransit> {
        self.enemies.first().map(Self::transit_of)
    }

    fn transit_of(enemy: &Enemy) -> EnemyTransit {
        EnemyTransit {
            from: enemy.at,
            to: enemy.target.unwrap_or(enemy.at),
            progress: if enemy.target.is_some() { enemy.progress } else { 0.0 },
        }
    }

    /// Every live Enemy's stable id, Kind, and current transit
    /// segment, for the Bevy layer to render each one. The id lets
    /// the Bevy layer give each Enemy a consistent draw-order/z-depth
    /// across frames even though its sprite is despawned and
    /// respawned fresh every frame — without it, two Enemy occupying
    /// the same on-screen position have no stable tie-break and
    /// visibly flicker as which one draws on top changes frame to
    /// frame.
    pub fn enemies_transits(&self) -> impl Iterator<Item = (u32, EnemyKind, EnemyTransit)> + '_ {
        self.enemies.iter().map(|e| (e.id, e.kind, Self::transit_of(e)))
    }

    /// Advances the whole Simulation by `dt` seconds: Wave spawning,
    /// then Enemy movement (at whatever speed the current Frost
    /// coverage allows), then Tower firing, then Projectile
    /// flight/impact, then Wave-completion bookkeeping. Returns every
    /// noteworthy `SimEvent` this call produced, in order. A no-op
    /// once `outcome()` is set — Defeat and Victory both freeze the
    /// Simulation exactly where it stood.
    pub fn tick(&mut self, dt: f32) -> Vec<SimEvent> {
        if self.outcome.is_some() {
            return Vec::new();
        }

        let mut events = Vec::new();
        self.tick_spawning(dt);
        events.extend(self.tick_enemies_movement(dt));
        if self.outcome.is_some() {
            // Defeat overrides everything else in progress this tick:
            // don't let Towers/Projectiles/Wave-completion act on a
            // board that no longer matters.
            return events;
        }
        self.tick_towers(dt);
        events.extend(self.tick_projectiles(dt));
        events.extend(self.tick_wave_completion());
        events
    }

    /// Dequeues Enemy from the in-progress Wave's `spawn_queue` one at
    /// a time, `SPAWN_INTERVAL_SECONDS` apart, starting immediately
    /// when a Wave begins.
    fn tick_spawning(&mut self, dt: f32) {
        if !self.wave_in_progress {
            return;
        }
        self.spawn_timer -= dt;
        while self.spawn_timer <= 0.0 {
            let Some(kind) = self.spawn_queue.pop_front() else {
                break;
            };
            let health = self.current_wave_enemy_health(kind);
            self.spawn_enemy_with_health(kind, health);
            self.spawn_timer += self.current_spawn_interval();
        }
    }

    /// A Wave is complete once every Enemy has been spawned and none
    /// remain alive. Advances to the next Wave number, unless the
    /// just-cleared Wave was the last (`TOTAL_WAVES`) — in which case,
    /// unless this was also the last Level (`LEVEL_COUNT`), the game
    /// instead advances to the next Level's map: every placed Tower is
    /// sold at the usual refund (the old map's layout no longer
    /// applies), Wave resets to 1, and Gold/Lives carry over. Only on
    /// the last Level's clear does Victory actually trigger (this only
    /// runs when `tick` hasn't already set Defeat, so Lives > 0 is
    /// implied).
    fn tick_wave_completion(&mut self) -> Vec<SimEvent> {
        if !(self.wave_in_progress && self.spawn_queue.is_empty() && self.enemies.is_empty()) {
            return Vec::new();
        }

        self.wave_in_progress = false;
        let cleared_wave = self.wave_number;
        let mut events = vec![SimEvent::WaveCleared(cleared_wave)];
        if cleared_wave >= TOTAL_WAVES {
            if self.level + 1 < LEVEL_COUNT {
                events.push(SimEvent::LevelCleared(self.level_number()));
                let refund: i32 = self
                    .towers
                    .values()
                    .map(|runtime| (runtime.gold_spent as f32 * SELL_REFUND_FRACTION).round() as i32)
                    .sum();
                self.gold += refund;
                self.towers.clear();
                self.level += 1;
                self.grid = Grid::for_level(self.level);
                self.wave_number = 1;
            } else {
                self.outcome = Some(GameOutcome::Victory);
                events.push(SimEvent::Victory);
            }
        } else {
            self.wave_number += 1;
        }
        events
    }

    /// Whether `enemy_pos` falls within any Frost Tower's Range right
    /// now. Re-evaluated fresh every tick, per Enemy — no lingering
    /// effect once an Enemy steps back outside.
    fn frost_slow_multiplier_at(&self, enemy_pos: (f32, f32)) -> f32 {
        let in_frost = self.towers.iter().any(|(pos, runtime)| {
            runtime.kind == TowerKind::Frost
                && distance((pos.x as f32, pos.y as f32), enemy_pos)
                    <= runtime.kind.range(runtime.tier)
        });
        if in_frost {
            FROST_SLOW_MULTIPLIER
        } else {
            1.0
        }
    }

    /// Advances every Enemy by `dt` seconds at its Kind's base speed
    /// times its own current Frost slow multiplier. Per ADR-0002, an
    /// Enemy's Path is only ever recomputed the instant it reaches a
    /// Cell center — never mid-transit, no matter how the Grid changes
    /// underneath it in the meantime. An Enemy that reaches Goal leaks:
    /// it despawns and Lives decrements by 1. The instant Lives
    /// reaches 0, Defeat triggers and this stops processing any
    /// remaining Enemy immediately — Defeat overrides everything else
    /// in progress.
    fn tick_enemies_movement(&mut self, dt: f32) -> Vec<SimEvent> {
        let mut events = Vec::new();
        let goal = self.grid.goal();
        let mut i = 0;
        while i < self.enemies.len() {
            let (at, target, mut progress, kind) = {
                let e = &self.enemies[i];
                (e.at, e.target, e.progress, e.kind)
            };

            let Some(target) = target else {
                // Stuck at a Cell whose onward Path vanished; try
                // again every tick in case the Grid opens back up.
                let new_target = self.shortest_path(at, None).and_then(|p| p.get(1).copied());
                self.enemies[i].target = new_target;
                i += 1;
                continue;
            };

            let speed_multiplier = self.frost_slow_multiplier_at(self.enemies[i].position_cells());
            progress += dt * kind.speed() * speed_multiplier;
            if progress < 1.0 {
                self.enemies[i].progress = progress;
                i += 1;
                continue;
            }

            if target == goal {
                self.enemies.remove(i);
                self.lives -= 1;
                events.push(SimEvent::Leak);
                if self.lives <= 0 {
                    self.outcome = Some(GameOutcome::Defeat);
                    events.push(SimEvent::Defeat);
                    return events;
                }
                continue;
            }

            // Just reached a Cell center: recompute the remaining Path
            // from here, picking up whatever the Grid looks like *now*.
            let new_target = self.shortest_path(target, None).and_then(|p| p.get(1).copied());
            let enemy = &mut self.enemies[i];
            enemy.at = target;
            enemy.progress = 0.0;
            enemy.target = new_target;
            i += 1;
        }
        events
    }

    /// Ticks down every projectile-firing Tower's cooldown and fires a
    /// Projectile at its nearest in-Range Enemy, if any, from any
    /// Tower that's ready. Frost Towers never fire — their slow is
    /// applied directly in `tick_enemies_movement`, not through this
    /// pipeline.
    fn tick_towers(&mut self, dt: f32) {
        if self.enemies.is_empty() {
            return;
        }

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
            let range = runtime.kind.range(runtime.tier);
            let nearest_in_range = self
                .enemies
                .iter()
                .map(|e| (e.id, distance(tower_pos, e.position_cells())))
                .filter(|&(_, dist)| dist <= range)
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if let Some((target_id, _)) = nearest_in_range {
                runtime.cooldown_remaining = runtime.kind.cooldown();
                self.projectiles.push(Projectile {
                    pos: tower_pos,
                    damage: runtime.kind.damage(runtime.tier),
                    target_id,
                });
            }
        }
    }

    /// Moves every Projectile toward its target Enemy's current
    /// position and resolves hits (ADR-0001: plain distance check). A
    /// Projectile whose target has died — from this hit or an earlier
    /// one this same tick, or from anything else — despawns without
    /// effect on any other Enemy.
    fn tick_projectiles(&mut self, dt: f32) -> Vec<SimEvent> {
        let in_flight = std::mem::take(&mut self.projectiles);
        let mut remaining = Vec::with_capacity(in_flight.len());
        let mut gold_earned = 0;
        let mut events = Vec::new();

        for mut projectile in in_flight {
            let Some(target_index) = self.enemies.iter().position(|e| e.id == projectile.target_id)
            else {
                continue;
            };
            let target_pos = self.enemies[target_index].position_cells();

            if distance(projectile.pos, target_pos) <= PROJECTILE_HIT_DISTANCE_CELLS {
                let enemy = &mut self.enemies[target_index];
                enemy.health -= projectile.damage;
                if enemy.health <= 0.0 {
                    gold_earned += enemy.kind.gold_reward();
                    events.push(SimEvent::EnemyKilled(enemy.kind));
                    self.enemies.remove(target_index);
                }
                continue;
            }

            let dx = target_pos.0 - projectile.pos.0;
            let dy = target_pos.1 - projectile.pos.1;
            let dist = (dx * dx + dy * dy).sqrt();
            let step = PROJECTILE_SPEED_CELLS_PER_SEC * dt;
            if dist > f32::EPSILON {
                projectile.pos.0 += dx / dist * step;
                projectile.pos.1 += dy / dist * step;
            }
            remaining.push(projectile);
        }
        self.projectiles = remaining;
        self.gold += gold_earned;
        events
    }

    /// BFS from `from` to Goal, treating every placed Tower — plus
    /// `extra_blocked`, if given — as impassable. `from` need not be
    /// Spawn: each Enemy recomputes its own remaining Path from
    /// wherever it currently stands (see ADR-0002).
    fn shortest_path(&self, from: CellPos, extra_blocked: Option<CellPos>) -> Option<Vec<CellPos>> {
        let goal = self.grid.goal();
        let is_blocked = |pos: CellPos| {
            Some(pos) == extra_blocked || self.towers.contains_key(&pos) || self.grid.is_obstacle(pos)
        };

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
        sim.enemies.clear();

        // Advancing further must not panic, must clear the now-orphaned
        // Projectile, and must not resurrect or otherwise affect anything.
        sim.tick(0.5);
        assert_eq!(sim.projectile_count(), 0);
        assert!(!sim.enemy_alive());
    }

    #[test]
    fn gatling_fires_faster_and_weaker_than_cannon() {
        assert!(TowerKind::Gatling.base_damage() < TowerKind::Cannon.base_damage());
        assert!(TowerKind::Gatling.cooldown() < TowerKind::Cannon.cooldown());
    }

    #[test]
    fn each_enemy_kind_has_distinct_correct_stats() {
        assert!(EnemyKind::Runner.health() < EnemyKind::Grunt.health());
        assert!(EnemyKind::Grunt.health() < EnemyKind::Tank.health());
        assert!(EnemyKind::Tank.health() < EnemyKind::Boss.health());
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
            let enemy = sim.enemies.first_mut().unwrap();
            enemy.at = CellPos::new(3, 12);
            enemy.target = Some(CellPos::new(4, 12));
            enemy.progress = 0.0;
        }
        let pos_in_range = sim.enemies[0].position_cells();
        assert_eq!(sim.frost_slow_multiplier_at(pos_in_range), FROST_SLOW_MULTIPLIER);

        // One Cell further out, (4,12), is just outside Range: distance
        // to (0,13) is sqrt(4^2 + 1^2) ~= 4.12, over FROST_RANGE_CELLS.
        sim.enemies.first_mut().unwrap().at = CellPos::new(4, 12);
        let pos_out_of_range = sim.enemies[0].position_cells();
        assert_eq!(sim.frost_slow_multiplier_at(pos_out_of_range), 1.0);
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
    fn the_first_two_towers_overall_are_flat_priced_then_price_climbs_at_a_capped_rate() {
        let mut sim = Simulation::new();
        sim.gold = 1_000_000;
        let base = TowerKind::Cannon.price();

        // fibonacci(1) == fibonacci(2) == 1: the first two Tower placed
        // overall cost the same, un-marked-up price.
        assert_eq!(sim.tower_price(TowerKind::Cannon), base);
        sim.place_tower(CellPos::new(1, 1), TowerKind::Cannon).unwrap();
        assert_eq!(
            sim.tower_price(TowerKind::Cannon),
            base,
            "the second Tower overall should still cost the base price"
        );
        sim.place_tower(CellPos::new(2, 1), TowerKind::Cannon).unwrap();

        // From the third Tower on, price climbs every further purchase.
        let mut prices = vec![sim.tower_price(TowerKind::Cannon)];
        assert!(prices[0] > base, "the third Tower should cost more than the base price");
        for (i, x) in (3..=9).enumerate() {
            sim.place_tower(CellPos::new(x, 1), TowerKind::Cannon)
                .unwrap_or_else(|e| panic!("Cannon #{} should place cleanly: {e:?}", i + 3));
            prices.push(sim.tower_price(TowerKind::Cannon));
        }
        for pair in prices.windows(2) {
            assert!(pair[1] > pair[0], "price should keep rising as more Tower overall are placed");
        }

        // A pure Fibonacci markup (no log) would widen the per-Tower
        // step every time, since Fibonacci itself grows multiplicatively.
        // The log instead caps *how fast the growth grows*: the late
        // step should not run away from the early one.
        let early_step = prices[1] - prices[0];
        let late_step = prices[prices.len() - 1] - prices[prices.len() - 2];
        assert!(
            late_step <= early_step * 2,
            "the per-Tower price increase should level off, not keep accelerating \
             (early step {early_step}, late step {late_step})"
        );

        // Selling back down to a single Tower returns to the flat
        // price (fibonacci(2) == 1 too, so one remaining Tower still
        // prices the next purchase at the base rate).
        for x in (2..=9).rev() {
            assert!(sim.sell_tower(CellPos::new(x, 1)));
        }
        assert_eq!(sim.tower_price(TowerKind::Cannon), base);
    }

    #[test]
    fn switching_kind_does_not_dodge_the_tower_count_markup() {
        let mut sim = Simulation::new();
        sim.gold = 1_000_000;

        // First two Tower overall, in different Kind, both flat-priced.
        assert_eq!(sim.tower_price(TowerKind::Cannon), TowerKind::Cannon.price());
        sim.place_tower(CellPos::new(1, 1), TowerKind::Cannon).unwrap();
        assert_eq!(
            sim.tower_price(TowerKind::Gatling),
            TowerKind::Gatling.price(),
            "the second Tower overall, even of a different Kind, should still be base price"
        );
        sim.place_tower(CellPos::new(2, 1), TowerKind::Gatling).unwrap();

        // The third Tower is marked up for every Kind, not just the
        // Kind already on the Grid.
        assert!(sim.tower_price(TowerKind::Cannon) > TowerKind::Cannon.price());
        let gatling_price_for_third = sim.tower_price(TowerKind::Gatling);
        assert!(gatling_price_for_third > TowerKind::Gatling.price());

        // Buying a Cannon (not a Gatling) still raises the price of the
        // next Gatling — the markup reads total Tower count, so
        // alternating Kind cannot dodge it.
        sim.place_tower(CellPos::new(3, 1), TowerKind::Cannon).unwrap();
        assert!(
            sim.tower_price(TowerKind::Gatling) > gatling_price_for_third,
            "a Cannon purchase should still raise the next Gatling's price"
        );

        // Selling any Tower — regardless of Kind — lowers the price of
        // the next purchase of every Kind.
        assert!(sim.sell_tower(CellPos::new(3, 1)));
        assert_eq!(sim.tower_price(TowerKind::Gatling), gatling_price_for_third);
    }

    #[test]
    fn upgrade_cost_ignores_the_tower_count_markup_a_tower_actually_paid() {
        let mut sim = Simulation::new();
        sim.gold = 1_000_000;

        // A Cannon bought cheap (first Tower overall) ...
        let cheap_pos = CellPos::new(1, 1);
        sim.place_tower(cheap_pos, TowerKind::Cannon).unwrap();
        let cheap_upgrade_cost = sim.upgrade_cost_at(cheap_pos).unwrap();

        // ... and one bought at a steep total-Tower-count markup ...
        for (i, x) in (2..=6).enumerate() {
            sim.place_tower(CellPos::new(x, 1), TowerKind::Cannon)
                .unwrap_or_else(|e| panic!("Cannon #{} should place cleanly: {e:?}", i + 2));
        }
        let marked_up_pos = CellPos::new(6, 1);
        assert!(
            sim.tower_price(TowerKind::Cannon) > TowerKind::Cannon.price(),
            "the next Cannon purchase should already be marked up"
        );
        let marked_up_upgrade_cost = sim.upgrade_cost_at(marked_up_pos).unwrap();

        // ... should both cost the same to reach Tier 2, tied to the
        // Kind's base price and target Tier rather than what either
        // Tower actually paid to place.
        assert_eq!(cheap_upgrade_cost, marked_up_upgrade_cost);
        assert_eq!(
            cheap_upgrade_cost,
            (TowerKind::Cannon.price() as f32
                * UPGRADE_COST_FRACTION
                * (1.0 + UPGRADE_COST_TIER_GROWTH_RATE))
                .round() as i32
        );
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

    #[test]
    fn upgrading_increases_damage_thirty_percent_per_tier_and_deducts_a_growing_cost() {
        let mut sim = Simulation::new();
        sim.gold = 100_000;
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        let gold_after_buying = sim.gold();
        let tier_one_damage = sim.tower_stats_at(pos).unwrap().damage;
        assert_eq!(tier_one_damage, TowerKind::Cannon.base_damage());

        let cost_to_tier_two = sim.upgrade_cost_at(pos).unwrap();
        assert_eq!(
            cost_to_tier_two,
            (TowerKind::Cannon.price() as f32
                * UPGRADE_COST_FRACTION
                * (1.0 + UPGRADE_COST_TIER_GROWTH_RATE))
                .round() as i32,
            "Tier 2 should cost the base upgrade fraction scaled for one step past Tier 1"
        );
        assert!(sim.upgrade_tower(pos).is_ok());
        let after_tier_two = sim.tower_stats_at(pos).unwrap();
        assert_eq!(after_tier_two.tier, TowerTier::Two);
        assert!((after_tier_two.damage - tier_one_damage * TIER_STAT_MULTIPLIER).abs() < 0.001);
        assert_eq!(sim.gold(), gold_after_buying - cost_to_tier_two);

        // Reaching Tier 3 costs more than reaching Tier 2 did — the
        // per-Tier growth keeps finishing a Tower to Tier 3 from being
        // *more* Gold-efficient than stopping at Tier 2, which a flat
        // upgrade cost would allow (damage keeps compounding by
        // TIER_STAT_MULTIPLIER while a flat cost wouldn't).
        let cost_to_tier_three = sim.upgrade_cost_at(pos).unwrap();
        assert!(
            cost_to_tier_three > cost_to_tier_two,
            "Tier 3 should cost more to reach than Tier 2 did"
        );
        assert_eq!(
            cost_to_tier_three,
            (TowerKind::Cannon.price() as f32
                * UPGRADE_COST_FRACTION
                * (1.0 + UPGRADE_COST_TIER_GROWTH_RATE * 2.0))
                .round() as i32
        );
        assert!(sim.upgrade_tower(pos).is_ok());
        let after_tier_three = sim.tower_stats_at(pos).unwrap();
        assert_eq!(after_tier_three.tier, TowerTier::Three);
        assert!(
            (after_tier_three.damage - tier_one_damage * TIER_STAT_MULTIPLIER * TIER_STAT_MULTIPLIER).abs()
                < 0.001
        );
    }

    #[test]
    fn upgrade_is_blocked_at_tier_three() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.gold = 100_000;
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        sim.upgrade_tower(pos).unwrap();
        sim.upgrade_tower(pos).unwrap();

        assert_eq!(sim.upgrade_tower(pos), Err(UpgradeError::AlreadyMaxTier));
        assert!(
            sim.upgrade_cost_at(pos).is_none(),
            "a Tier 3 Tower should report no upgrade available"
        );
    }

    #[test]
    fn upgrade_is_blocked_when_unaffordable_and_state_is_unchanged() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        sim.gold = 0;
        let tier_before = sim.tower_stats_at(pos).unwrap().tier;

        assert_eq!(sim.upgrade_tower(pos), Err(UpgradeError::InsufficientGold));
        assert_eq!(sim.gold(), 0);
        assert_eq!(sim.tower_stats_at(pos).unwrap().tier, tier_before);
    }

    #[test]
    fn selling_an_upgraded_tower_refunds_seventy_percent_of_purchase_plus_upgrade_spend() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.gold = 100_000;
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        let upgrade_cost = sim.upgrade_cost_at(pos).unwrap();
        sim.upgrade_tower(pos).unwrap();
        let gold_before_sell = sim.gold();
        let total_spent = TowerKind::Cannon.price() + upgrade_cost;

        assert!(sim.sell_tower(pos));

        let expected_refund = (total_spent as f32 * SELL_REFUND_FRACTION).round() as i32;
        assert_eq!(sim.gold(), gold_before_sell + expected_refund);
    }

    #[test]
    fn wave_enemy_count_and_health_scale_correctly_for_wave_one_a_middle_wave_and_wave_fifteen() {
        fn assert_wave_matches_formula_then_force_complete(sim: &mut Simulation, n: u32) {
            assert_eq!(sim.wave_number(), n);
            sim.start_next_wave().unwrap();
            let quadratic = (WAVE_ENEMY_COUNT_GROWTH * n as f32 * n as f32).floor() as u32;
            let expected_count = WAVE_BASE_ENEMY_COUNT
                + n
                + quadratic
                + if n % BOSS_WAVE_INTERVAL == 0 { 1 } else { 0 };
            assert_eq!(sim.spawn_queue.len() as u32, expected_count);

            // spawn_timer starts at 0.0, so a negligible tick spawns
            // exactly the queue's first (Grunt) Enemy and no more.
            sim.tick(0.001);
            let expected_health =
                EnemyKind::Grunt.health() * (1.0 + n as f32 * WAVE_HEALTH_SCALING_PER_WAVE);
            assert_eq!(sim.enemy_health(), Some(expected_health));

            // Force this Wave to completion so the next assertion
            // starts from a clean, un-in-progress state. Wave
            // TOTAL_WAVES's clear triggers Victory instead of
            // advancing wave_number further.
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
            if n >= TOTAL_WAVES {
                assert_eq!(sim.outcome(), Some(GameOutcome::Victory));
            } else {
                assert_eq!(sim.wave_number(), n + 1);
            }
        }

        // Runs on the last Level, so clearing Wave 15 triggers Victory
        // rather than advancing to another Level's map — this test is
        // about Wave scaling, not Level progression (see the
        // `level_*` tests below for that).
        let mut sim = Simulation::new_at_level(LEVEL_COUNT - 1);
        assert_wave_matches_formula_then_force_complete(&mut sim, 1);

        while sim.wave_number() < 7 {
            sim.start_next_wave().unwrap();
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
        }
        assert_wave_matches_formula_then_force_complete(&mut sim, 7);

        while sim.wave_number() < 15 {
            sim.start_next_wave().unwrap();
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
        }
        assert_wave_matches_formula_then_force_complete(&mut sim, 15);
    }

    #[test]
    fn starting_next_wave_is_rejected_while_current_wave_is_in_progress() {
        let mut sim = Simulation::new();
        sim.start_next_wave().unwrap();

        assert_eq!(sim.start_next_wave(), Err(WaveError::WaveInProgress));

        // Still rejected once every Enemy has spawned but at least one
        // is still alive.
        sim.spawn_queue.clear();
        assert_eq!(sim.start_next_wave(), Err(WaveError::WaveInProgress));
    }

    #[test]
    fn a_leak_decrements_lives_and_emits_a_leak_event() {
        let mut sim = Simulation::new();
        sim.spawn_enemy(EnemyKind::Grunt);
        let lives_before = sim.lives();

        let steps_to_goal = sim.grid().goal().x - sim.grid().spawn().x;
        let mut saw_leak = false;
        for _ in 0..steps_to_goal {
            let events = sim.tick(1.0 / EnemyKind::Grunt.speed());
            if events.contains(&SimEvent::Leak) {
                saw_leak = true;
            }
        }

        assert!(saw_leak, "walking an Enemy to Goal should emit a Leak event");
        assert_eq!(sim.lives(), lives_before - 1);
        assert!(!sim.enemy_alive());
    }

    #[test]
    fn lives_hitting_zero_emits_defeat_immediately_regardless_of_remaining_enemy() {
        let mut sim = Simulation::new();
        sim.lives = 1;
        // Grunt (faster) leaks first and should zero Lives before the
        // still-alive, slower Tank ever gets processed this tick.
        sim.spawn_enemy(EnemyKind::Grunt);
        sim.spawn_enemy(EnemyKind::Tank);

        let steps_to_goal = sim.grid().goal().x - sim.grid().spawn().x;
        let mut saw_defeat = false;
        for _ in 0..steps_to_goal {
            let events = sim.tick(1.0 / EnemyKind::Grunt.speed());
            if events.contains(&SimEvent::Defeat) {
                saw_defeat = true;
                break;
            }
        }

        assert!(saw_defeat, "Lives reaching 0 should emit Defeat");
        assert_eq!(sim.outcome(), Some(GameOutcome::Defeat));
        assert_eq!(sim.lives(), 0);
        assert!(
            sim.enemy_alive(),
            "Defeat should override processing the remaining Tank this tick, leaving it untouched"
        );

        // Once Defeat has triggered, further ticks are pure no-ops.
        let events = sim.tick(1.0);
        assert!(events.is_empty());
    }

    #[test]
    fn wave_fifteen_clear_with_lives_above_zero_emits_victory() {
        // Runs on the last Level; see `level_*` tests for the
        // non-final-Level case, where the same clear instead advances
        // to the next map.
        let mut sim = Simulation::new_at_level(LEVEL_COUNT - 1);
        while sim.wave_number() < TOTAL_WAVES {
            sim.start_next_wave().unwrap();
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
        }
        assert_eq!(sim.wave_number(), TOTAL_WAVES);
        assert!(sim.lives() > 0);

        sim.start_next_wave().unwrap();
        sim.spawn_queue.clear();
        sim.enemies.clear();
        let events = sim.tick(0.001);

        assert!(events.contains(&SimEvent::Victory));
        assert_eq!(sim.outcome(), Some(GameOutcome::Victory));
    }

    #[test]
    fn a_non_final_wave_clear_emits_neither_victory_nor_defeat() {
        let mut sim = Simulation::new();
        sim.start_next_wave().unwrap();
        sim.spawn_queue.clear();
        sim.enemies.clear();

        let events = sim.tick(0.001);

        assert!(events.contains(&SimEvent::WaveCleared(1)));
        assert!(!events
            .iter()
            .any(|e| matches!(e, SimEvent::Victory | SimEvent::Defeat)));
        assert_eq!(sim.outcome(), None);
        assert_eq!(sim.wave_number(), 2);
    }

    #[test]
    fn once_the_game_has_ended_placement_upgrade_sell_and_next_wave_all_reject() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        sim.outcome = Some(GameOutcome::Defeat);

        assert_eq!(
            sim.place_tower(CellPos::new(6, 6), TowerKind::Cannon),
            Err(PlacementError::GameOver)
        );
        assert_eq!(sim.upgrade_tower(pos), Err(UpgradeError::GameOver));
        assert!(
            !sim.sell_tower(pos),
            "selling should be inert once the game has ended"
        );
        assert!(sim.has_tower(pos), "the Tower should be untouched by the rejected sell");
        assert_eq!(sim.start_next_wave(), Err(WaveError::GameOver));
    }

    #[test]
    fn every_level_has_a_valid_path_and_at_least_one_obstacle_beyond_level_one() {
        for level in 0..LEVEL_COUNT {
            let sim = Simulation::new_at_level(level);
            assert!(
                sim.current_path().is_some(),
                "Level {level} should always have an open Path from Spawn to Goal"
            );
            let obstacle_count = sim
                .grid()
                .cells()
                .filter(|&pos| sim.grid().kind_at(pos) == CellKind::Obstacle)
                .count();
            if level == 0 {
                assert_eq!(obstacle_count, 0, "Level 0 is the original obstacle-free map");
            } else {
                assert!(obstacle_count > 0, "Level {level} should have Obstacle walls forcing turns");
            }
        }
    }

    #[test]
    fn clearing_the_last_wave_on_a_non_final_level_advances_the_level_instead_of_ending_the_game() {
        let mut sim = Simulation::new();
        assert_eq!(sim.level_number(), 1);
        sim.gold = 100_000;
        let pos = CellPos::new(1, 1);
        sim.place_tower(pos, TowerKind::Cannon).unwrap();
        let gold_before_clear = sim.gold();

        while sim.wave_number() < TOTAL_WAVES {
            sim.start_next_wave().unwrap();
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
        }
        sim.start_next_wave().unwrap();
        sim.spawn_queue.clear();
        sim.enemies.clear();
        let events = sim.tick(0.001);

        assert!(events.contains(&SimEvent::LevelCleared(1)));
        assert!(!events.iter().any(|e| matches!(e, SimEvent::Victory | SimEvent::Defeat)));
        assert_eq!(sim.outcome(), None);
        assert_eq!(sim.level_number(), 2);
        assert_eq!(sim.wave_number(), 1);
        assert!(!sim.has_tower(pos), "advancing a Level should clear every placed Tower");
        assert!(sim.gold() > gold_before_clear, "the cleared Tower should be sold for its refund");
    }

    #[test]
    fn clearing_the_last_wave_on_the_final_level_still_emits_victory() {
        let mut sim = Simulation::new_at_level(LEVEL_COUNT - 1);
        while sim.wave_number() < TOTAL_WAVES {
            sim.start_next_wave().unwrap();
            sim.spawn_queue.clear();
            sim.enemies.clear();
            sim.tick(0.001);
        }
        sim.start_next_wave().unwrap();
        sim.spawn_queue.clear();
        sim.enemies.clear();
        let events = sim.tick(0.001);

        assert!(events.contains(&SimEvent::Victory));
        assert!(!events.iter().any(|e| matches!(e, SimEvent::LevelCleared(_))));
        assert_eq!(sim.outcome(), Some(GameOutcome::Victory));
    }
}
