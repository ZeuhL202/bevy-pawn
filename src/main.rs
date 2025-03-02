#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f64::consts::PI;
use rand::Rng;
use bevy::{
    input::mouse::{self, MouseWheel}, prelude::*, sprite::Wireframe2dPlugin
};

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const TILE_SIZE: f32 = 50.0;
const TILE_RANGE: i32 = 10;

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
    tile: Option<Vec2>,
}

#[derive(Component)]
struct DebugLog(String);

impl DebugLog {
    fn add(&mut self, s: String) {
        self.0 += &format!("\n{}", s);
    }
}

fn tile_rounded_field_position(position: Vec2) -> Vec2 {
    let x = (position.x / TILE_SIZE).round() * TILE_SIZE;
    let y = (position.y / TILE_SIZE).round() * TILE_SIZE;
    Vec2::new(x, y)
}

fn setup(
    mut commands: Commands,
    mut windows: Query<&mut Window>,
) {
    // ウィンドウ
    let mut window = windows.single_mut();
    window.resolution.set(WINDOW_WIDTH, WINDOW_HEIGHT);
    window.resizable = false;
    window.title = "Bevy".to_string();

    let camera_position = 25.0 * TILE_RANGE as f32;

    commands.spawn((
        Camera2d,
        Transform::from_xyz(camera_position, camera_position, 0.0),
    ));
}

