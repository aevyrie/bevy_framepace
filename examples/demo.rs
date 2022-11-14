use bevy::{diagnostic::LogDiagnosticsPlugin, prelude::*, render::camera::Projection};
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, PickableBundle, PickingCameraBundle, PickingPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the framepacing plugin
        .add_plugin(FramepacePlugin)
        // Our systems for this demo
        .add_startup_system(setup)
        .add_system(toggle_plugin)
        .add_system(update_ui)
        // Mouse picking to visualize latency
        .add_plugin(PickingPlugin)
        .add_plugin(DebugCursorPickingPlugin)
        // Log framepace custom bevy diagnostics to stdout
        .add_plugin(LogDiagnosticsPlugin::default())
        .run();
}

#[derive(Component)]
struct EnableText;

fn toggle_plugin(mut settings: ResMut<FramepaceSettings>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        settings.limiter = match settings.limiter {
            Limiter::Auto => Limiter::Off,
            Limiter::Off => Limiter::from_framerate(30.0),
            Limiter::Manual(_) => Limiter::Auto,
        }
    }
}

fn update_ui(mut text: Query<&mut Text, With<EnableText>>, settings: Res<FramepaceSettings>) {
    text.single_mut().sections[1].value = format!("{}", settings.limiter);
}

/// set up the scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut windows: ResMut<Windows>,
    asset_server: Res<AssetServer>,
) {
    windows
        .get_primary_mut()
        .unwrap()
        .set_cursor_icon(CursorIcon::Crosshair);
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 25.0 })),
            material: materials.add(Color::BLACK.into()),
            ..Default::default()
        },
        PickableBundle::default(),
    ));
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.01,
                ..Default::default()
            }),
            ..Default::default()
        },
        PickingCameraBundle::default(),
    ));
    // UI
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let style = TextStyle {
        font,
        font_size: 40.0,
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
                        value: " Vsync: On\n Frame pacing: ".to_string(),
                        style: style.clone(),
                    },
                    TextSection {
                        value: "".to_string(),
                        style: style.clone(),
                    },
                    TextSection {
                        value: "\n [press space to switch]".to_string(),
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
