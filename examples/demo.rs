use bevy::{prelude::*, render::camera::Projection};
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, PickableBundle, PickingCameraBundle, PickingPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the framepacing plugin.
        .add_plugin(FramepacePlugin)
        .insert_resource(FramepaceSettings::default().with_warnings(true))
        // Our systems for this demo
        .add_startup_system(setup)
        .add_system(toggle_plugin)
        .add_system(update_ui)
        // Mouse picking to visualize latency
        .add_plugin(PickingPlugin)
        .add_plugin(DebugCursorPickingPlugin)
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
    text.single_mut().sections[1].value = format!("{:?}", settings.limiter);
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
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 25.0 })),
            material: materials.add(Color::GRAY.into()),
            ..Default::default()
        })
        .insert_bundle(PickableBundle::default());
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.01,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default());
    // UI
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextStyle {
        font: font,
        font_size: 40.0,
        color: Color::WHITE,
    };
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
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
        })
        .insert(EnableText);
}
