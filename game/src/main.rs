use bevy::color::palettes::css;
use bevy::prelude::*;
use simulation::{CellKind, CellPos, Grid, Simulation, CELL_SIZE_PX, GRID_SIZE};

const GRID_PX: f32 = CELL_SIZE_PX * GRID_SIZE as f32; // 500.0
const SIDEBAR_PX: f32 = 200.0;
const WINDOW_WIDTH: f32 = GRID_PX + SIDEBAR_PX; // 700.0
const WINDOW_HEIGHT: f32 = GRID_PX; // 500.0

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tower Defense".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(SimState(Simulation::new()))
        .add_systems(
            Startup,
            (spawn_camera, spawn_grid, spawn_sidebar, spawn_enemy),
        )
        .add_systems(Update, (interact_with_grid, move_enemy))
        .run();
}

/// The Bevy-independent game rules, owned as a resource. This is the
/// one seam the Bevy layer talks to for all placement/Path logic.
#[derive(Resource)]
struct SimState(Simulation);

/// Marks the Tower sprite placed at a given Cell, so it can be found
/// again on sell.
#[derive(Component)]
struct TowerAt(CellPos);

/// Marks a translucent Path-preview overlay tile, so all of them can
/// be cleared before redrawing each frame.
#[derive(Component)]
struct PathPreviewTile;

/// Marks the sprite entity representing the live Enemy. Ticket 03
/// only has a single Enemy on screen at once.
#[derive(Component)]
struct EnemyMarker;

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// World position of a Cell's center. The grid occupies the left
/// `GRID_PX` x `GRID_PX` square of the window; the origin (0,0) in
/// Bevy 2D world space is the center of the window.
fn cell_world_pos(pos: CellPos) -> Vec3 {
    let left_edge = -WINDOW_WIDTH / 2.0;
    let bottom_edge = -WINDOW_HEIGHT / 2.0;
    let x = left_edge + pos.x as f32 * CELL_SIZE_PX + CELL_SIZE_PX / 2.0;
    let y = bottom_edge + pos.y as f32 * CELL_SIZE_PX + CELL_SIZE_PX / 2.0;
    Vec3::new(x, y, 0.0)
}

/// Inverse of `cell_world_pos`: which Cell (if any) a world position
/// falls inside. Returns `None` for anything outside the Grid's
/// GRID_PX x GRID_PX square (e.g. the sidebar).
fn world_to_cell(world: Vec2) -> Option<CellPos> {
    let left_edge = -WINDOW_WIDTH / 2.0;
    let bottom_edge = -WINDOW_HEIGHT / 2.0;
    let rel_x = world.x - left_edge;
    let rel_y = world.y - bottom_edge;
    if rel_x < 0.0 || rel_y < 0.0 || rel_x >= GRID_PX || rel_y >= GRID_PX {
        return None;
    }
    Some(CellPos::new(
        (rel_x / CELL_SIZE_PX) as usize,
        (rel_y / CELL_SIZE_PX) as usize,
    ))
}

fn tower_color() -> Color {
    Color::Srgba(css::RED) // Cannon; other Tower Kind land in ticket 05
}

fn path_preview_color() -> Color {
    Color::srgba(1.0, 1.0, 0.0, 0.35) // translucent yellow
}

fn enemy_color() -> Color {
    Color::Srgba(css::LIME) // Grunt; other Enemy Kind land in ticket 05
}

fn cell_color(kind: CellKind) -> Color {
    match kind {
        CellKind::Buildable => Color::srgb(0.82, 0.82, 0.82), // light gray
        CellKind::Spawn => Color::Srgba(css::INDIGO),         // dark purple
        CellKind::Goal => Color::Srgba(css::ORANGE),
    }
}

fn spawn_grid(mut commands: Commands) {
    let grid = Grid::new();
    for pos in grid.cells() {
        let kind = grid.kind_at(pos);
        commands.spawn((
            Sprite {
                color: cell_color(kind),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(pos)),
        ));
    }
}

