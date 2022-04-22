use bevy::{prelude::*, window::PresentMode};
use bevy_framepace::{FramepacePlugin, FramerateLimit};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, PickableBundle, PickingCameraBundle, PickingPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(PresentMode::Mailbox)
        // Add the framepacing plugin.
        .add_plugin(FramepacePlugin::default())
        // Our systems for this demo
        .add_startup_system(setup)
        .add_system(toggle_plugin)
        .add_system(update_ui)
        // Mouse picking to visualize latency
        .add_plugin(PickingPlugin)
        .add_plugin(DebugCursorPickingPlugin)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 25.0 })),
            material: materials.add(Color::BLACK.into()),
            ..Default::default()
        })
        .insert_bundle(PickableBundle::default());
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
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
    commands.spawn_bundle(UiCameraBundle::default());
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
                        value: " Press space to switch mode: ".to_string(),
                        style: style.clone(),
                    },
                    TextSection {
                        value: "".to_string(),
                        style,
                    },
                ],
                ..default()
            },
            ..default()
        })
        .insert(EnableText);
}

#[derive(Component)]
struct EnableText;

fn toggle_plugin(mut plugin: ResMut<FramepacePlugin>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        plugin.framerate_limit = match plugin.framerate_limit {
            FramerateLimit::Auto => FramerateLimit::Off,
            FramerateLimit::Off => FramerateLimit::Manual(15),
            FramerateLimit::Manual(_) => FramerateLimit::Auto,
        }
    }
}

fn update_ui(mut text: Query<&mut Text, With<EnableText>>, plugin: Res<FramepacePlugin>) {
    text.single_mut().sections[1].value = format!("{:?}", plugin.framerate_limit);
}
