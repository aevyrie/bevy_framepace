//! This is a [`bevy`] plugin that adds framepacing and framelimiting to improve input latency and
//! power use.
//!
//! # How it works
//!
//! This works by sleeping the app immediately before the event loop starts. In doing so, this
//! minimizes the time from when user input is captured (start of event loop), to when the rendered
//! frame using this input data is presented (`RenderStage::Render`). Because the event loop is,
//! well, a loop, it is equally accurate to think of this as sleeping at the beginning of the frame,
//! before input is captured. Graphically, it looks like this:
//!
//! ```none
//!  /-- latency --\             /-- latency --\
//!  input -> render -> sleep -> input -> render -> sleep
//!  \----- event loop -----/    \----- event loop -----/
//! ```
//!
//! One of the interesting benefits of this is that you can keep latency low even if the framerate
//! is limited to a low value. Assuming you are able to reach the target frametime, there should be
//! no difference in motion-to-photon latency when limited to 10fps or 120fps.
//!
//! ```none
//!      same                        same
//!  /-- latency --\             /-- latency --\
//!  input -> render -> sleep    input -> render -> sleeeeeeeeeeeeeeeeeeeeeeeep
//!  \----- event loop -----/    \---------------- event loop ----------------/
//!           60 fps                           limited to 10 fps
//! ```

#![deny(missing_docs)]

#[cfg(not(target_arch = "wasm32"))]
use bevy::winit::WinitWindows;
use bevy::{
    ecs::schedule::ShouldRun,
    prelude::*,
    render::{Extract, RenderApp, RenderStage},
    utils::Instant,
};

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(feature = "debug")]
mod debug;

/// Adds framepacing and framelimiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FramepaceSettings>()
            .init_resource::<FrametimeLimit>()
            .init_resource::<FramePaceStats>();

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system_to_stage(CoreStage::Update, get_display_refresh_rate);
        #[cfg(feature = "debug")]
        app.add_plugin(debug::FramePaceDiagnosticsPlugin);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_resources)
            .add_system_to_stage(
                RenderStage::Cleanup,
                // We need this system to run at the end, immediately before the event loop restarts
                framerate_limiter
                    .at_end()
                    .with_run_criteria(|settings: Res<FramepaceSettings>| {
                        if settings.limiter.is_enabled() {
                            ShouldRun::Yes
                        } else {
                            ShouldRun::No
                        }
                    }),
            );
    }
}

/// Framepacing plugin configuration.
#[derive(Debug, Clone, Resource)]
pub struct FramepaceSettings {
    /// Configures the framerate limiting strategy.
    pub limiter: Limiter,
}
impl FramepaceSettings {
    /// Builds plugin settings with the specified [`Limiter`] configuration.
    pub fn with_limiter(mut self, limiter: Limiter) -> Self {
        self.limiter = limiter;
        self
    }
}
impl Default for FramepaceSettings {
    fn default() -> FramepaceSettings {
        FramepaceSettings {
            limiter: Limiter::Auto,
        }
    }
}

/// Configures the framelimiting technique for the app.
#[derive(Debug, Clone)]
pub enum Limiter {
    /// Uses the window's refresh rate to set the frametime limit, updating when the window changes
    /// monitors.
    Auto,
    /// Set a fixed manual frametime limit. This should be greater than the monitors frametime
    /// (`1.0 / monitor frequency`).
    Manual(Duration),
    /// Disables frame limiting
    Off,
}

impl Limiter {
    /// Returns `true` if the [`Limiter`] is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Limiter::Off)
    }

    /// Constructs a new [`Limiter`] from the provided `framerate`.
    pub fn from_framerate(framerate: f64) -> Self {
        Limiter::Manual(Duration::from_secs_f64(1.0 / framerate))
    }
}

impl std::fmt::Display for Limiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            Limiter::Auto => "Auto".into(),
            Limiter::Manual(t) => format!("{:.2} fps", 1.0 / t.as_secs_f32()),
            Limiter::Off => "Off".into(),
        };
        write!(f, "{}", output)
    }
}

/// Current frametime limit based on settings and monitor refresh rate.
#[derive(Debug, Default, Clone, Reflect, Resource)]
pub struct FrametimeLimit(Duration);

/// Tracks the instant of the end of the previous frame.
#[derive(Debug, Resource)]
pub struct FrameTimer {
    render_end: Instant,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            render_end: Instant::now(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_display_refresh_rate(
    settings: Res<FramepaceSettings>,
    winit: NonSend<WinitWindows>,
    windows: Res<Windows>,
    mut frame_limit: ResMut<FrametimeLimit>,
) {
    if !settings.is_changed() && !winit.is_changed() {
        return;
    }
    let new_frametime = match settings.limiter {
        Limiter::Auto => match detect_frametime(winit, windows) {
            Some(frametime) => frametime,
            None => return,
        },
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            info!("Frame limiter disabled");
            return;
        }
    };

    if new_frametime != frame_limit.0 {
        info!("Frametime limit changed to: {:?}", new_frametime);
        frame_limit.0 = new_frametime;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_frametime(winit: NonSend<WinitWindows>, windows: Res<Windows>) -> Option<Duration> {
    let best_framerate = {
        let monitor = winit
            .get_window(windows.get_primary()?.id())?
            .current_monitor()?;

        // We need to subtract 0.5 because winit only reads framerate to the nearest 1 hertz. To
        // prevent frames building up, adding latency, we need to use the most conservative possible
        // refresh rate that could round up to the integer value reported by winit.
        bevy::winit::get_best_videomode(&monitor).refresh_rate_millihertz() as f64 / 1000.0 - 0.5
    };

    let best_frametime = Duration::from_secs_f64(1.0 / best_framerate);
    Some(best_frametime)
}

fn extract_resources(
    mut commands: Commands,
    settings: Extract<Res<FramepaceSettings>>,
    framerate_limit: Extract<Res<FrametimeLimit>>,
    stats: Extract<Res<FramePaceStats>>,
) {
    commands.insert_resource(settings.to_owned());
    commands.insert_resource(framerate_limit.to_owned());
    commands.insert_resource(stats.to_owned());
}

/// Holds frame time measurements for framepacing diagnostics
#[derive(Clone, Debug, Resource)]
pub struct FramePaceStats {
    frametime: Arc<Mutex<Duration>>,
    oversleep: Arc<Mutex<Duration>>,
}

impl Default for FramePaceStats {
    fn default() -> Self {
        Self {
            frametime: Default::default(),
            oversleep: Default::default(),
        }
    }
}

/// Accurately sleeps until it's time to start the next frame.
///
/// The `spin_sleep` dependency makes it possible to get extremely accurate sleep times across
/// platforms. Using `std::thread::sleep()` will not be precise enough, especially windows. Using a
/// spin lock, even with `std::hint::spin_loop()`, will result in significant power usage.
///
/// `spin_sleep` sleeps as long as possible given the platform's sleep accuracy, and spins for the
/// remainder. The dependency is however not WASM compatible, which is fine, because frame limiting
/// should not be used in a browser; this would compete with the browser's frame limiter.
pub fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    spin_sleep::sleep(
        target_frametime
            .0
            .saturating_sub(timer.render_end.elapsed()),
    );

    let frame_time_actual = timer.render_end.elapsed();
    *stats.frametime.try_lock().unwrap() = frame_time_actual;
    *stats.oversleep.try_lock().unwrap() = frame_time_actual.saturating_sub(target_frametime.0);
    timer.render_end = Instant::now();
}
