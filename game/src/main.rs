use bevy::color::palettes::css;
use bevy::prelude::*;
use simulation::{CellKind, CellPos, Grid, CELL_SIZE_PX, GRID_SIZE};

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
        .add_systems(Startup, (spawn_camera, spawn_grid, spawn_sidebar))
        .run();
}

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
