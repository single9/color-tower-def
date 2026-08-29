use bevy::color::palettes::css;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use simulation::{
    CellKind, CellPos, EnemyKind, GameOutcome, Simulation, TowerKind, TowerTier, CELL_SIZE_PX, GRID_SIZE,
};

/// Deployment version, injected by the GitHub Action at build time via
/// `DEPLOY_VERSION`. Falls back to "dev" for local `cargo run` builds.
const DEPLOY_VERSION: &str = match option_env!("DEPLOY_VERSION") {
    Some(version) => version,
    None => "dev",
};

const GRID_PX: f32 = CELL_SIZE_PX * GRID_SIZE as f32; // 500.0
const SIDEBAR_PX: f32 = 200.0;
const WINDOW_WIDTH: f32 = GRID_PX + SIDEBAR_PX; // 700.0
/// Extra window height beyond the Grid's, giving the sidebar vertical
/// room for its Tower info panel (Upgrade/Sell) without clipping.
const WINDOW_VERTICAL_PAD: f32 = 200.0;
const WINDOW_HEIGHT: f32 = GRID_PX + WINDOW_VERTICAL_PAD; // 700.0

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Tower Defense v{DEPLOY_VERSION}"),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(SimState(Simulation::new()))
        .insert_resource(SelectedTowerKind(TowerKind::Cannon))
        .insert_resource(SelectedTower(None))
        .insert_resource(CommandPalette::default())
        .insert_resource(PendingConfirm(None))
        .add_systems(
            Startup,
            (
                spawn_camera,
                spawn_grid,
                spawn_sidebar,
                spawn_result_overlay,
                spawn_command_palette,
                spawn_confirm_dialog,
            ),
        )
        .add_systems(
            Update,
            (
                select_tower_kind,
                interact_with_grid,
                draw_range_rings,
                handle_tower_panel_buttons,
                sync_tower_info_panel,
                handle_wave_button,
                handle_reset_button,
                toggle_command_palette,
                type_into_command_palette,
                sync_command_palette_ui,
                (handle_confirm_dialog, sync_confirm_dialog).chain(),
                tick_simulation,
                sync_enemies,
                sync_projectiles,
                sync_gold_text,
                sync_lives_text,
                sync_wave_ui,
                sync_level_text,
                sync_grid_cells,
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

/// Marks the sidebar's live Level readout text.
#[derive(Component)]
struct LevelText;

/// Marks a base Grid Cell sprite with its Cell, so `sync_grid_cells`
/// can repaint it when the Level (and so the Grid's Obstacle layout)
/// changes.
#[derive(Component)]
struct CellAt(CellPos);

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

/// State for the dev-only Command Palette (toggled with the Backquote
/// key): whether it's open, the in-progress input line, and the
/// result/help line shown under it. A debugging/playtesting aid for
/// reaching later Level/Wave/Gold states without grinding — see
/// `run_command` for the supported commands.
#[derive(Resource, Default)]
struct CommandPalette {
    open: bool,
    input: String,
    message: Option<String>,
}

/// The Command Palette's root Node, toggled between `Visibility::Visible`
/// and `Visibility::Hidden` to show/hide the whole bar.
#[derive(Component)]
struct CommandPaletteRoot;

/// The Command Palette's input line (`"> {input}_"`).
#[derive(Component)]
struct CommandPaletteInputText;

/// The Command Palette's help/result line, shown under the input line.
#[derive(Component)]
struct CommandPaletteMessageText;

/// A Tower action the player is being asked to confirm. Set by the info
/// panel's Upgrade/Sell buttons; drives the confirmation dialog.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfirmAction {
    Upgrade(CellPos),
    Sell(CellPos),
}

/// The Tower action awaiting confirmation, if any. `Some` shows the
/// confirmation dialog; `None` hides it.
#[derive(Resource, Default)]
struct PendingConfirm(Option<ConfirmAction>);

/// The confirmation dialog's root Node, toggled between `Visibility::Visible`
/// and `Visibility::Hidden` as `PendingConfirm` changes.
#[derive(Component)]
struct ConfirmDialogRoot;

/// The full-window dimming backdrop behind the confirmation dialog;
/// clicking it cancels the pending action.
#[derive(Component)]
struct ConfirmDialogBackdrop;

/// The dialog's "Confirm" button: commits the pending action.
#[derive(Component)]
struct ConfirmDialogConfirmButton;

/// The dialog's "Cancel" button: dismisses the pending action.
#[derive(Component)]
struct ConfirmDialogCancelButton;

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// World position for a point given in Cell units (fractional units
/// allowed, e.g. a Projectile's `simulation`-side position). The grid
/// occupies the left `GRID_PX` x `GRID_PX` square of the window; the
/// origin (0,0) in Bevy 2D world space is the center of the window.
fn cell_units_to_world(x: f32, y: f32) -> Vec3 {
    let left_edge = -WINDOW_WIDTH / 2.0;
    let bottom_edge = -GRID_PX / 2.0;
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
    let bottom_edge = -GRID_PX / 2.0;
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
        EnemyKind::Boss => Color::Srgba(css::PURPLE),
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
        CellKind::Obstacle => Color::srgb(0.1, 0.1, 0.12), // near-black wall
    }
}

