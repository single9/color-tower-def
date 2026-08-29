//! Bevy-independent core game rules for the tower defense game.
//!
//! Ticket 01 only needs the `Grid`/`Cell`/`CellKind` shape so the Bevy
//! layer has something real to render. Tower/Enemy/Wave/etc. land in
//! later tickets.

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
}
