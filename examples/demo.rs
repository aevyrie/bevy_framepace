use bevy::prelude::*;

const WINDOWED_TOGGLE_KEY  : KeyCode = KeyCode::W;
const VSYNC_TOGGLE_KEY     : KeyCode = KeyCode::V;
const FRAMEPACE_TOGGLE_KEY : KeyCode = KeyCode::Space;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_framepace::debug::DiagnosticsPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_window_mode
            .run_if(|key_pressed: Res<Input<KeyCode>>| key_pressed.just_pressed(WINDOWED_TOGGLE_KEY)))
        .add_systems(Update, toggle_vsync
            .run_if(|key_pressed: Res<Input<KeyCode>>| key_pressed.just_pressed(VSYNC_TOGGLE_KEY)))
        .add_systems(Update, toggle_framepace
            .run_if(|key_pressed: Res<Input<KeyCode>>| key_pressed.just_pressed(FRAMEPACE_TOGGLE_KEY)))
        .add_systems(Update, update_ui_limiter)
        .add_systems(Update, update_ui_vsync)
        .add_systems(Update, update_ui_windowmode)
        .add_systems(Update, update_cursor)
        .run();
}

fn toggle_window_mode(mut window: Query<&mut Window, With<bevy::window::PrimaryWindow>>)
{
    let window: &mut Window = &mut window.single_mut();
    if window.mode == bevy::window::WindowMode::Windowed
        { window.mode = bevy::window::WindowMode::SizedFullscreen; }
    else
        { window.mode = bevy::window::WindowMode::Windowed; }
}

fn toggle_vsync(mut window: Query<&mut Window, With<bevy::window::PrimaryWindow>>)
{
    let window: &mut Window = &mut window.single_mut();
    if window.present_mode == bevy::window::PresentMode::AutoNoVsync
        { window.present_mode = bevy::window::PresentMode::AutoVsync; }
    else
        { window.present_mode = bevy::window::PresentMode::AutoNoVsync; }
}

#[derive(Component)]
struct EnableLimiterText;

#[derive(Component)]
struct EnableVsyncText;

#[derive(Component)]
struct WindowModeText;

fn toggle_framepace(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    use bevy_framepace::Limiter;
    settings.limiter = match settings.limiter {
        Limiter::Auto => Limiter::Off,
        Limiter::Off => Limiter::from_framerate(30.0),
        Limiter::Manual(_) => Limiter::Auto,
    }
}

fn update_ui_limiter(
    mut text: Query<&mut Text, With<EnableLimiterText>>,
    settings: Res<bevy_framepace::FramepaceSettings>,
) {
    text.single_mut().sections[1].value = format!("{}", settings.limiter);
}

pub fn update_cursor(windows: Query<&Window>, mut gizmos: bevy::gizmos::gizmos::Gizmos) {
    if let Some(pos) = windows.single().cursor_position() {
        let pos = Vec2::new(
            pos.x - windows.single().width() / 2.0,
            windows.single().height() / 2.0 - pos.y,
        );
        gizmos.circle_2d(pos, 10.0, Color::GREEN);
    }
}

fn update_ui_vsync(
    mut text: Query<&mut Text, With<EnableVsyncText>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let window: &Window = &window.single();
    text.single_mut().sections[1].value =
        match window.present_mode {
            bevy::window::PresentMode::AutoVsync => String::from("ON"),
            _                                    => String::from("OFF")
        };
}

fn update_ui_windowmode(
    mut text: Query<&mut Text, With<WindowModeText>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let window: &Window = &window.single();
    text.single_mut().sections[1].value =
        match window.mode {
            bevy::window::WindowMode::Windowed             => String::from("Windowed"),
            bevy::window::WindowMode::SizedFullscreen      => String::from("SizedFullscreen"),
            bevy::window::WindowMode::Fullscreen           => String::from("Fullscreen"),
            bevy::window::WindowMode::BorderlessFullscreen => String::from("BorderlessFullscreen"),
        };
}

/// set up the scene
fn setup(mut commands: Commands, mut windows: Query<&mut Window>) {
    windows.iter_mut().next().unwrap().cursor.icon = CursorIcon::Crosshair;
    commands.spawn((Camera2dBundle {
        camera: Camera {
            order: 10,
            ..default()
        },
        ..default()
    },));
    commands.spawn((Camera3dBundle::default(),));
    // UI
    let style = TextStyle {
        font_size: 60.0,
        color: Color::WHITE,
        ..default()
    };
    commands.spawn((
        TextBundle::from_sections(vec![
            TextSection {
                value: "Frame pacing: ".to_string(),
                style: style.clone(),
            },
            TextSection {
                value: "".to_string(),
                style: style.clone(),
            },
            TextSection {
                value: "\n[press space]".to_string(),
                style: style.clone(),
            },
        ]),
        EnableLimiterText,
    ));

    commands.spawn((
        Text2dBundle{
            text: Text::from_sections(vec![
                TextSection {
                    value: "Vsync [v]: ".to_string(),
                    style: style.clone(),
                },
                TextSection {
                    value: "".to_string(),
                    style: style.clone(),
                },
            ]),
            transform: Transform::from_xyz(-100., -100., 0.),
            ..default()
        },
        EnableVsyncText,
    ));

    commands.spawn((
        Text2dBundle{
            text: Text::from_sections(vec![
                TextSection {
                    value: "Window mode [w]: ".to_string(),
                    style: style.clone(),
                },
                TextSection {
                    value: "".to_string(),
                    style: style.clone(),
                },
            ]),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        WindowModeText,
    ));
}
