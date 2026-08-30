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
        .insert_resource(PendingPlacement(None))
        .insert_resource(PreviewTarget(None))
        .insert_resource(LastTouchAt(None))
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
                // Grouped so the whole Update list stays inside Bevy's
                // 20-system tuple limit; `.chain()` keeps them running
                // in this order all the same.
                (track_touch_activity, select_tower_kind, interact_with_grid).chain(),
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

/// Marks the translucent "ghost" Tower drawn on the Cell awaiting
/// placement confirmation (see `PendingPlacement`), cleared and
/// redrawn each frame like `PathPreviewTile`.
#[derive(Component)]
struct PlacementGhost;

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

/// The Cell a touch tap has picked but not yet committed: the first tap
/// of the two-step touch placement flow puts the Cell here (drawn as a
/// translucent ghost Tower plus its Path/Range preview), a second tap on
/// the same Cell places for real. `None` when nothing is awaiting
/// confirmation — which is always the case for mouse input, where hover
/// already previews a placement before the click commits it.
#[derive(Resource, Default)]
struct PendingPlacement(Option<CellPos>);

/// The Cell this frame's placement previews are drawn for: the pending
/// Cell if there is one, otherwise whatever the cursor (or a held touch)
/// is over. Published by `interact_with_grid`, which computes it anyway
/// for the Path overlay, so `draw_range_rings` can put its ring on the
/// same Cell without redoing the window → world → Cell conversion.
#[derive(Resource, Default)]
struct PreviewTarget(Option<CellPos>);

/// Time (seconds since app start) of the most recent touch input, or
/// `None` until the first touch — on a mouse-only device this stays
/// `None` and never affects anything. See `TOUCH_MOUSE_SUPPRESSION_SECS`.
#[derive(Resource, Default)]
struct LastTouchAt(Option<f32>);

/// How long after a touch mouse input is ignored. Browsers on touch
/// devices replay every tap as a synthetic mouse click a moment after
/// the finger lifts; without this window that replay would land on the
/// Cell the tap has just pended and place the Tower straight away,
/// collapsing the two-step flow back into one.
const TOUCH_MOUSE_SUPPRESSION_SECS: f32 = 1.0;

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

/// The Tower Kind's own color, faded down for the ghost drawn on a Cell
/// that is only awaiting confirmation. Low enough alpha that the Cell
/// underneath still shows through, so it can't be mistaken for a Tower
/// that is already paid for and placed.
fn placement_ghost_color(kind: TowerKind) -> Color {
    tower_color(kind).with_alpha(0.4)
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
/// window/camera → world → Cell conversion. Falls back to the active
/// touch point (logical coords, same space as `cursor_position`) on
/// touch-only devices where no cursor exists.
fn hovered_cell(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform)>,
    touch_position: Option<Vec2>,
) -> Option<CellPos> {
    let window = windows.get_single().ok()?;
    let cursor = window.cursor_position().or(touch_position)?;
    screen_to_cell(camera, cursor)
}

/// Which Cell (if any) a window-space position falls on. Shared by
/// `hovered_cell` and the touch-tap handling in `interact_with_grid`,
/// which needs the position of the touch that was *just pressed* rather
/// than whichever one is currently held.
fn screen_to_cell(camera: &Query<(&Camera, &GlobalTransform)>, position: Vec2) -> Option<CellPos> {
    let (camera, camera_transform) = camera.get_single().ok()?;
    let world = camera.viewport_to_world_2d(camera_transform, position).ok()?;
    world_to_cell(world)
}

/// Records when a touch was last active, so `interact_with_grid` can
/// ignore the synthetic mouse clicks a browser replays after each tap
/// (see `TOUCH_MOUSE_SUPPRESSION_SECS`).
fn track_touch_activity(
    time: Res<Time>,
    touches: Res<Touches>,
    mut last_touch: ResMut<LastTouchAt>,
) {
    if touches.iter().next().is_some() || touches.any_just_released() {
        last_touch.0 = Some(time.elapsed_secs());
    }
}

/// Whether a touch is still recent enough that mouse input should be
/// treated as its synthetic replay rather than a real click.
fn mouse_suppressed_by_touch(last_touch: &LastTouchAt, time: &Time) -> bool {
    last_touch
        .0
        .is_some_and(|at| time.elapsed_secs() - at < TOUCH_MOUSE_SUPPRESSION_SECS)
}

