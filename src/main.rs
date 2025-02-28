use bevy::{ input::mouse::MouseWheel, prelude::*, sprite::Wireframe2dPlugin };

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
struct Thing;

#[derive(Component)]
struct Selecter(u32);

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
struct HoveredWhat {
    id: Vec<u32>,
}

impl Default for HoveredWhat {
    fn default() -> Self {
        Self {
            id: vec![]
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

fn new_debug_log(asset_server: Res<AssetServer>) -> (Text, TextFont, BackgroundColor, DebugLog) {
    let font_handle: Handle<Font> = asset_server.load("fonts/Menlo-Regular.ttf");
    (
        Text::new(""),
        TextFont {
            font: font_handle,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        DebugLog("---LOG---".to_string()),
    )
}

#[derive(Event)]
struct Click(Vec2);

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

    cmds.spawn(Camera2d);

    cmds.spawn((
        Sprite::from_image(asset_server.load("pawn.png")),
        Transform::from_xyz(0.0, 0.0, 1.0)
            .with_scale(Vec3::splat(5.0 / TILE_SIZE)),
        Thing,
        Pawn {
            state: PawnState::Idle
        },
    ));
}

fn handle_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera: Single<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
    mut events: EventWriter<Click>,
    mut debug_log: Query<&mut DebugLog>,
) {
    if let Some(mut pos) = window.single().cursor_position() {
        pos = current_mouse_pos(pos, camera.1.translation, camera.0.scale);
        if mouse_button_input.just_pressed(MouseButton::Left) {
            events.send(Click(pos));
        }
        if let Ok(mut debug_log) = debug_log.get_single_mut() {
            debug_log.add(format!("MousePosition x:{:.2}, y:{:.2}", pos.x, pos.y));
        }
    } else {
        if let Ok(mut debug_log) = debug_log.get_single_mut() {
            debug_log.add("MousePosition None".to_string());
        }
    }
}

fn handle_right_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    camera: Single<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
    mut events: EventWriter<RightClick>,
) {
    if let Some(mut pos) = window.single().cursor_position() {
        pos = current_mouse_pos(pos, camera.1.translation, camera.0.scale);
        if mouse_button_input.just_pressed(MouseButton::Right) {
            events.send(RightClick(pos));
        }
    }
}

fn select_pawn(
    mut event: EventReader<Click>,
    selecters: Query<Entity, With<Selecter>>,
    things: Query<(Entity, &Transform), (With<Thing>, Without<Selecter>)>,
    hovered: Res<HoveredWhat>,
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Some(_pos) = event.read().next() {
        for entity in selecters.iter() {
            cmds.entity(entity).despawn();
        }
        // let pos = get_tile_pos(pos.0);
        let hovered = things.iter().find(|e| hovered.id.contains(&e.0.index()));
        if let Some((e, t)) = hovered{
            cmds.spawn((
                Sprite::from_image(asset_server.load("frame.png")),
                Transform::from_xyz(t.translation.x, t.translation.y, 1.5)
                    .with_scale(Vec3::splat(0.1)),
                Selecter(e.index()),
            ));
        }
    }
}

fn let_move_pawn(
    mut event: EventReader<RightClick>,
    mut pawns: Query<&mut Pawn>,
) {
    if let Some(pos) = event.read().next() {
        let pos = get_tile_pos(pos.0);
        for mut p in pawns.iter_mut() {
            match p.state {
                PawnState::Idle =>    p.state = PawnState::Move(pos),
                PawnState::Move(_) => p.state = PawnState::Move(pos),
            }
        }
    }
}

fn move_pawn(
    mut pawns: Query<(&mut Pawn, &mut Transform)>,
    time: Res<Time>,
) {
    for (p, mut t) in pawns.iter_mut() {
        if let PawnState::Move(mut pos) = p.state {
            if pos.x > pos.y {
                pos.x = 1f32;
                pos.y = pos.y / pos.x;
            } else {
                pos.y = 1f32;
                pos.x = pos.x / pos.y;
            }

            t.translation.x += pos.x * time.delta_secs();
            t.translation.y += pos.y * time.delta_secs();
        }
    };
}

fn move_selecter(
    mut selecters: Query<(&Selecter, &mut Transform), With<Selecter>>,
    things: Query<(Entity, &Transform), (With<Thing>, Without<Selecter>)>,
) {
    for (selecter, mut selecter_t) in selecters.iter_mut() {
        if let Some(t) = things.iter().find(|e| e.0.index() == selecter.0) {
            selecter_t.translation = t.1.translation;
        }
    }
}

fn hovered_what(
    mut res: ResMut<HoveredWhat>,
    things: Query<(Entity, &Transform), With<Thing>>,
    camera: Single<(&mut OrthographicProjection, &Transform), With<Camera2d>>,
    window: Query<&Window>,
    mut debug_log: Query<&mut DebugLog>,
) {
    if let Some(pos) = window.single().cursor_position() {
        let pos = current_mouse_pos(pos, camera.1.translation ,camera.0.scale);
        res.id.clear();

        for t in things.iter() {
            let x = t.1.translation.x;
            let y = t.1.translation.y;
            let half_size = TILE_SIZE / 2.0;

            if ((x - half_size)..(x + half_size)).contains(&pos.x)
            && ((y - half_size)..(y + half_size)).contains(&pos.y) {
                res.id.push(t.0.index());
            }
        }

        if let Ok(mut debug_log) = debug_log.get_single_mut() {
            debug_log.add(format!("HoveredWhat: {:?}", res.id));
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

    // debug!
    if let Ok(mut debug_log) = debug_log.get_single_mut() {
        debug_log.add(format!(
            "CameraPosition x:{:.2}, y:{:.2}",
            camera.translation.x,
            camera.translation.y
        ));
    };
}

fn zoom_camera(
    mut camera: Query<&mut OrthographicProjection, With<Camera2d>>,
    mut evr_scroll: EventReader<MouseWheel>,
    mut debug_log: Query<&mut DebugLog>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    let mut camera = camera.single_mut();
    use bevy::input::mouse::MouseScrollUnit;
    let evr_scroll_first = evr_scroll.read().next();
    if let Some(ev) = evr_scroll_first {
        if ev.unit == MouseScrollUnit::Pixel {
            let ds = ev.y * settings.camera_zoom_speed * time.delta_secs();
            let post_scale = camera.scale + ds;

            if 0.1 < post_scale && post_scale < 10.0 {
                camera.scale += ds;
            }
        }
    }
    // debug!
    if let Ok(mut debug_log) = debug_log.get_single_mut() {
        debug_log.add(format!("CameraScale: {:.2}", camera.scale));
    };
}

fn output_log(
    mut text: Query<&mut Text, With<DebugLog>>,
    mut buf: Query<&mut DebugLog>,
) {
    let text = text.get_single_mut();
    let buf = buf.get_single_mut();

    if let (Ok(mut text), Ok(mut buf)) = (text, buf) {
        text.0 = buf.0.clone();
        buf.0 = "---LOG---".to_string();
    }
}

fn show_hide_log(
    entity: Query<Entity, With<DebugLog>>,
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) { return }

    if let Ok(e) = entity.get_single() {
        cmds.entity(e).despawn();
    } else {
        cmds.spawn(new_debug_log(asset_server));
    }
}

fn close_on_q(
    mut cmds: Commands,
    window: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if let Ok((window, _focus)) = window.get_single() {
        if input.just_pressed(KeyCode::KeyQ) {
            cmds.entity(window).despawn();
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Wireframe2dPlugin,
        ))
        .insert_resource(Settings::default())
        .insert_resource(HoveredWhat::default())
        .add_event::<Click>()
        .add_event::<RightClick>()
        .add_systems(Startup, (
            setup,
            spawn_tile,
        ))
        .add_systems(Update, (
            handle_click,
            handle_right_click,
            select_pawn,
            move_selecter,
            let_move_pawn,
            move_pawn,
            hovered_what,
            move_camera,
            zoom_camera,
            output_log,
            show_hide_log,
            close_on_q,
        ))
        .run();
}
