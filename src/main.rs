#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f64::consts::PI;
use rand::Rng;
use toml;
use bevy::{
    input::mouse::MouseWheel,
    prelude::*
};
use bevy_embedded_assets::EmbeddedAssetPlugin;

const DEFAULT_WINDOW_WIDTH: f32 = 1600.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 900.0;
const TILE_SIZE: f32 = 50.0;
const TILE_RANGE: i32 = 10;

struct MapInfo {
    tile_keys: Vec<String>,
    tile_values: Vec<Vec<u8>>,
}

enum TileKey {
    Decorationable(),
}

#[derive(Component)]
struct Tile {
    position: IVec2,
}

#[derive(PartialEq)]
enum PawnState {
    Idle,
    Move(Vec2),
}

#[derive(Component)]
struct Pawn {
    state: PawnState
}

impl Default for Pawn {
    fn default() -> Self {
        Self {
            state: PawnState::Idle
        }
    }
}

#[derive(Component)]
struct Thing {
    hovered: bool,
    _selected: bool,
}

#[derive(Component)]
struct Selecter;

#[derive(Resource)]
struct Settings {
    camera_move_speed: f32,
    camera_zoom_speed: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            camera_move_speed: 100.0,
            camera_zoom_speed: 0.1,
        }
    }
}

#[derive(Resource)]
struct MousePosition {
    window: Option<Vec2>,
    field: Option<Vec2>,
    field_tile_rounded: Option<Vec2>,
    field_before_middle_pressed: Option<Vec2>,
    tile: Option<IVec2>,
}

#[derive(Component)]
struct DebugLog(String);

impl DebugLog {
    fn add(&mut self, s: String) {
        self.0 += &format!("\n{}", s);
    }
}

fn setup(
    mut commands: Commands,
    mut windows: Query<&mut Window>,
) {
    // ウィンドウ
    let mut window = windows.single_mut().unwrap();
    window.resolution.set(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
    window.resizable = false;
    window.enabled_buttons.maximize = false;
    window.title = "Bevy".to_string();

    commands.spawn(Camera2d);
}

fn spawn_pawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::rng();
    let half_tile_range = TILE_RANGE / 2;
    for _ in 0..5 {
        let rng_x = rng.random_range(-half_tile_range..half_tile_range) as f32 * TILE_SIZE;
        let rng_y = rng.random_range(-half_tile_range..half_tile_range) as f32 * TILE_SIZE;

        commands.spawn((
            Sprite::from_image(asset_server.load("embedded://pawn.png")),
            Transform::from_xyz(rng_x, rng_y, 1.0)
                .with_scale(Vec3::splat(TILE_SIZE / 512.0)),
            Thing {
                hovered: false,
                _selected: false,
            },
            Pawn {
                state: PawnState::Idle
            },
        ));
    }
}

fn send_resouce_mouse_position(
    button: Res<ButtonInput<MouseButton>>,
    mut mouse_position: ResMut<MousePosition>,
    window: Query<&Window>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
) {
    let Some(window) = window.single().ok() else { return };
    let Some(camera) = camera.single().ok() else { return };

    let transform = camera.0.translation;
    let scale = if let Projection::Orthographic(o) = camera.1 { o.scale } else { return };

    if let Some(position) = window.cursor_position() {
        let field_position = Vec2 {
            x: (position.x + transform.x / scale - DEFAULT_WINDOW_WIDTH / 2.0) * scale,
            y: (position.y - transform.y / scale - DEFAULT_WINDOW_HEIGHT / 2.0) * -scale,
        };

        let field_tile_rounded_position = Vec2 {
            x: (field_position.x / TILE_SIZE).round() * TILE_SIZE,
            y: (field_position.y / TILE_SIZE).round() * TILE_SIZE,
        };

        let tile = IVec2 {
            x: (field_position.x / TILE_SIZE).round() as i32,
            y: (field_position.y / TILE_SIZE).round() as i32,
        };

        mouse_position.window = Some(Vec2::new(position.x, position.y));
        mouse_position.field = Some(field_position);
        mouse_position.field_tile_rounded = Some(field_tile_rounded_position);
        mouse_position.tile = Some(tile);

        if !button.pressed(MouseButton::Middle) {
            mouse_position.field_before_middle_pressed = Some(field_position);
        }
    } else {
        mouse_position.window = None;
        mouse_position.field = None;
        mouse_position.field_tile_rounded = None;
        mouse_position.field_before_middle_pressed = None;
        mouse_position.tile = None;
    }
}

