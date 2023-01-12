use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        // Uncomment the next line to test in reactive rendering mode
        // .insert_resource(bevy::winit::WinitSettings::desktop_app())
        // Add the framepacing plugin
        .add_plugin(bevy_framepace::FramepacePlugin)
        // Our systems for this demo
        .add_startup_system(setup)
        .add_system(toggle_plugin)
        .add_system(update_ui)
        // Log framepace custom bevy diagnostics to stdout
        .add_plugin(LogDiagnosticsPlugin::default())
        .run();
}

#[derive(Component)]
struct EnableText;

fn toggle_plugin(
    mut settings: ResMut<bevy_framepace::FramepaceSettings>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        use bevy_framepace::Limiter;
        settings.limiter = match settings.limiter {
            Limiter::Auto => Limiter::Off,
            Limiter::Off => Limiter::from_framerate(20.0),
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

/// set up the scene
fn setup(mut commands: Commands, mut windows: ResMut<Windows>, asset_server: Res<AssetServer>) {
    windows
        .get_primary_mut()
        .unwrap()
        .set_cursor_icon(CursorIcon::Crosshair);
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    // UI
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let style = TextStyle {
        font,
        font_size: 60.0,
        color: Color::WHITE,
    };
    commands.spawn((
        TextBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![
                    TextSection {
                        value: " Frame pacing: ".to_string(),
                        style: style.clone(),
                    },
                    TextSection {
                        value: "".to_string(),
                        style: style.clone(),
                    },
                    TextSection {
                        value: "\n [press space]".to_string(),
                        style,
                    },
                ],
                ..default()
            },
            ..default()
        },
        EnableText,
    ));
}
