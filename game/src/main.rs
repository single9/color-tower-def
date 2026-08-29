use bevy::color::palettes::css;
use bevy::prelude::*;
use simulation::{
    CellKind, CellPos, EnemyKind, GameOutcome, Grid, Simulation, TowerKind, TowerTier,
    CELL_SIZE_PX, GRID_SIZE,
};

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
        .insert_resource(SelectedTowerKind(TowerKind::Cannon))
        .insert_resource(SelectedTower(None))
        .add_systems(
            Startup,
            (spawn_camera, spawn_grid, spawn_sidebar, spawn_result_overlay),
        )
        .add_systems(
            Update,
            (
                select_tower_kind,
                interact_with_grid,
                handle_tower_panel_buttons,
                sync_tower_info_panel,
                handle_wave_button,
                handle_reset_button,
                tick_simulation,
                sync_enemies,
                sync_projectiles,
                sync_gold_text,
                sync_lives_text,
                sync_wave_ui,
                sync_result_overlay,
            )
                .chain(),
        )
        .run();
}

/// The Bevy-independent game rules, owned as a resource. This is the
/// one seam the Bevy layer talks to for all placement/Path logic.
#[derive(Resource)]
struct SimState(Simulation);

/// Which Tower Kind the next Cell click will place, chosen via the
/// sidebar's Tower Kind buttons.
#[derive(Resource)]
struct SelectedTowerKind(TowerKind);

/// Marks a sidebar Tower Kind selection button with the Kind it
/// selects, so `select_tower_kind` can tell them apart and
/// re-highlight whichever is currently selected.
#[derive(Component)]
struct TowerKindButton(TowerKind);

/// Marks the Tower sprite placed at a given Cell, so it can be found
/// again on sell.
#[derive(Component)]
struct TowerAt(CellPos);

/// Marks a translucent Path-preview overlay tile, so all of them can
/// be cleared before redrawing each frame.
#[derive(Component)]
struct PathPreviewTile;

/// Marks a live Enemy sprite, cleared and redrawn from `simulation`
/// state every frame (mirrors `PathPreviewTile`'s approach).
#[derive(Component)]
struct EnemyMarker;

/// Marks a Projectile sprite, cleared and redrawn from `simulation`
/// state every frame (mirrors `PathPreviewTile`'s approach).
#[derive(Component)]
struct ProjectileMarker;

/// Marks the sidebar's live Gold readout text.
#[derive(Component)]
struct GoldText;

/// Marks the sidebar's live Lives readout text.
#[derive(Component)]
struct LivesText;

/// Marks the sidebar's live Wave readout text.
#[derive(Component)]
struct WaveText;

/// The sidebar's "Start Next Wave" button.
#[derive(Component)]
struct WaveButton;

/// The Tower currently selected for inspection (clicked on the Grid),
/// whose info panel is showing in the sidebar. `None` when nothing is
/// selected.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct SelectedTower(Option<CellPos>);

/// Empty container the sidebar's Tower info panel is rebuilt into
/// whenever `SelectedTower` (or its Tower's Tier) changes.
#[derive(Component)]
struct InfoPanelRoot;

/// The Upgrade button inside the Tower info panel.
#[derive(Component)]
struct UpgradeButton;

/// The Sell button inside the Tower info panel.
#[derive(Component)]
struct SellButton;

/// Full-window container the Victory/Defeat result overlay is
/// rebuilt into whenever `simulation`'s outcome changes.
#[derive(Component)]
struct ResultOverlayRoot;

