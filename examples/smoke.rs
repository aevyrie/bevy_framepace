use bevy::{diagnostic::DiagnosticsStore, prelude::*, winit::WinitSettings};
use bevy_framepace::{
    FramepacePlugin, FramepaceSettings, FrametimeLimit, Limiter, debug::DiagnosticsPlugin,
};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FramepacePlugin, DiagnosticsPlugin))
        .insert_resource(WinitSettings::game())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, exercise_limiters)
        .run();
}

fn exercise_limiters(
    mut frames: Local<u32>,
    mut settings: ResMut<FramepaceSettings>,
    limit: Res<FrametimeLimit>,
    diagnostics: Res<DiagnosticsStore>,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;
    match *frames {
        60 => {
            assert!(matches!(settings.limiter, Limiter::Auto));
            settings.limiter = Limiter::from_framerate(30.0);
        }
        120 => {
            assert_eq!(
                *limit.0.lock().unwrap(),
                Duration::from_secs_f64(1.0 / 30.0)
            );
            settings.limiter = Limiter::Off;
        }
        180 => {
            for path in [
                DiagnosticsPlugin::FRAMEPACE_FRAMETIME,
                DiagnosticsPlugin::FRAMEPACE_OVERSLEEP,
            ] {
                let value = diagnostics
                    .get(&path)
                    .and_then(|diagnostic| diagnostic.value());
                assert!(value.is_some_and(|value| value.is_finite() && value >= 0.0));
            }
            println!("Frame pacing smoke test passed after 180 frames");
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