fn spawn_pawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::rng();
    for _ in 0..5 {
        let rng_x = rng.random_range(0..TILE_RANGE) as f32 * TILE_SIZE;
        let rng_y = rng.random_range(0..TILE_RANGE) as f32 * TILE_SIZE;

        commands.spawn((
            Sprite::from_image(asset_server.load("pawn.png")),
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
    window: Query<&Window>,
    camera: Query<(&Transform, &OrthographicProjection), With<Camera2d>>,
    mut mouse_position: ResMut<MousePosition>,
) {
    let Some(window) = window.get_single().ok() else { return };
    let Some(camera) = camera.get_single().ok() else { return };

    let transform = camera.0.translation;
    let scale = camera.1.scale;

    if let Some(position) = window.cursor_position() {
        let field_position = Vec2 {
            x: (position.x + transform.x / scale - WINDOW_WIDTH / 2.0) * scale,
            y: (position.y - transform.y / scale - WINDOW_HEIGHT / 2.0) * -scale,
        };

        mouse_position.window = Some(Vec2::new(position.x, position.y));
        mouse_position.field = Some(field_position);
        mouse_position.tile = Some(field_position / TILE_SIZE);
    } else {
        mouse_position.window = None;
        mouse_position.field = None;
        mouse_position.tile = None;
    }
}

fn log_mouse_position(
    mouse_position: Res<MousePosition>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(mut debug_log) = debug_log.get_single_mut().ok() else { return };

    let string =
        if let Some(position) = mouse_position.window {
            format!("x:{:.2}, y:{:.2}", position.x, position.y)
        } else {
            "NoCursor".to_string()
        };

    debug_log.add(format!("MousePosition {}", string));
}

fn select_pawn(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    asset_server: Res<AssetServer>,
    selecters: Query<Entity, With<Selecter>>,
    things: Query<(&Thing, Entity), (With<Thing>, Without<Selecter>)>,
    mut commands: Commands,
) {
    // if no click then return
    if !mouse_button_input.just_pressed(MouseButton::Left) { return };

    // Remove all selecters
    selecters.iter().for_each(|e| commands.entity(e).despawn());

    // get the hovered thing
    let Some(hovered) = things.iter().filter(|(c, _)| c.hovered).next() else { return };
    let hovered = hovered.1;

    // create a selecter as a child of the hovered things
    let image_hundle = asset_server.load("frame.png");
    let child = commands.spawn((
        Sprite::from_image(image_hundle),
        Transform::from_xyz(0.0, 0.0, 1.5),
        Selecter,
    )).id();

    commands.entity(hovered).add_child(child);
}

fn let_move_pawn(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    selecter: Query<Entity, With<Selecter>>,
    mut pawns: Query<(&Children, &mut Pawn), With<Pawn>>,
) {
    if !mouse_button_input.just_pressed(MouseButton::Right) { return };
    let field_position = field_position(position.0);

    for (_, mut pawn) in pawns.iter_mut().filter(|(c, _)| c.iter().any(|&e| selecter.get(e).is_ok())) {
        pawn.state = PawnState::Move(field_position);
    };
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
    mut things: Query<(&Transform, &mut Thing), With<Thing>>,
    camera: Query<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
) {
    let Some(window) = window.get_single().ok() else { return };
    let Some(cursor_position) = window.cursor_position() else { return };

    for (transform, mut component) in things.iter_mut() {
        let cursor_position = field_mouse_position(cursor_position, camera.single().1.translation, camera.single().0.scale);
        let thing_x = transform.translation.x;
        let thing_y = transform.translation.y;
        let half_size = TILE_SIZE / 2.0;

        // The area is a square centered on the current coordinate system.
        component.hovered = ((thing_x - half_size)..(thing_x + half_size)).contains(&cursor_position.x)
                         && ((thing_y - half_size)..(thing_y + half_size)).contains(&cursor_position.y);
    }
}

fn spawn_tile(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let t: fn(i32) -> f32 = |i| i as f32 * TILE_SIZE;

    let image_hundle = asset_server.load("debug_tile.png");

    for i in 0..TILE_RANGE {
        for j in 0..TILE_RANGE {
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

fn log_camera_position(
    camera: Query<&Transform, With<Camera2d>>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(camera) = camera.get_single().ok() else { return };
    let Some(mut debug_log) = debug_log.get_single_mut().ok() else { return };
    debug_log.add(format!(
        "CameraPosition x:{:.2}, y:{:.2}",
        camera.translation.x,
        camera.translation.y
    ));
}

fn move_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let Some(mut camera) = camera.get_single_mut().ok() else { return };
    let mut distance = Vec2::ZERO;

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

fn log_camera_scale(
    camera: Query<&OrthographicProjection, With<Camera2d>>,
    mut debug_log: Query<&mut DebugLog>,
) {
    let Some(camera) = camera.get_single().ok() else { return };
    let Some(mut debug_log) = debug_log.get_single_mut().ok() else { return };

    debug_log.add(format!("CameraScale: {:.2}", camera.scale));
}

fn zoom_camera(
    mut camera: Query<&mut OrthographicProjection, With<Camera2d>>,
    mut event_read_scroll: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let Some(mut camera) = camera.get_single_mut().ok() else { return };
    let Some(ev) = event_read_scroll.read().next() else { return };

    let shift_speed = if keys.pressed(KeyCode::ShiftLeft) { 10.0 } else { 1.0 };
    let ds = ev.y * settings.camera_zoom_speed * shift_speed * 20.0 * time.delta_secs();
    let post_scale = camera.scale + ds;

    if (0.1..10.0).contains(&post_scale) {
        camera.scale += ds;
    }
}

fn output_log(
    mut text: Query<&mut Text, With<DebugLog>>,
    mut buf: Query<&mut DebugLog>,
) {
    let Some(mut text) = text.get_single_mut().ok() else { return };
    let Some(mut buf) = buf.get_single_mut().ok() else { return };
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

    if let Some(e) = entity.get_single().ok() {
        commands.entity(e).despawn();
    } else {
        commands.spawn((
            Text::new(""),
            TextFont {
                font: asset_server.load("fonts/Menlo-Regular.ttf"),
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
    let Some((window, _focus)) = window.get_single().ok() else { return };
    if input.just_pressed(KeyCode::KeyQ) {
        commands.entity(window).despawn();
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Wireframe2dPlugin,
        ))
        .insert_resource(Settings::default())
        .add_systems(Startup, (
            setup,
            spawn_tile,
            spawn_pawn,
        ))
        .add_systems(Update, (
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