fn spawn_grid(mut commands: Commands, sim: Res<SimState>) {
    for pos in sim.0.grid().cells() {
        let kind = sim.0.grid().kind_at(pos);
        commands.spawn((
            Sprite {
                color: cell_color(kind),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(pos)),
            CellAt(pos),
        ));
    }
}

/// Keeps every Cell sprite's color in sync with `simulation`'s Grid.
/// The Grid only ever changes shape when the Level advances (see
/// `SimEvent::LevelCleared`), so this is gated on `sim.0.level_number()`
/// changing rather than repainting all `GRID_SIZE * GRID_SIZE` sprites
/// every frame.
fn sync_grid_cells(
    sim: Res<SimState>,
    mut cells: Query<(&CellAt, &mut Sprite)>,
    mut last_level: Local<Option<u32>>,
) {
    let current = sim.0.level_number();
    if *last_level == Some(current) {
        return;
    }
    *last_level = Some(current);

    for (cell, mut sprite) in &mut cells {
        sprite.color = cell_color(sim.0.grid().kind_at(cell.0));
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
                Text::new(format!("Wave: {}/{}", sim.0.wave_number(), simulation::TOTAL_WAVES)),
                TextColor(Color::WHITE),
                WaveText,
            ));
            sidebar.spawn((
                Text::new(format!("Level: {}/{}", sim.0.level_number(), simulation::LEVEL_COUNT)),
                TextColor(Color::WHITE),
                LevelText,
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
                        button.spawn((
                            Text::new(format!("{label} ({}g)", kind.price())),
                            TextColor(Color::WHITE),
                        ));
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

            sidebar.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });

            sidebar.spawn((
                Text::new(format!("v{DEPLOY_VERSION}")),
                TextColor(Color::srgb(0.5, 0.5, 0.55)),
            ));
        });
}

/// Handles clicks on the sidebar's Tower Kind buttons: updates which
/// Kind `interact_with_grid` will place next, and re-highlights the
/// buttons to reflect the current selection.
fn select_tower_kind(
    mut selected: ResMut<SelectedTowerKind>,
    mut selected_tower: ResMut<SelectedTower>,
    interactions: Query<(&Interaction, &TowerKindButton), Changed<Interaction>>,
    mut buttons: Query<(&TowerKindButton, &mut BackgroundColor)>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = button.0;
            // Picking a Kind to place next means the player is done
            // inspecting whatever Tower was selected — deselect it so
            // its info panel and Range ring don't linger over the
            // placement preview.
            selected_tower.0 = None;
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

/// Which Cell (if any) the cursor is currently hovering, shared by
/// `interact_with_grid` (Path preview / placement / selection) and
/// `draw_range_rings` (placement Range preview) so both apply the same
/// window/camera → world → Cell conversion.
fn hovered_cell(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform)>,
) -> Option<CellPos> {
    let window = windows.get_single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, camera_transform) = camera.get_single().ok()?;
    let world = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    world_to_cell(world)
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

    let Some(hovered) = hovered_cell(&windows, &camera) else {
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
        return;
    }

    // Any click on a Cell with no Tower deselects whatever was
    // previously selected — including a failed placement (e.g. the
    // Blocking Rule rejected it, or the Cell isn't Buildable) — so a
    // stray click off a Tower always clears its Range ring instead of
    // leaving it stuck showing.
    selected_tower.0 = None;
    if sim.0.place_tower(hovered, selected.0).is_ok() {
        commands.spawn((
            Sprite {
                color: tower_color(selected.0),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(hovered).with_z(1.0)),
            TowerAt(hovered),
        ));
    }
}