/// The result overlay's "Reset" button.
#[derive(Component)]
struct ResetButton;

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// World position for a point given in Cell units (fractional units
/// allowed, e.g. a Projectile's `simulation`-side position). The grid
/// occupies the left `GRID_PX` x `GRID_PX` square of the window; the
/// origin (0,0) in Bevy 2D world space is the center of the window.
fn cell_units_to_world(x: f32, y: f32) -> Vec3 {
    let left_edge = -WINDOW_WIDTH / 2.0;
    let bottom_edge = -WINDOW_HEIGHT / 2.0;
    Vec3::new(
        left_edge + x * CELL_SIZE_PX + CELL_SIZE_PX / 2.0,
        bottom_edge + y * CELL_SIZE_PX + CELL_SIZE_PX / 2.0,
        0.0,
    )
}

/// World position of a Cell's center.
fn cell_world_pos(pos: CellPos) -> Vec3 {
    cell_units_to_world(pos.x as f32, pos.y as f32)
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

fn tower_color(kind: TowerKind) -> Color {
    match kind {
        TowerKind::Cannon => Color::Srgba(css::RED),
        TowerKind::Gatling => Color::Srgba(css::GREEN),
        TowerKind::Frost => Color::Srgba(css::BLUE),
    }
}

fn path_preview_color() -> Color {
    Color::srgba(1.0, 1.0, 0.0, 0.35) // translucent yellow
}

fn enemy_color(kind: EnemyKind) -> Color {
    match kind {
        EnemyKind::Grunt => Color::Srgba(css::LIME),
        EnemyKind::Runner => Color::Srgba(css::YELLOW),
        EnemyKind::Tank => Color::Srgba(css::SIENNA),
    }
}

fn button_color(selected: bool) -> Color {
    if selected {
        Color::srgb(0.45, 0.45, 0.55)
    } else {
        Color::srgb(0.25, 0.25, 0.3)
    }
}

fn projectile_color() -> Color {
    Color::Srgba(css::WHITE)
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

fn spawn_sidebar(mut commands: Commands, sim: Res<SimState>) {
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
                Text::new(format!("Gold: {}", sim.0.gold())),
                TextColor(Color::WHITE),
                GoldText,
            ));
            sidebar.spawn((
                Text::new(format!("Lives: {}", sim.0.lives())),
                TextColor(Color::WHITE),
                LivesText,
            ));
            sidebar.spawn((
                Text::new(format!("Wave: {}", sim.0.wave_number())),
                TextColor(Color::WHITE),
                WaveText,
            ));

            sidebar
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.35, 0.2)),
                    WaveButton,
                ))
                .with_children(|button| {
                    button.spawn((Text::new("Start Next Wave"), TextColor(Color::WHITE)));
                });

            sidebar.spawn(Node {
                height: Val::Px(16.0),
                ..default()
            });

            for (label, kind) in [
                ("Cannon", TowerKind::Cannon),
                ("Gatling", TowerKind::Gatling),
                ("Frost", TowerKind::Frost),
            ] {
                sidebar
                    .spawn((
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(button_color(kind == TowerKind::Cannon)),
                        TowerKindButton(kind),
                    ))
                    .with_children(|button| {
                        button.spawn((Text::new(label), TextColor(Color::WHITE)));
                    });
            }

            sidebar.spawn(Node {
                height: Val::Px(16.0),
                ..default()
            });

            sidebar.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                InfoPanelRoot,
            ));
        });
}

/// Handles clicks on the sidebar's Tower Kind buttons: updates which
/// Kind `interact_with_grid` will place next, and re-highlights the
/// buttons to reflect the current selection.
fn select_tower_kind(
    mut selected: ResMut<SelectedTowerKind>,
    interactions: Query<(&Interaction, &TowerKindButton), Changed<Interaction>>,
    mut buttons: Query<(&TowerKindButton, &mut BackgroundColor)>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = button.0;
        }
    }
    for (button, mut background) in &mut buttons {
        *background = BackgroundColor(button_color(button.0 == selected.0));
    }
}

/// Despawns the Tower sprite at `pos`, if any is currently tagged
/// with it. Shared by selling from the info panel and (in earlier
/// tickets) from a direct Grid click.
fn despawn_tower_sprite(commands: &mut Commands, towers: &Query<(Entity, &TowerAt)>, pos: CellPos) {
    for (entity, tower_at) in towers {
        if tower_at.0 == pos {
            commands.entity(entity).despawn();
        }
    }
}

