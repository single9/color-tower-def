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

/// Owns the Grid and every placed Tower, and enforces the Blocking Rule.
///
/// Ticket 02 only needs a single Tower Kind, so a Tower is represented
/// as bare occupancy (a `CellPos` with nothing else attached) for now;
/// Tower Kind/Tier land in later tickets.
#[derive(Debug, Clone)]
pub struct Simulation {
    grid: Grid,
    towers: HashSet<CellPos>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            grid: Grid::new(),
            towers: HashSet::new(),
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
        self.shortest_path(None)
    }

    /// What the shortest Path from Spawn to Goal would be if a Tower
    /// were additionally placed at `pos`. Used for the pre-placement
    /// preview; does not mutate any state.
    pub fn preview_path_if_placed(&self, pos: CellPos) -> Option<Vec<CellPos>> {
        self.shortest_path(Some(pos))
    }

    /// Whether a Tower could be placed at `pos` right now, and if not, why.
    pub fn can_place(&self, pos: CellPos) -> Result<(), PlacementError> {
        if self.grid.kind_at(pos) != CellKind::Buildable {
            return Err(PlacementError::NotBuildable);
        }
        if self.towers.contains(&pos) {
            return Err(PlacementError::AlreadyOccupied);
        }
        if self.shortest_path(Some(pos)).is_none() {
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

    /// BFS from Spawn to Goal, treating every placed Tower — plus
    /// `extra_blocked`, if given — as impassable.
    fn shortest_path(&self, extra_blocked: Option<CellPos>) -> Option<Vec<CellPos>> {
        let spawn = self.grid.spawn();
        let goal = self.grid.goal();
        let is_blocked = |pos: CellPos| Some(pos) == extra_blocked || self.towers.contains(&pos);

        if is_blocked(spawn) || is_blocked(goal) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut came_from: HashMap<CellPos, CellPos> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(spawn);
        queue.push_back(spawn);

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
}