/// Draws a translucent Range ring so the player can see how far a
/// Tower's Range actually reaches in Cell units, rather than reading a
/// bare number off the info panel:
/// - a Tower selected for inspection shows its current (Tiered) Range;
/// - otherwise, hovering a Buildable Cell previews the selected Tower
///   Kind's Tier 1 Range for that placement.
/// Gizmos redraw every frame with no despawn bookkeeping needed, unlike
/// the Sprite-based overlays elsewhere in this file.
fn draw_range_rings(
    mut gizmos: Gizmos,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    sim: Res<SimState>,
    selected_tower: Res<SelectedTower>,
    selected_kind: Res<SelectedTowerKind>,
) {
    if let Some(pos) = selected_tower.0 {
        if let Some(stats) = sim.0.tower_stats_at(pos) {
            let center = cell_world_pos(pos).truncate();
            gizmos.circle_2d(center, stats.range * CELL_SIZE_PX, range_ring_color());
        }
        return;
    }

    if sim.0.outcome().is_some() {
        return;
    }

    let Some(hovered) = hovered_cell(&windows, &camera) else {
        return;
    };
    if sim.0.has_tower(hovered) || sim.0.grid().kind_at(hovered) != CellKind::Buildable {
        return;
    }

    let range = selected_kind.0.range(TowerTier::One);
    let center = cell_world_pos(hovered).truncate();
    gizmos.circle_2d(center, range * CELL_SIZE_PX, range_ring_color());
}

fn range_ring_color() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.55)
}

/// Handles clicks on the info panel's Upgrade/Sell buttons for
/// whichever Tower `SelectedTower` currently points at. Rather than
/// acting immediately, either press opens a confirmation dialog (see
/// `handle_confirm_dialog`) the player must explicitly commit.
fn handle_tower_panel_buttons(
    selected_tower: ResMut<SelectedTower>,
    upgrade_interactions: Query<&Interaction, (With<UpgradeButton>, Changed<Interaction>)>,
    sell_interactions: Query<&Interaction, (With<SellButton>, Changed<Interaction>)>,
    mut pending: ResMut<PendingConfirm>,
) {
    let Some(pos) = selected_tower.0 else {
        return;
    };

    for interaction in &upgrade_interactions {
        if *interaction == Interaction::Pressed {
            pending.0 = Some(ConfirmAction::Upgrade(pos));
        }
    }

    for interaction in &sell_interactions {
        if *interaction == Interaction::Pressed {
            pending.0 = Some(ConfirmAction::Sell(pos));
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
fn tick_simulation(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimState>,
    towers: Query<Entity, With<TowerAt>>,
) {
    let events = sim.0.tick(time.delta_secs());
    // A LevelCleared Wave-completion sells every placed Tower inside
    // `sim` (see SimEvent::LevelCleared) for the new Level's Grid, but
    // has no way to reach into Bevy to despawn the Sprites those Tower
    // used to have — do that here, the same way `run_command`'s `level`
    // Command Palette command and `handle_reset_button` already do.
    if events.iter().any(|event| matches!(event, simulation::SimEvent::LevelCleared(_))) {
        for entity in &towers {
            commands.entity(entity).despawn();
        }
    }
}

/// Redraws every live Enemy from `simulation` state, mirroring the
/// Path-preview/Projectile approach: clear last frame's sprites,
/// respawn fresh ones at this frame's positions and Kind's color.
///
/// Two Enemy occupying the same Cell (e.g. queued up behind each
/// other at a chokepoint) would otherwise share the exact same z,
/// leaving the GPU's draw order for that pair unstable frame to frame
/// since these sprites are despawned/respawned fresh every frame —
/// visible as flicker. Each Enemy's stable id (not its per-frame
/// Entity, which is fresh every time) offsets its z by a tiny,
/// consistent amount, so the same pair always draws in the same
/// relative order.
fn sync_enemies(
    mut commands: Commands,
    sim: Res<SimState>,
    existing: Query<Entity, With<EnemyMarker>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (id, kind, transit) in sim.0.enemies_transits() {
        let from = cell_world_pos(transit.from);
        let to = cell_world_pos(transit.to);
        let z = 2.0 + (id % 1000) as f32 * 0.0001;
        commands.spawn((
            Sprite {
                color: enemy_color(kind),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX * 0.6)),
                ..default()
            },
            Transform::from_translation(from.lerp(to, transit.progress).with_z(z)),
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
        text.0 = format!("Wave: {}/{}", sim.0.wave_number(), simulation::TOTAL_WAVES);
    }
    if let Ok(mut background) = button.get_single_mut() {
        *background = BackgroundColor(if sim.0.wave_in_progress() {
            Color::srgb(0.15, 0.2, 0.15)
        } else {
            Color::srgb(0.2, 0.35, 0.2)
        });
    }
}

/// Keeps the sidebar's Level readout in sync with `simulation` state.
fn sync_level_text(sim: Res<SimState>, mut text: Query<&mut Text, With<LevelText>>) {
    let Ok(mut text) = text.get_single_mut() else {
        return;
    };
    text.0 = format!("Level: {}/{}", sim.0.level_number(), simulation::LEVEL_COUNT);
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

/// Spawns the dev-only Command Palette bar, hidden by default, docked
/// across the bottom of the whole window (Grid and sidebar both) so it
/// never fights the sidebar's own layout.
fn spawn_command_palette(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden,
            CommandPaletteRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("> "),
                TextColor(Color::WHITE),
                CommandPaletteInputText,
            ));
            panel.spawn((
                Text::new(command_palette_help()),
                TextColor(Color::srgb(0.6, 0.6, 0.65)),
                CommandPaletteMessageText,
            ));
        });
}

