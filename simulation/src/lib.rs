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
    /// Placing here would leave no Path from Spawn to Goal (the Blocking Rule).
    WouldBlockPath,
}

/// How many Cell-widths a Grunt-stat Enemy crosses per second. A
/// placeholder value pending playtesting; Enemy Kind-specific speeds
/// land in ticket 05.
const ENEMY_SPEED_CELLS_PER_SEC: f32 = 2.0;

/// A single Enemy in transit between two Cell centers. `target` is
/// `None` only in the rare case its onward Path vanished entirely
/// (see `Simulation::tick`).
#[derive(Debug, Clone, Copy)]
struct Enemy {
    at: CellPos,
    target: Option<CellPos>,
    progress: f32,
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
    towers: HashSet<CellPos>,
    enemy: Option<Enemy>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            grid: Grid::new(),
            towers: HashSet::new(),
            enemy: None,
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn has_tower(&self, pos: CellPos) -> bool {
        self.towers.contains(&pos)
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

    /// Whether a Tower could be placed at `pos` right now, and if not, why.
    pub fn can_place(&self, pos: CellPos) -> Result<(), PlacementError> {
        if self.grid.kind_at(pos) != CellKind::Buildable {
            return Err(PlacementError::NotBuildable);
        }
        if self.towers.contains(&pos) {
            return Err(PlacementError::AlreadyOccupied);
        }
        if self.shortest_path(self.grid.spawn(), Some(pos)).is_none() {
            return Err(PlacementError::WouldBlockPath);
        }
        Ok(())
    }

    pub fn place_tower(&mut self, pos: CellPos) -> Result<(), PlacementError> {
        self.can_place(pos)?;
        self.towers.insert(pos);
        Ok(())
    }

    /// Removes the Tower at `pos`, if any. Returns whether a Tower was there.
    pub fn sell_tower(&mut self, pos: CellPos) -> bool {
        self.towers.remove(&pos)
    }

    /// Spawns one Grunt-stat Enemy at Spawn, replacing any Enemy
    /// already present. Ticket 03 only needs a single Enemy; Wave
    /// spawning of many at once lands in ticket 08.
    pub fn spawn_enemy(&mut self) {
        let spawn = self.grid.spawn();
        let path = self.shortest_path(spawn, None);
        self.enemy = Some(Enemy {
            at: spawn,
            target: path.and_then(|p| p.get(1).copied()),
            progress: 0.0,
        });
    }

    pub fn enemy_alive(&self) -> bool {
        self.enemy.is_some()
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

    /// Advances the live Enemy by `dt` seconds. Per ADR-0002, the
    /// Enemy's Path is only ever recomputed the instant it reaches a
    /// Cell center — never mid-transit, no matter how the Grid changes
    /// underneath it in the meantime.
    pub fn tick(&mut self, dt: f32) {
        let Some((at, target, mut progress)) = self
            .enemy
            .as_ref()
            .map(|enemy| (enemy.at, enemy.target, enemy.progress))
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

        progress += dt * ENEMY_SPEED_CELLS_PER_SEC;
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

    /// BFS from `from` to Goal, treating every placed Tower — plus
    /// `extra_blocked`, if given — as impassable. `from` need not be
    /// Spawn: each Enemy recomputes its own remaining Path from
    /// wherever it currently stands (see ADR-0002).
    fn shortest_path(&self, from: CellPos, extra_blocked: Option<CellPos>) -> Option<Vec<CellPos>> {
        let goal = self.grid.goal();
        let is_blocked = |pos: CellPos| Some(pos) == extra_blocked || self.towers.contains(&pos);

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
        assert!(sim.place_tower(CellPos::new(5, 5)).is_ok());
        assert!(sim.has_tower(CellPos::new(5, 5)));
    }

    #[test]
    fn placing_on_spawn_or_goal_is_rejected() {
        let mut sim = Simulation::new();
        assert_eq!(
            sim.place_tower(sim.grid().spawn()),
            Err(PlacementError::NotBuildable)
        );
        assert_eq!(
            sim.place_tower(sim.grid().goal()),
            Err(PlacementError::NotBuildable)
        );
    }

    #[test]
    fn placing_on_an_already_occupied_cell_is_rejected() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos).unwrap();
        assert_eq!(
            sim.place_tower(pos),
            Err(PlacementError::AlreadyOccupied)
        );
    }

    #[test]
    fn sealing_the_entire_maze_is_rejected_but_a_single_gap_stays_valid() {
        let mut sim = Simulation::new();

        // Wall off the whole column x=1 except one gap at y=24: Spawn
        // (x=0) can only reach the rest of the grid through column 1.
        for y in 0..GRID_SIZE - 1 {
            sim.place_tower(CellPos::new(1, y))
                .expect("leaving a gap open should keep placement valid");
        }

        // A narrow path through the single remaining gap must still exist.
        let path = sim.current_path().expect("a narrow path should remain");
        assert!(path.contains(&CellPos::new(1, GRID_SIZE - 1)));

        // Sealing the last gap would cut Spawn off from Goal entirely.
        let last_gap = CellPos::new(1, GRID_SIZE - 1);
        assert_eq!(
            sim.place_tower(last_gap),
            Err(PlacementError::WouldBlockPath)
        );
        assert!(!sim.has_tower(last_gap));
    }

    #[test]
    fn selling_a_tower_frees_the_cell() {
        let mut sim = Simulation::new();
        let pos = CellPos::new(5, 5);
        sim.place_tower(pos).unwrap();

        assert!(sim.sell_tower(pos));
        assert!(!sim.has_tower(pos));

        // Selling an empty cell is a no-op that reports nothing was there.
        assert!(!sim.sell_tower(pos));
    }

    #[test]
    fn enemy_spawns_at_spawn_heading_toward_goal() {
        let mut sim = Simulation::new();
        sim.spawn_enemy();
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
        sim.spawn_enemy();

        // Advance partway into the first Cell — not far enough to reach its center.
        sim.tick(0.1);
        let before = sim.enemy_transit().unwrap();
        assert_eq!(before.to, CellPos::new(1, 12));
        assert!(before.progress > 0.0 && before.progress < 1.0);

        // A Grid mutation happening mid-transit must not retarget the Enemy.
        sim.place_tower(CellPos::new(5, 12))
            .expect("placing off to the side of Spawn should stay legal");
        let after = sim.enemy_transit().unwrap();
        assert_eq!(after.to, before.to);
        assert_eq!(after.progress, before.progress);
    }

    #[test]
    fn enemy_recomputes_and_picks_up_a_changed_grid_on_reaching_a_cell_center() {
        let mut sim = Simulation::new();
        sim.spawn_enemy();

        // Cross the full first Cell: Spawn -> (1,12), recomputing there
        // onto the still-open straight row, so target becomes (2,12).
        sim.tick(1.0 / ENEMY_SPEED_CELLS_PER_SEC);
        let at_first_center = sim.enemy_transit().unwrap();
        assert_eq!(at_first_center.from, CellPos::new(1, 12));
        assert_eq!(at_first_center.to, CellPos::new(2, 12));

        // Block the cell the Enemy was about to walk into next, *after*
        // it already committed to heading toward (2,12).
        sim.place_tower(CellPos::new(3, 12))
            .expect("blocking one cell should still leave a detour");

        // Cross into (2,12): this is the recompute point.
        sim.tick(1.0 / ENEMY_SPEED_CELLS_PER_SEC);
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
        sim.spawn_enemy();

        let steps_to_goal = sim.grid().goal().x - sim.grid().spawn().x;
        for _ in 0..steps_to_goal {
            sim.tick(1.0 / ENEMY_SPEED_CELLS_PER_SEC);
        }

        assert!(!sim.enemy_alive());
        assert!(sim.enemy_transit().is_none());
    }
}
