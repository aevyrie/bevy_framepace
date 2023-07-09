use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bevy_framepace::FramepacePlugin,
            bevy_framepace::debug::CursorPlugin, // Optional
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera3dBundle::default(),));
}