fn command_palette_help() -> String {
    format!(
        "level <1..{}> | gold <amount> | skipwave  (Enter to run, Esc to close)",
        simulation::LEVEL_COUNT
    )
}

/// Opens/closes the Command Palette on Backquote (the `` ` `` /`~` key),
/// clearing any leftover input/result each time it opens.
fn toggle_command_palette(keys: Res<ButtonInput<KeyCode>>, mut palette: ResMut<CommandPalette>) {
    if keys.just_pressed(KeyCode::Backquote) {
        palette.open = !palette.open;
        palette.input.clear();
        palette.message = None;
    }
}

/// While the Command Palette is open, feeds raw `KeyboardInput` events
/// into its input line (Backspace to edit, Escape to close) and runs
/// the line through `run_command` on Enter. Ignored entirely while
/// closed — including the very Backquote-press event that just opened
/// or closed it, via the `key_code` check below, so that keystroke
/// never leaks into the input line.
fn type_into_command_palette(
    mut commands: Commands,
    mut key_events: EventReader<KeyboardInput>,
    mut palette: ResMut<CommandPalette>,
    mut sim: ResMut<SimState>,
    towers: Query<Entity, With<TowerAt>>,
) {
    if !palette.open {
        key_events.clear();
        return;
    }

    for event in key_events.read() {
        if event.state != ButtonState::Pressed || event.key_code == KeyCode::Backquote {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => palette.input.push_str(text),
            // `Key::Space` is a separate variant from `Key::Character`,
            // not text " " — without this arm Space silently falls
            // through to the catch-all below and never reaches the
            // input line.
            Key::Space => palette.input.push(' '),
            Key::Backspace => {
                palette.input.pop();
            }
            Key::Enter => {
                let command = std::mem::take(&mut palette.input);
                palette.message = Some(run_command(&mut commands, &mut sim.0, &towers, command.trim()));
            }
            Key::Escape => palette.open = false,
            _ => {}
        }
    }
}

/// Parses and runs one Command Palette line, returning the result/error
/// text to show under the input line. Every command here is a dev-only
/// shortcut for playtesting — none of it is reachable through normal
/// play.
fn run_command(
    commands: &mut Commands,
    sim: &mut Simulation,
    towers: &Query<Entity, With<TowerAt>>,
    command: &str,
) -> String {
    let mut parts = command.split_whitespace();
    match parts.next() {
        Some("level") => match parts.next().and_then(|arg| arg.parse::<usize>().ok()) {
            Some(n) if (1..=simulation::LEVEL_COUNT).contains(&n) => {
                sim.debug_set_level(n - 1);
                // The Tower sprites the Bevy layer spawned no longer
                // correspond to anything in `sim` once its Grid
                // changes underneath them.
                for entity in towers {
                    commands.entity(entity).despawn();
                }
                format!("Jumped to Level {n}/{}", simulation::LEVEL_COUNT)
            }
            _ => format!("Usage: level <1..{}>", simulation::LEVEL_COUNT),
        },
        Some("gold") => match parts.next().and_then(|arg| arg.parse::<i32>().ok()) {
            Some(amount) => {
                sim.debug_add_gold(amount);
                format!("+{amount} Gold (now {})", sim.gold())
            }
            None => "Usage: gold <amount>".to_string(),
        },
        Some("skipwave") => {
            sim.debug_skip_wave();
            "Wave skipped".to_string()
        }
        Some(other) => format!("Unknown command: {other}"),
        None => String::new(),
    }
}