fn log_mouse_position(
    mouse_position: Res<MousePosition>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(mut debug_log) = debug_log.single_mut().ok() else { return };

    let string = |opt_vec2: Option<Vec2>| {
        if let Some(vec2) = opt_vec2 {
            format!("x:{:.2}, y:{:.2}", vec2.x, vec2.y)
        } else {
            "OutOfWindow".to_string()
        }
    };

    let string_i = |opt_vec2: Option<IVec2>| {
        if let Some(vec2) = opt_vec2 {
            format!("x:{:.2}, y:{:.2}", vec2.x, vec2.y)
        } else {
            "OutOfWindow".to_string()
        }
    };

    debug_log.add(format!("MousePosition win: {}", string(mouse_position.window)));
    debug_log.add(format!("              fld: {}", string(mouse_position.field)));
    debug_log.add(format!("              ftr: {}", string(mouse_position.field_tile_rounded)));
    debug_log.add(format!("              fbm: {}", string(mouse_position.field_before_middle_pressed)));
    debug_log.add(format!("              tle: {}", string_i(mouse_position.tile)));
}

fn select_pawn(
    mouse_button: Res<ButtonInput<MouseButton>>,
    asset_server: Res<AssetServer>,
    selecters: Query<Entity, With<Selecter>>,
    things: Query<(&Thing, Entity), (With<Thing>, Without<Selecter>)>,
    mut commands: Commands,
) {
    // if no click then return
    if !mouse_button.just_pressed(MouseButton::Left) { return };

    // Remove all selecters
    selecters.iter().for_each(|e| commands.entity(e).despawn());

    // get the hovered thing
    let Some(hovered) = things.iter().filter(|(c, _)| c.hovered).next() else { return };
    let hovered = hovered.1;

    // create a selecter as a child of the hovered things
    let image_hundle = asset_server.load("embedded://frame.png");
    let child = commands.spawn((
        Sprite::from_image(image_hundle),
        Transform::from_xyz(0.0, 0.0, 1.5),
        Selecter,
    )).id();

    commands.entity(hovered).add_child(child);
}

fn let_move_pawn(
    mouse_position: Res<MousePosition>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    selecter: Query<Entity, With<Selecter>>,
    pawns: Query<(&Children, &mut Pawn), With<Pawn>>,
) {
    if !mouse_button.just_pressed(MouseButton::Right) { return };
    let Some(mouse_position) = mouse_position.field_tile_rounded else { return };

    for (children, mut pawn) in pawns {
        for &child in children {
            if let Ok(_) = selecter.get(child) {
                pawn.state = PawnState::Move(mouse_position);
            }
        }
    }
}

fn move_pawn(
    mut pawns: Query<(&mut Pawn, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut component, mut transform) in pawns.iter_mut() {
        // if pawn's state is not Move then continue
        let PawnState::Move(destination) = component.state else { continue };

        // straight angle to the destination
        let theta = (transform.translation.y - destination.y).atan2(transform.translation.x - destination.x) + PI as f32;
        let distance = transform.translation.distance(Vec3::new(destination.x, destination.y, 1.0));

        if distance < 0.1 {
            component.state = PawnState::Idle;
            transform.translation = Vec3::new(destination.x, destination.y, 1.0);
            return;
        }

        // The speed increase is capped at 10.0
        let speed = distance.min(10.0) * 10.0;

        // The intersection of the straight line to the destination and the unit circle with itself as the origin is the coordinate to move forward.
        transform.translation.x += theta.cos() * speed * time.delta_secs();
        transform.translation.y += theta.sin() * speed * time.delta_secs();
    };
}

fn is_thing_hovered(
    mouse_position: Res<MousePosition>,
    mut things: Query<(&Transform, &mut Thing), With<Thing>>,
) {
    let Some(mouse_position) = mouse_position.field_tile_rounded else { return };

    for (transform, mut component) in things.iter_mut() {
        let thing_x = transform.translation.x;
        let thing_y = transform.translation.y;
        let half_size = TILE_SIZE / 2.0;

        // The area is a square centered on the current coordinate system.
        component.hovered = ((thing_x - half_size)..(thing_x + half_size)).contains(&mouse_position.x)
                         && ((thing_y - half_size)..(thing_y + half_size)).contains(&mouse_position.y);
    }
}

fn spawn_tile(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let t: fn(i32) -> f32 = |i| i as f32 * TILE_SIZE;

    let image_hundle = asset_server.load("embedded://debug_tile.png");

    let half_tile_range = TILE_RANGE / 2;

    for i in -half_tile_range..half_tile_range {
        for j in -half_tile_range..half_tile_range {
            commands.spawn((
                Sprite::from_image(image_hundle.clone()),
                Transform::from_xyz(t(i), t(j), 0.0)
                    .with_scale(Vec3::splat(TILE_SIZE / 512.0)),
            ))
            .with_child((
                Text2d::new(format!("({},{})", i, j)),
                Transform::from_xyz(0.0, 0.0, 0.5)
                    .with_scale(Vec3::splat(2.5)),
            ));
        }
    }
}