/// Single seam between mouse input and the `simulation` crate: figures
/// out which Cell the cursor is over, redraws the Path preview for it,
/// and on a left click either selects an existing Tower (opening its
/// info panel) or places a new one (subject to the Blocking Rule
/// enforced entirely inside `sim`).
fn interact_with_grid(
    mut commands: Commands,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut sim: ResMut<SimState>,
    selected: Res<SelectedTowerKind>,
    mut selected_tower: ResMut<SelectedTower>,
    preview_tiles: Query<Entity, With<PathPreviewTile>>,
) {
    for entity in &preview_tiles {
        commands.entity(entity).despawn();
    }

    if sim.0.outcome().is_some() {
        // The result overlay is showing: no Grid click has any effect.
        return;
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
        selected_tower.0 = Some(hovered);
    } else if sim.0.place_tower(hovered, selected.0).is_ok() {
        commands.spawn((
            Sprite {
                color: tower_color(selected.0),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(hovered).with_z(1.0)),
            TowerAt(hovered),
        ));
        selected_tower.0 = None;
    }
}

/// Handles clicks on the info panel's Upgrade/Sell buttons for
/// whichever Tower `SelectedTower` currently points at. Failed
/// attempts (already Tier 3, unaffordable) are simply no-ops, mirroring
/// how `interact_with_grid` already treats a failed placement.
fn handle_tower_panel_buttons(
    mut commands: Commands,
    mut sim: ResMut<SimState>,
    mut selected_tower: ResMut<SelectedTower>,
    towers: Query<(Entity, &TowerAt)>,
    upgrade_interactions: Query<&Interaction, (With<UpgradeButton>, Changed<Interaction>)>,
    sell_interactions: Query<&Interaction, (With<SellButton>, Changed<Interaction>)>,
) {
    let Some(pos) = selected_tower.0 else {
        return;
    };

    for interaction in &upgrade_interactions {
        if *interaction == Interaction::Pressed {
            let _ = sim.0.upgrade_tower(pos);
        }
    }

    for interaction in &sell_interactions {
        if *interaction == Interaction::Pressed && sim.0.sell_tower(pos) {
            despawn_tower_sprite(&mut commands, &towers, pos);
            selected_tower.0 = None;
        }
    }
}

/// Rebuilds the sidebar's Tower info panel whenever `SelectedTower` or
/// its Tower's Tier changes (tracked via `last_rendered`, since a
/// naive every-frame rebuild would reset the Upgrade/Sell buttons'
/// `Interaction` state before a click could ever register).
fn sync_tower_info_panel(
    mut commands: Commands,
    selected_tower: Res<SelectedTower>,
    sim: Res<SimState>,
    panel_root: Query<Entity, With<InfoPanelRoot>>,
    children: Query<&Children>,
    mut last_rendered: Local<Option<(CellPos, TowerTier)>>,
) {
    let Ok(root) = panel_root.get_single() else {
        return;
    };

    let current = selected_tower
        .0
        .and_then(|pos| sim.0.tower_stats_at(pos).map(|stats| (pos, stats.tier)));

    if current == *last_rendered {
        return;
    }
    *last_rendered = current;

    if let Ok(existing_children) = children.get(root) {
        for &child in existing_children {
            commands.entity(child).despawn_recursive();
        }
    }

    let Some((pos, _)) = current else {
        return;
    };
    let stats = sim.0.tower_stats_at(pos).expect("just confirmed present above");

    commands.entity(root).with_children(|panel| {
        let kind_label = match stats.kind {
            TowerKind::Cannon => "Cannon",
            TowerKind::Gatling => "Gatling",
            TowerKind::Frost => "Frost",
        };
        let tier_label = match stats.tier {
            TowerTier::One => "Tier 1",
            TowerTier::Two => "Tier 2",
            TowerTier::Three => "Tier 3",
        };
        panel.spawn((
            Text::new(format!("{kind_label} ({tier_label})")),
            TextColor(Color::WHITE),
        ));
        if stats.kind == TowerKind::Frost {
            panel.spawn((
                Text::new(format!("Range: {:.1}", stats.range)),
                TextColor(Color::WHITE),
            ));
        } else {
            panel.spawn((
                Text::new(format!("Damage: {:.0}", stats.damage)),
                TextColor(Color::WHITE),
            ));
        }

        if let Some(cost) = sim.0.upgrade_cost_at(pos) {
            panel
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                    UpgradeButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(format!("Upgrade ({cost}g)")),
                        TextColor(Color::WHITE),
                    ));
                });
        }

        panel
            .spawn((
                Button,
                Node {
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.35, 0.2, 0.2)),
                SellButton,
            ))
            .with_children(|button| {
                button.spawn((Text::new("Sell"), TextColor(Color::WHITE)));
            });
    });
}

