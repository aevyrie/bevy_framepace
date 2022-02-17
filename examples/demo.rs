use std::time::Duration;

use bevy::{prelude::*, window::PresentMode};
use bevy_framepace::PowerSaver;
use bevy_mod_picking::{
    DebugCursorPickingPlugin, DefaultPickingPlugins, PickableBundle, PickingCameraBundle,
};

fn main() {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        present_mode: PresentMode::Immediate,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins)
    // Add the framepacing plugin.
    .add_plugin(bevy_framepace::FramepacePlugin {
        enabled: true,
        framerate_limit: bevy_framepace::FramerateLimit::Auto,
        warn_on_frame_drop: true,
        safety_margin: std::time::Duration::from_millis(2),
        power_saver: PowerSaver::Enabled(Duration::from_millis(500)),
    })
    // Picking and scene setup
    .add_plugins(DefaultPickingPlugins)
    .add_plugin(DebugCursorPickingPlugin)
    .add_startup_system(setup);
    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 25.0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..Default::default()
        })
        .insert_bundle(PickableBundle::default());
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(1.0, 5.0, 0.0),
        ..Default::default()
    });
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(1.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default());
}
