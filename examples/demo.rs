use bevy::{prelude::*, window::PresentMode, winit::WinitSettings};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, DefaultPickingPlugins, PickableBundle, PickingCameraBundle,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Mailbox,
            ..Default::default()
        })
        // Add the framepacing plugin.
        .add_plugin(bevy_framepace::FramepacePlugin {
            enabled: true,
            framerate_limit: bevy_framepace::FramerateLimit::Auto,
            warn_on_frame_drop: true,
            safety_margin: std::time::Duration::from_micros(50),
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
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default());
}