fn spawn_sidebar(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Px(SIDEBAR_PX),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .insert(BackgroundColor(Color::srgb(0.12, 0.12, 0.14)))
        .with_children(|sidebar| {
            sidebar.spawn((
                Text::new("Gold: -"),
                TextColor(Color::WHITE),
            ));
            sidebar.spawn((
                Text::new("Lives: -"),
                TextColor(Color::WHITE),
            ));
            sidebar.spawn((
                Text::new("Wave: -"),
                TextColor(Color::WHITE),
            ));

            sidebar.spawn(Node {
                height: Val::Px(16.0),
                ..default()
            });

            for label in ["Cannon", "Gatling", "Frost"] {
                sidebar
                    .spawn((
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                    ))
                    .with_children(|button| {
                        button.spawn((Text::new(label), TextColor(Color::WHITE)));
                    });
            }
        });
}

/// Single seam between mouse input and the `simulation` crate: figures
/// out which Cell the cursor is over, redraws the Path preview for it,
/// and on a left click either sells an existing Tower or places a new
/// one (subject to the Blocking Rule enforced entirely inside `sim`).
fn interact_with_grid(
    mut commands: Commands,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut sim: ResMut<SimState>,
    towers: Query<(Entity, &TowerAt)>,
    preview_tiles: Query<Entity, With<PathPreviewTile>>,
) {
    for entity in &preview_tiles {
        commands.entity(entity).despawn();
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.get_single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    let Some(hovered) = world_to_cell(world) else {
        return;
    };

    if !sim.0.has_tower(hovered) && sim.0.grid().kind_at(hovered) == CellKind::Buildable {
        if let Some(path) = sim.0.preview_path_if_placed(hovered) {
            for pos in path {
                commands.spawn((
                    Sprite {
                        color: path_preview_color(),
                        custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                        ..default()
                    },
                    Transform::from_translation(cell_world_pos(pos).with_z(0.5)),
                    PathPreviewTile,
                ));
            }
        }
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    if sim.0.has_tower(hovered) {
        sim.0.sell_tower(hovered);
        for (entity, tower_at) in &towers {
            if tower_at.0 == hovered {
                commands.entity(entity).despawn();
            }
        }
    } else if sim.0.place_tower(hovered).is_ok() {
        commands.spawn((
            Sprite {
                color: tower_color(),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(hovered).with_z(1.0)),
            TowerAt(hovered),
        ));
    }
}

fn spawn_enemy(mut commands: Commands, mut sim: ResMut<SimState>) {
    sim.0.spawn_enemy();
    let Some(transit) = sim.0.enemy_transit() else {
        return;
    };
    commands.spawn((
        Sprite {
            color: enemy_color(),
            custom_size: Some(Vec2::splat(CELL_SIZE_PX * 0.6)),
            ..default()
        },
        Transform::from_translation(cell_world_pos(transit.from).with_z(2.0)),
        EnemyMarker,
    ));
}

/// Advances the `simulation` Enemy and mirrors its position onto the
/// Bevy sprite, interpolating between the two Cell centers of its
/// current transit segment. Despawns the sprite once the Enemy
/// reaches Goal and `simulation` drops it.
fn move_enemy(
    time: Res<Time>,
    mut sim: ResMut<SimState>,
    mut commands: Commands,
    mut enemy_sprite: Query<(Entity, &mut Transform), With<EnemyMarker>>,
) {
    sim.0.tick(time.delta_secs());

    let Ok((entity, mut transform)) = enemy_sprite.get_single_mut() else {
        return;
    };

    match sim.0.enemy_transit() {
        Some(transit) => {
            let from = cell_world_pos(transit.from);
            let to = cell_world_pos(transit.to);
            transform.translation = from.lerp(to, transit.progress).with_z(2.0);
        }
        None => commands.entity(entity).despawn(),
    }
}