/// What a touch tap on a Grid Cell means, given whichever Cell (if any)
/// is already awaiting confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TapOutcome {
    /// The Cell holds a Tower: open its info panel.
    SelectTower,
    /// First tap on a placeable Cell: pick the position without
    /// committing to it, so the player can see the Path/Range it would
    /// produce before paying for it.
    Pend,
    /// Second tap on the Cell already pending: place the Tower.
    Place,
    /// Nothing to pick here (Obstacle/Spawn/Goal): drop whatever was
    /// pending or selected.
    Clear,
}

/// The two-step touch placement rule, kept pure so it can be unit-tested
/// without standing up a Bevy `App`.
fn resolve_tap(
    pending: Option<CellPos>,
    tapped: CellPos,
    has_tower: bool,
    buildable: bool,
) -> TapOutcome {
    if has_tower {
        TapOutcome::SelectTower
    } else if !buildable {
        TapOutcome::Clear
    } else if pending == Some(tapped) {
        TapOutcome::Place
    } else {
        TapOutcome::Pend
    }
}

/// Single seam between pointer input and the `simulation` crate: figures
/// out which Cell the player is aiming at, redraws the Path preview for
/// it, and either selects an existing Tower (opening its info panel) or
/// places a new one (subject to the Blocking Rule enforced entirely
/// inside `sim`).
///
/// Mouse and touch commit differently, because a finger has no hover
/// state: with a mouse, hovering already previews the placement and the
/// click commits it, while a touch tap only reaches the Cell it lands
/// on at the moment it lands. So touch is two-step (see
/// `PendingPlacement`): the first tap picks the Cell and shows it as a
/// translucent ghost Tower with its Path/Range preview, and a second tap
/// on that same Cell places for real.
fn interact_with_grid(
    mut commands: Commands,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    time: Res<Time>,
    last_touch: Res<LastTouchAt>,
    mut sim: ResMut<SimState>,
    selected: Res<SelectedTowerKind>,
    mut selected_tower: ResMut<SelectedTower>,
    mut pending: ResMut<PendingPlacement>,
    mut preview_target: ResMut<PreviewTarget>,
    pending_confirm: Res<PendingConfirm>,
    preview_tiles: Query<Entity, With<PathPreviewTile>>,
    ghosts: Query<Entity, With<PlacementGhost>>,
) {
    for entity in &preview_tiles {
        commands.entity(entity).despawn();
    }
    for entity in &ghosts {
        commands.entity(entity).despawn();
    }

    if sim.0.outcome().is_some() {
        // The result overlay is showing: no Grid click has any effect.
        pending.0 = None;
        preview_target.0 = None;
        return;
    }

    let hovered = hovered_cell(&windows, &camera, touches.first_pressed_position());

    // A pending Cell outranks the hovered one: on touch the finger has
    // usually lifted by the time the player is reading the preview, so
    // without this the Path/Range preview would blink out with it.
    preview_target.0 = pending.0.or(hovered);

    if let Some(target) = preview_target.0 {
        if !sim.0.has_tower(target) && sim.0.grid().kind_at(target) == CellKind::Buildable {
            if let Some(path) = sim.0.preview_path_if_placed(target) {
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
    }

    // The ghost Tower: the real thing's color and size but see-through, so
    // a pending Cell never reads as an already-placed Tower.
    if let Some(target) = pending.0 {
        commands.spawn((
            Sprite {
                color: placement_ghost_color(selected.0),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(target).with_z(0.75)),
            PlacementGhost,
        ));
    }

    // The Upgrade/Sell confirmation dialog covers the Grid: while it's
    // up its buttons own every tap and click, so none of them reach the
    // Cell that happens to sit under the dialog.
    if pending_confirm.0.is_some() {
        return;
    }

    // Touch taps go through the two-step flow. A tap that misses the
    // Grid entirely (the sidebar, say) leaves the pending Cell alone, so
    // switching Tower Kind mid-placement just recolors the ghost.
    if let Some(tapped) = touches
        .iter_just_pressed()
        .next()
        .and_then(|touch| screen_to_cell(&camera, touch.position()))
    {
        match resolve_tap(
            pending.0,
            tapped,
            sim.0.has_tower(tapped),
            sim.0.grid().kind_at(tapped) == CellKind::Buildable,
        ) {
            TapOutcome::SelectTower => {
                selected_tower.0 = Some(tapped);
                pending.0 = None;
            }
            TapOutcome::Pend => {
                selected_tower.0 = None;
                pending.0 = Some(tapped);
            }
            TapOutcome::Place => {
                selected_tower.0 = None;
                pending.0 = None;
                place_tower(&mut commands, &mut sim.0, tapped, selected.0);
            }
            TapOutcome::Clear => {
                selected_tower.0 = None;
                pending.0 = None;
            }
        }
        return;
    }

    if !mouse.just_pressed(MouseButton::Left) || mouse_suppressed_by_touch(&last_touch, &time) {
        return;
    }

    let Some(hovered) = hovered else {
        return;
    };

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
    place_tower(&mut commands, &mut sim.0, hovered, selected.0);
}

/// Places a Tower and spawns its sprite, if `sim` accepts the placement
/// (Blocking Rule, Gold, Cell Kind). A rejected placement is simply
/// ignored, as it has been since the Grid was click-only.
fn place_tower(commands: &mut Commands, sim: &mut Simulation, pos: CellPos, kind: TowerKind) {
    if sim.place_tower(pos, kind).is_ok() {
        commands.spawn((
            Sprite {
                color: tower_color(kind),
                custom_size: Some(Vec2::splat(CELL_SIZE_PX - 1.0)),
                ..default()
            },
            Transform::from_translation(cell_world_pos(pos).with_z(1.0)),
            TowerAt(pos),
        ));
    }
}

/// Draws a translucent Range ring so the player can see how far a
/// Tower's Range actually reaches in Cell units, rather than reading a
/// bare number off the info panel:
/// - a Tower selected for inspection shows its current (Tiered) Range;
/// - otherwise, the Cell awaiting touch confirmation — or, failing that,
///   a hovered Buildable Cell — previews the selected Tower Kind's Tier 1
///   Range for that placement.
/// Gizmos redraw every frame with no despawn bookkeeping needed, unlike
/// the Sprite-based overlays elsewhere in this file.
fn draw_range_rings(
    mut gizmos: Gizmos,
    sim: Res<SimState>,
    selected_tower: Res<SelectedTower>,
    selected_kind: Res<SelectedTowerKind>,
    preview_target: Res<PreviewTarget>,
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

    let Some(target) = preview_target.0 else {
        return;
    };
    if sim.0.has_tower(target) || sim.0.grid().kind_at(target) != CellKind::Buildable {
        return;
    }

    let range = selected_kind.0.range(TowerTier::One);
    let center = cell_world_pos(target).truncate();
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
    mut pending: ResMut<PendingPlacement>,
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
        // The new Level's Grid has its own Obstacle layout, so a Cell
        // picked on the old one means nothing now.
        pending.0 = None;
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
    mut pending: ResMut<PendingPlacement>,
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
            pending.0 = None;
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
    mut pending: ResMut<PendingPlacement>,
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
                palette.message = Some(run_command(
                    &mut commands,
                    &mut sim.0,
                    &mut pending,
                    &towers,
                    command.trim(),
                ));
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
    pending: &mut PendingPlacement,
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
                pending.0 = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: CellPos = CellPos { x: 3, y: 4 };
    const OTHER_CELL: CellPos = CellPos { x: 7, y: 2 };

    #[test]
    fn first_tap_on_a_buildable_cell_only_pends_it() {
        assert_eq!(resolve_tap(None, CELL, false, true), TapOutcome::Pend);
    }

    #[test]
    fn second_tap_on_the_pending_cell_places() {
        assert_eq!(
            resolve_tap(Some(CELL), CELL, false, true),
            TapOutcome::Place
        );
    }

    #[test]
    fn tapping_another_cell_moves_the_pending_cell_instead_of_placing() {
        assert_eq!(
            resolve_tap(Some(CELL), OTHER_CELL, false, true),
            TapOutcome::Pend
        );
    }

    #[test]
    fn tapping_a_tower_selects_it_even_while_a_cell_is_pending() {
        assert_eq!(
            resolve_tap(Some(CELL), OTHER_CELL, true, false),
            TapOutcome::SelectTower
        );
    }

    #[test]
    fn tapping_an_unbuildable_cell_clears_the_pending_cell() {
        assert_eq!(
            resolve_tap(Some(CELL), OTHER_CELL, false, false),
            TapOutcome::Clear
        );
    }
}
