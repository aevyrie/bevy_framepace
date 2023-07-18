use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, bevy_framepace::FramepacePlugin))
        .run();
}