/// Advances `simulation` by one frame: Wave spawning, Enemy movement,
/// Tower firing, Projectile flight/impact, Wave-completion bookkeeping.
/// Kept separate from rendering so `sync_enemies`/`sync_projectiles`
/// read post-tick state, mirroring how `sync_projectiles` already
/// depended on `move_enemy` ticking first in earlier tickets.
fn tick_simulation(time: Res<Time>, mut sim: ResMut<SimState>) {
    sim.0.tick(time.delta_secs());
}

/// Redraws every live Enemy from `simulation` state, mirroring the
/// Path-preview/Projectile approach: clear last frame's sprites,
/// respawn fresh ones at this frame's positions and Kind's color.
fn sync_enemies(
    mut commands: Commands,
    sim: Res<SimState>,
    existing: Query<Entity, With<EnemyMarker>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (kind, transit) in sim.0.enemies_transits() {
        let from = cell_world_pos(transit.from);
        let to = cell_world_pos(transit.to);
        commands.spawn((
            Sprite {
                color: enemy_color(kind),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX * 0.6)),
                ..default()
            },
            Transform::from_translation(from.lerp(to, transit.progress).with_z(2.0)),
            EnemyMarker,
        ));
    }
}

/// Starts the next Wave when the sidebar's button is pressed. A press
/// while a Wave is still in progress is a no-op, mirroring how a
/// failed placement or upgrade is already just ignored elsewhere.
fn handle_wave_button(
    mut sim: ResMut<SimState>,
    interactions: Query<&Interaction, (With<WaveButton>, Changed<Interaction>)>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            let _ = sim.0.start_next_wave();
        }
    }
}

/// Redraws every in-flight Projectile from `simulation` state, mirroring
/// the Path-preview approach: clear last frame's sprites, respawn fresh
/// ones at this frame's positions. `simulation::Simulation::tick`
/// (called by `move_enemy`, which runs first in this chain) already
/// owns all Tower-firing and Projectile-flight/impact logic.
fn sync_projectiles(
    mut commands: Commands,
    sim: Res<SimState>,
    existing: Query<Entity, With<ProjectileMarker>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (x, y) in sim.0.projectile_positions() {
        commands.spawn((
            Sprite {
                color: projectile_color(),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX * 0.3)),
                ..default()
            },
            Transform::from_translation(cell_units_to_world(x, y).with_z(1.5)),
            ProjectileMarker,
        ));
    }
}

/// Keeps the sidebar's Lives readout in sync with `simulation` state.
fn sync_lives_text(sim: Res<SimState>, mut text: Query<&mut Text, With<LivesText>>) {
    let Ok(mut text) = text.get_single_mut() else {
        return;
    };
    text.0 = format!("Lives: {}", sim.0.lives());
}

