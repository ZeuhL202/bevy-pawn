#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f64::consts::PI;
use rand::Rng;
use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    sprite::Wireframe2dPlugin
};

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const TILE_SIZE: f32 = 50.0;

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

#[derive(Component)]
struct DebugLog(String);

impl DebugLog {
    fn add(&mut self, s: String) {
        self.0 += &format!("\n{}", s);
    }
}

#[derive(Event)]
struct Click();

#[derive(Event)]
struct RightClick(Vec2);

fn get_tile_pos(pos: Vec2) -> Vec2 {
    let x = (pos.x / TILE_SIZE).round() * TILE_SIZE;
    let y = (pos.y / TILE_SIZE).round() * TILE_SIZE;
    Vec2::new(x, y)
}

fn current_mouse_pos(pos: Vec2, transform: Vec3, scale: f32) -> Vec2 {
    Vec2 {
        x: (pos.x + transform.x / scale - WINDOW_WIDTH / 2.0) * scale,
        y: (pos.y - transform.y / scale - WINDOW_HEIGHT / 2.0) * -scale,
    }
}

fn setup(
    mut windows: Query<&mut Window>,
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
) {
    // ウィンドウ
    let mut window = windows.single_mut();
    window.resolution.set(WINDOW_WIDTH, WINDOW_HEIGHT);
    window.title = "Bevy".to_string();

    cmds.spawn((
        Camera2d,
        Transform::from_xyz(250.0, 250.0, 0.0),
    ));

    let mut rng = rand::rng();
    for _ in 0..5 {
        let rng_x = rng.random_range(0..10) as f32 * TILE_SIZE;
        let rng_y = rng.random_range(0..10) as f32 * TILE_SIZE;
        cmds.spawn((
            Sprite::from_image(asset_server.load("pawn.png")),
            Transform::from_xyz(rng_x, rng_y, 1.0)
                .with_scale(Vec3::splat(5.0 / TILE_SIZE)),
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

fn click_handle(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera: Single<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
    mut events: EventWriter<Click>,
    mut debug_log: Query<&mut DebugLog>,
) {
    if let Some(window) = window.get_single().ok() {
        if let Some(mut pos) = window.cursor_position() {
            pos = current_mouse_pos(pos, camera.1.translation, camera.0.scale);
            if mouse_button_input.just_pressed(MouseButton::Left) {
                events.send(Click());
            }
            if let Ok(mut debug_log) = debug_log.get_single_mut() {
                debug_log.add(format!("MousePosition x:{:.2}, y:{:.2}", pos.x, pos.y));
            }
        } else {
            if let Ok(mut debug_log) = debug_log.get_single_mut() {
                debug_log.add("MousePosition NoCursor".to_string());
            }
        }
    }
}

fn right_click_handle(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera: Single<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
    mut events: EventWriter<RightClick>,
) {
    if let Some(window) = window.get_single().ok() {
        if let Some(mut pos) = window.cursor_position() {
            pos = current_mouse_pos(pos, camera.1.translation, camera.0.scale);
            if mouse_button_input.just_pressed(MouseButton::Right) {
                events.send(RightClick(pos));
            }
        }
    }
}

fn select_pawn(
    mut event: EventReader<Click>,
    selecters: Query<Entity, With<Selecter>>,
    things: Query<(&Thing, Entity), (With<Thing>, Without<Selecter>)>,
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
) {
    if let None = event.read().next() { return }; // if no click event then return
    selecters.iter().for_each(|e| cmds.entity(e).despawn()); // Remove all selecters

    // get the hovered thing
    let Some(hovered) = things.iter().filter(|(c, _)| c.hovered).next() else { return };
    let hovered = hovered.1;

    // create a selecter as a child of the hovered things
    let image_hundle = asset_server.load("frame.png");
    let child = cmds.spawn((
        Sprite::from_image(image_hundle),
        Transform::from_xyz(0.0, 0.0, 1.5),
        Selecter,
    )).id();

    cmds.entity(hovered).add_child(child);
}

fn let_move_pawn(
    mut event: EventReader<RightClick>,
    selecter: Query<Entity, With<Selecter>>,
    mut pawns: Query<(&Children, &mut Pawn), With<Pawn>>,
) {
    let Some(pos) = event.read().next() else { return };
    let tile_pos = get_tile_pos(pos.0);

    pawns.iter_mut().filter(|(c, _)| c.iter().any(|&e| selecter.get(e).is_ok())).for_each(|(_, mut pawn)| {
        pawn.state = PawnState::Move(tile_pos);
    });
}

fn move_pawn(
    mut pawns: Query<(&mut Pawn, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut pawn, mut pawn_pos) in pawns.iter_mut() {
        if let PawnState::Move(dest_pos) = pawn.state {
            let pawn_y = pawn_pos.translation.y;
            let dest_y = dest_pos.y;
            let pawn_x = pawn_pos.translation.x;
            let dest_x = dest_pos.x;

            let theta = (pawn_y - dest_y).atan2(pawn_x - dest_x) + PI as f32;
            let dist = ((pawn_y - dest_y).powi(2) + (pawn_x - dest_x).powi(2)).sqrt();
            let power =
                if dist < 1.0 {
                    pawn.state = PawnState::Idle;
                    pawn_pos.translation.x += (dest_x - pawn_x) * time.delta_secs();
                    pawn_pos.translation.y += (dest_y - pawn_y) * time.delta_secs();
                    return
                } else if dist < 10.0 {
                    dist / 10.0 * 100.0
                } else {
                    100.0
                };

            pawn_pos.translation.x += theta.cos() * power * time.delta_secs();
            pawn_pos.translation.y += theta.sin() * power * time.delta_secs();
        }
    };
}

fn is_thing_hovered(
    mut things: Query<(&Transform, &mut Thing), With<Thing>>,
    camera: Query<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
) {
    let Some(window) = window.get_single().ok() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };

    for (transform, mut component) in things.iter_mut() {
        let cursor_pos = current_mouse_pos(cursor_pos, camera.single().1.translation, camera.single().0.scale);
        let thing_x = transform.translation.x;
        let thing_y = transform.translation.y;
        let half_size = TILE_SIZE / 2.0;

        if ((thing_x - half_size)..(thing_x + half_size)).contains(&cursor_pos.x)
        && ((thing_y - half_size)..(thing_y + half_size)).contains(&cursor_pos.y) {
            component.hovered = true;
        } else {
            component.hovered = false;
        }
    }
}

fn spawn_tile(
    mut cmds: Commands,
) {
    let c: fn(i32) -> f32 = |i| i as f32 / 10.0;
    let t: fn(i32) -> f32 = |i| i as f32 * TILE_SIZE;

    for i in 0..10 {
        for j in 0..10 {
            cmds.spawn((
                Sprite::from_color(
                    Color::srgb(c(i), c(j), 0.5),
                    Vec2 { x: TILE_SIZE, y: TILE_SIZE }
                ),
                Transform::from_xyz(t(i), t(j), 0.0),
            ));
            cmds.spawn((
                Text2d::new(format!("({},{})", i, j),),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                Transform::from_xyz(t(i), t(j), 0.5)
            ));
        }
    }
}

fn move_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    mut debug_log: Query<&mut DebugLog>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let mut camera = camera.single_mut();
    let mut dx: f32 = 0.0;
    let mut dy: f32 = 0.0;
    let camera_speed = settings.camera_move_speed;

    if keys.pressed(KeyCode::KeyW) { dy += 1.0 * camera_speed }
    if keys.pressed(KeyCode::KeyA) { dx -= 1.0 * camera_speed }
    if keys.pressed(KeyCode::KeyS) { dy -= 1.0 * camera_speed }
    if keys.pressed(KeyCode::KeyD) { dx += 1.0 * camera_speed }
    if keys.pressed(KeyCode::ShiftLeft) {
        dx *= 3.0;
        dy *= 3.0;
    }

    camera.translation.x += dx * time.delta_secs();
    camera.translation.y += dy * time.delta_secs();

    // Add a camera position to the log
    let Some(mut debug_log) = debug_log.get_single_mut().ok() else { return };
    debug_log.add(format!(
        "CameraPosition x:{:.2}, y:{:.2}",
        camera.translation.x,
        camera.translation.y
    ));
}

fn zoom_camera(
    mut camera: Query<&mut OrthographicProjection, With<Camera2d>>,
    mut evr_scroll: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_log: Query<&mut DebugLog>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let mut camera = camera.single_mut();
    let evr_scroll_first = evr_scroll.read().next();

    let Some(ev) = evr_scroll_first else { return };
    match ev.unit {
        MouseScrollUnit::Line => {
            let shift_speed = if keys.pressed(KeyCode::ShiftLeft) { 10.0 } else { 1.0 };
            let ds = ev.y * settings.camera_zoom_speed * shift_speed * 20.0 * time.delta_secs();
            let post_scale = camera.scale + ds;

            if 0.1 < post_scale && post_scale < 10.0 {
                camera.scale += ds;
            }
        },
        MouseScrollUnit::Pixel => {
            let shift_speed = if keys.pressed(KeyCode::ShiftLeft) { 10.0 } else { 1.0 };
            let ds = ev.y * settings.camera_zoom_speed * shift_speed * time.delta_secs();
            let post_scale = camera.scale + ds;

            if 0.1 < post_scale && post_scale < 10.0 {
                camera.scale += ds;
            }
        },
    }

    // Add a camera scale to the log
    let Some(mut debug_log) = debug_log.get_single_mut().ok() else { return };
    debug_log.add(format!("CameraScale: {:.2}", camera.scale));
}

fn output_log(
    mut text: Query<&mut Text, With<DebugLog>>,
    mut buf: Query<&mut DebugLog>,
) {
    let (Some(mut text), Some(mut buf)) = (text.get_single_mut().ok(), buf.get_single_mut().ok()) else { return };
    text.0 = buf.0.clone();
    buf.0 = "--- LOG ---".to_string();
}

fn toggle_log(
    entity: Query<Entity, With<DebugLog>>,
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) { return }

    let Some(e) = entity.get_single().ok() else {
        let font_handle = asset_server.load("fonts/Menlo-Regular.ttf");
        cmds.spawn((
            Text::new(""),
            TextFont {
                font: font_handle,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            DebugLog("--- LOG ---".to_string())
        ));
        return
    };
    cmds.entity(e).despawn();
}

fn close_on_q(
    mut cmds: Commands,
    window: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let Ok((window, _focus)) = window.get_single() else { return };
    if input.just_pressed(KeyCode::KeyQ) {
        cmds.entity(window).despawn();
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Wireframe2dPlugin,
        ))
        .insert_resource(Settings::default())
        .add_event::<Click>()
        .add_event::<RightClick>()
        .add_systems(Startup, (
            setup,
            spawn_tile,
        ))
        .add_systems(Update, (
            click_handle,
            right_click_handle,
            select_pawn,
            let_move_pawn,
            move_pawn,
            is_thing_hovered,
            move_camera,
            zoom_camera,
            output_log,
            toggle_log,
            close_on_q,
        ).chain())
        .run();
}