/// Keeps the Command Palette's visibility and text in sync with
/// `CommandPalette`: shows/hides the whole bar, and while open, mirrors
/// the in-progress input line and the last result (or the help text,
/// before anything's been run yet).
fn sync_command_palette_ui(
    palette: Res<CommandPalette>,
    mut root: Query<&mut Visibility, With<CommandPaletteRoot>>,
    mut input_text: Query<&mut Text, (With<CommandPaletteInputText>, Without<CommandPaletteMessageText>)>,
    mut message_text: Query<&mut Text, (With<CommandPaletteMessageText>, Without<CommandPaletteInputText>)>,
) {
    let Ok(mut visibility) = root.get_single_mut() else {
        return;
    };
    *visibility = if palette.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if !palette.open {
        return;
    }

    if let Ok(mut text) = input_text.get_single_mut() {
        text.0 = format!("> {}_", palette.input);
    }
    if let Ok(mut text) = message_text.get_single_mut() {
        text.0 = palette.message.clone().unwrap_or_else(command_palette_help);
    }
}

/// Spawns the (initially hidden) full-window container the confirmation
/// dialog is rebuilt into whenever `PendingConfirm` changes.
fn spawn_confirm_dialog(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Hidden,
        ConfirmDialogRoot,
    ));
}

/// Rebuilds the confirmation dialog whenever `PendingConfirm` changes:
/// hidden (and transparent) while nothing awaits confirmation, otherwise
/// a dimming backdrop plus a message and Confirm/Cancel buttons.
fn sync_confirm_dialog(
    mut commands: Commands,
    pending: Res<PendingConfirm>,
    root: Query<Entity, With<ConfirmDialogRoot>>,
    children: Query<&Children>,
    mut last_rendered: Local<Option<ConfirmAction>>,
) {
    let Ok(root) = root.get_single() else {
        return;
    };

    if pending.0 == *last_rendered {
        return;
    }
    *last_rendered = pending.0;

    if let Ok(existing_children) = children.get(root) {
        for &child in existing_children {
            commands.entity(child).despawn_recursive();
        }
    }

    let Some(action) = pending.0 else {
        commands.entity(root).insert(Visibility::Hidden);
        return;
    };
    commands.entity(root).insert(Visibility::Visible);

    commands.entity(root).with_children(|overlay| {
        overlay.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            Interaction::default(),
            ConfirmDialogBackdrop,
        ));

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
                let message = match action {
                    ConfirmAction::Upgrade(_) => "Upgrade this tower?",
                    ConfirmAction::Sell(_) => "Sell this tower?",
                };
                panel.spawn((Text::new(message), TextColor(Color::WHITE)));
                panel
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                            ConfirmDialogCancelButton,
                        ))
                        .with_children(|button| {
                            button.spawn((Text::new("Cancel"), TextColor(Color::WHITE)));
                        });
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
                            ConfirmDialogConfirmButton,
                        ))
                        .with_children(|button| {
                            button.spawn((Text::new("Confirm"), TextColor(Color::WHITE)));
                        });
                    });
            });
    });
}

/// Commits or dismisses the pending Tower action: Confirm applies the
/// Upgrade/Sell (selling despawns the Tower sprite and deselects it),
/// while Cancel (or clicking the backdrop) simply clears `PendingConfirm`.
fn handle_confirm_dialog(
    mut commands: Commands,
    mut pending: ResMut<PendingConfirm>,
    mut sim: ResMut<SimState>,
    mut selected_tower: ResMut<SelectedTower>,
    confirm: Query<&Interaction, (With<ConfirmDialogConfirmButton>, Changed<Interaction>)>,
    cancel_or_backdrop: Query<
        &Interaction,
        (
            Or<(With<ConfirmDialogCancelButton>, With<ConfirmDialogBackdrop>)>,
            Changed<Interaction>,
        ),
    >,
    towers: Query<(Entity, &TowerAt)>,
) {
    if pending.0.is_none() {
        return;
    }

    let confirmed = confirm
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    let dismissed = cancel_or_backdrop
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if confirmed {
        match pending.0 {
            Some(ConfirmAction::Upgrade(pos)) => {
                let _ = sim.0.upgrade_tower(pos);
            }
            Some(ConfirmAction::Sell(pos)) => {
                if sim.0.sell_tower(pos) {
                    despawn_tower_sprite(&mut commands, &towers, pos);
                    selected_tower.0 = None;
                }
            }
            None => {}
        }
        pending.0 = None;
    } else if dismissed {
        pending.0 = None;
    }
}