/// Keeps the sidebar's Gold readout in sync with `simulation` state.
fn sync_gold_text(sim: Res<SimState>, mut text: Query<&mut Text, With<GoldText>>) {
    let Ok(mut text) = text.get_single_mut() else {
        return;
    };
    text.0 = format!("Gold: {}", sim.0.gold());
}

/// Keeps the sidebar's Wave readout and "Start Next Wave" button in
/// sync with `simulation` state: the button dims while a Wave is
/// still in progress, since pressing it then is a no-op.
fn sync_wave_ui(
    sim: Res<SimState>,
    mut text: Query<&mut Text, With<WaveText>>,
    mut button: Query<&mut BackgroundColor, With<WaveButton>>,
) {
    if let Ok(mut text) = text.get_single_mut() {
        text.0 = format!("Wave: {}", sim.0.wave_number());
    }
    if let Ok(mut background) = button.get_single_mut() {
        *background = BackgroundColor(if sim.0.wave_in_progress() {
            Color::srgb(0.15, 0.2, 0.15)
        } else {
            Color::srgb(0.2, 0.35, 0.2)
        });
    }
}

/// Spawns the (initially empty) full-window container the result
/// overlay is rebuilt into once Victory or Defeat fires.
fn spawn_result_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ResultOverlayRoot,
    ));
}

/// Rebuilds the result overlay whenever `simulation`'s outcome
/// changes: empty (and transparent) while the game is in progress,
/// a dimming backdrop plus a Victory/Defeat panel and Reset button
/// the instant it ends.
fn sync_result_overlay(
    mut commands: Commands,
    sim: Res<SimState>,
    root: Query<Entity, With<ResultOverlayRoot>>,
    children: Query<&Children>,
    mut last_rendered: Local<Option<GameOutcome>>,
) {
    let Ok(root) = root.get_single() else {
        return;
    };

    let current = sim.0.outcome();
    if current == *last_rendered {
        return;
    }
    *last_rendered = current;

    if let Ok(existing_children) = children.get(root) {
        for &child in existing_children {
            commands.entity(child).despawn_recursive();
        }
    }

    let Some(outcome) = current else {
        commands.entity(root).remove::<BackgroundColor>();
        return;
    };

    commands
        .entity(root)
        .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)));
    commands.entity(root).with_children(|overlay| {
        overlay
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
            ))
            .with_children(|panel| {
                let (label, color) = match outcome {
                    GameOutcome::Victory => ("Victory!", Color::Srgba(css::LIME)),
                    GameOutcome::Defeat => ("Defeat", Color::Srgba(css::RED)),
                };
                panel.spawn((Text::new(label), TextColor(color)));
                panel
                    .spawn((
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                        ResetButton,
                    ))
                    .with_children(|button| {
                        button.spawn((Text::new("Reset"), TextColor(Color::WHITE)));
                    });
            });
    });
}

/// Resets to a completely fresh game, equivalent to a cold app
/// launch: a new `Simulation` (empty Grid, starting Gold/Lives, Wave
/// counter before Wave 1), plus every Tower/Enemy/Projectile sprite
/// and UI selection state cleared.
fn handle_reset_button(
    mut commands: Commands,
    mut sim: ResMut<SimState>,
    mut selected_tower: ResMut<SelectedTower>,
    mut selected_kind: ResMut<SelectedTowerKind>,
    interactions: Query<&Interaction, (With<ResetButton>, Changed<Interaction>)>,
    towers: Query<Entity, With<TowerAt>>,
    enemies: Query<Entity, With<EnemyMarker>>,
    projectiles: Query<Entity, With<ProjectileMarker>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sim.0 = Simulation::new();
            selected_tower.0 = None;
            selected_kind.0 = TowerKind::Cannon;
            for entity in &towers {
                commands.entity(entity).despawn();
            }
            for entity in &enemies {
                commands.entity(entity).despawn();
            }
            for entity in &projectiles {
                commands.entity(entity).despawn();
            }
        }
    }
}
