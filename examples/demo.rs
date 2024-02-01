use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
            bevy_framepace::FramepacePlugin,
            bevy_framepace::debug::DiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_plugin, update_ui, update_cursor))
        .run();
}

#[derive(Component)]
struct EnableText;

fn toggle_plugin(
    mut settings: ResMut<bevy_framepace::FramepaceSettings>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        use bevy_framepace::Limiter;
        settings.limiter = match settings.limiter {
            Limiter::Auto => Limiter::Off,
            Limiter::Off => Limiter::from_framerate(30.0),
            Limiter::Manual(_) => Limiter::Auto,
        }
    }
}

fn update_ui(
    mut text: Query<&mut Text, With<EnableText>>,
    settings: Res<bevy_framepace::FramepaceSettings>,
) {
    text.single_mut().sections[1].value = format!("{}", settings.limiter);
}

pub fn update_cursor(windows: Query<&Window>, mut gizmos: Gizmos) {
    if let Some(pos) = windows.single().cursor_position() {
        let pos = Vec2::new(
            pos.x - windows.single().width() / 2.0,
            windows.single().height() / 2.0 - pos.y,
        );
        gizmos.circle_2d(pos, 10.0, Color::GREEN);
    }
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
                style,
            },
        ]),
        EnableText,
    ));
}