fn load_map(
    mut tiles: Query<Entity, With<Tile>>,
) {
    for tile in tiles.iter() {
        
    }
}

fn log_camera_position(
    camera: Query<&Transform, With<Camera2d>>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(camera) = camera.single().ok() else { return };
    let Some(mut debug_log) = debug_log.single_mut().ok() else { return };
    debug_log.add(format!(
        "CameraPosition x:{:.2}, y:{:.2}",
        camera.translation.x,
        camera.translation.y
    ));
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_position: Res<MousePosition>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let Some(mut camera) = camera.single_mut().ok() else { return };
    let mut distance = Vec2::ZERO;

    if mouse_button.pressed(MouseButton::Middle) {
        if let (
            Some(field),
            Some(field_before_middle)
        ) = (
            mouse_position.field,
            mouse_position.field_before_middle_pressed
        ) {
            camera.translation.x += field_before_middle.x - field.x;
            camera.translation.y += field_before_middle.y - field.y;
        }
    } else {
        if keys.pressed(KeyCode::KeyW) { distance.y += 1.0 }
        if keys.pressed(KeyCode::KeyA) { distance.x -= 1.0 }
        if keys.pressed(KeyCode::KeyS) { distance.y -= 1.0 }
        if keys.pressed(KeyCode::KeyD) { distance.x += 1.0 }

        let shift_multiplier = if keys.pressed(KeyCode::ShiftLeft) { 10.0 } else { 1.0 };

        if distance.length_squared() > 0.0 {
            distance = distance.normalize();
        }

        camera.translation.x += distance.x * settings.camera_move_speed * shift_multiplier * time.delta_secs();
        camera.translation.y += distance.y * settings.camera_move_speed * shift_multiplier * time.delta_secs();
    }
}

fn log_camera_scale(
    camera: Query<&Projection, With<Camera2d>>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(camera) = camera.single().ok() else { return };
    let scale = if let Projection::Orthographic(o) = camera { o.scale } else { return };

    let Some(mut debug_log) = debug_log.single_mut().ok() else { return };

    debug_log.add(format!("CameraScale: {:.2}", scale));
}

fn zoom_camera(
    mut camera: Query<&mut Projection, With<Camera2d>>,
    mut event_read_scroll: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let Some(mut camera) = camera.single_mut().ok() else { return };
    let Some(ev) = event_read_scroll.read().next() else { return };

    let shift_multiplier =
        if keys.pressed(KeyCode::ShiftLeft) {
            10.0
        } else {
            1.0
        };

    let change = ev.y * settings.camera_zoom_speed * shift_multiplier * 20.0 * time.delta_secs();

    if let Projection::Orthographic(ortho) = camera.as_mut() {
        let post_scale = ortho.scale - change;

        if (0.1..10.0).contains(&post_scale) {
            ortho.scale -= change;
        }
    };
}

fn output_log(
    mut text: Query<&mut Text, With<DebugLog>>,
    mut buf: Query<&mut DebugLog>,
) {
    let Some(mut text) = text.single_mut().ok() else { return };
    let Some(mut buf) = buf.single_mut().ok() else { return };

    text.0 = buf.0.clone();
    buf.0 = "--- LOG ---".to_string();
}

fn toggle_log(
    entity: Query<Entity, With<DebugLog>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) { return }

    if let Some(e) = entity.single().ok() {
        commands.entity(e).despawn();
    } else {
        commands.spawn((
            Text::new(""),
            TextFont {
                font: asset_server.load("embedded://fonts/Menlo-Regular.ttf"),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            DebugLog("--- LOG ---".to_string())
        ));
    }
}

fn close_on_q(
    mut commands: Commands,
    window: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let Some((window, _focus)) = window.single().ok() else { return };

    if input.just_pressed(KeyCode::KeyQ) {
        commands.entity(window).despawn();
    }
}

fn main() {
    App::new()
        .add_plugins((
            EmbeddedAssetPlugin::default(),
            DefaultPlugins,
        ))
        .insert_resource(Settings::default())
        .insert_resource(MousePosition{
            window: None,
            field: None,
            field_tile_rounded: None,
            field_before_middle_pressed: None,
            tile: None
        })
        .add_systems(Startup, (
            setup,
            spawn_tile,
            spawn_pawn,
        ))
        .add_systems(Update, (
            send_resouce_mouse_position,
            select_pawn,
            let_move_pawn,
            move_pawn,
            is_thing_hovered,
            move_camera,
            zoom_camera,
            close_on_q,
        ))
        .add_systems(Update, (
            log_mouse_position,
            log_camera_position,
            log_camera_scale,
            output_log,
            toggle_log,
        ).chain())
        .run();
}
