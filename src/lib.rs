//! This is a `bevy` plugin that adds framepacing and framelimiting to improve input latency and
//! power use.
//!
//! # How it works
//!
//! This works by sleeping the app immediately before the event loop starts. In doing so, this
//! minimizes the time from when user input is captured (start of event loop), to when the frame is
//! presented on screen. Graphically, it looks like this:
//!
//! ```none
//!           /-- latency --\             /-- latency --\
//!  sleep -> input -> render -> sleep -> input -> render
//!  \----- event loop -----/    \----- event loop -----/
//! ```
//!
//! One of the interesting benefits of this is that you can keep latency low even if the framerate
//! is limited to a low value. Assuming you are able to reach the target frametime, there should be
//! no difference in motion-to-photon latency when limited to 10fps or 120fps.
//!
//! ```none
//!                same                                              same
//!           /-- latency --\                                   /-- latency --\
//!  sleep -> input -> render -> sleeeeeeeeeeeeeeeeeeeeeeeep -> input -> render
//!  \----- event loop -----/    \---------------- event loop ----------------/
//!           60 fps                           limited to 10 fps
//! ```

#![deny(missing_docs)]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_platform::time::Instant;
use bevy_reflect::prelude::*;
use bevy_render::{Render, RenderApp, RenderSystems};

use bevy_ecs::system::NonSendMarker;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(feature = "framepace_debug")]
pub mod debug;

/// Adds frame-pacing and frame-limiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FramepaceSettings>();

        let limit = FrametimeLimit::default();
        let settings = FramepaceSettings::default();
        let settings_proxy = FramepaceSettingsProxy::default();
        let stats = FramePaceStats::default();

        app.insert_resource(settings)
            .insert_resource(settings_proxy.clone())
            .insert_resource(limit.clone())
            .insert_resource(stats.clone())
            .add_systems(Update, update_proxy_resources);

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, get_display_refresh_rate);

        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .insert_resource(settings_proxy)
            .insert_resource(limit)
            .insert_resource(stats)
            .add_systems(
                Render,
                framerate_limiter
                    .in_set(RenderSystems::Cleanup)
                    .after(World::clear_entities),
            );
    }
}

/// Framepacing plugin configuration.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
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

#[derive(Default, Debug, Clone, Resource)]
struct FramepaceSettingsProxy {
    /// Configures the framerate limiting strategy.
    limiter: Arc<Mutex<Limiter>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FramepaceSettingsProxy {
    fn is_enabled(&self) -> bool {
        read(&self.limiter).is_enabled()
    }
}

fn update_proxy_resources(settings: Res<FramepaceSettings>, proxy: Res<FramepaceSettingsProxy>) {
    if settings.is_changed() {
        write(&proxy.limiter, settings.limiter.clone());
    }
}

/// Configures the framelimiting technique for the app.
#[derive(Debug, Default, Clone, Reflect)]
pub enum Limiter {
    /// Uses the window's refresh rate to set the frametime limit, updating when the window changes
    /// monitors.
    #[default]
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
        match self {
            Limiter::Auto => write!(f, "Auto"),
            Limiter::Manual(t) => write!(f, "{:.2} fps", 1.0 / t.as_secs_f32()),
            Limiter::Off => write!(f, "Off"),
        }
    }
}

/// Current frametime limit based on settings and monitor refresh rate.
#[derive(Debug, Default, Clone, Resource)]
pub struct FrametimeLimit(pub Arc<Mutex<Duration>>);

/// Tracks the instant of the end of the previous frame.
#[derive(Debug, Clone, Resource, Reflect)]
pub struct FrameTimer {
    sleep_end: Instant,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            sleep_end: Instant::now(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_display_refresh_rate(
    settings: Res<FramepaceSettings>,
    windows: Query<Entity, With<bevy_window::Window>>,
    frame_limit: Res<FrametimeLimit>,
    non_send: NonSendMarker,
) {
    let new_frametime = match settings.limiter {
        Limiter::Auto => match detect_frametime(windows.iter(), non_send) {
            Some(frametime) => frametime,
            None => return,
        },
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            #[cfg(feature = "framepace_debug")]
            if settings.is_changed() {
                bevy_log::info!("Frame limiter disabled");
            }
            return;
        }
    };

    if read(&frame_limit.0) != new_frametime {
        #[cfg(feature = "framepace_debug")]
        bevy_log::info!("Frametime limit changed to: {:?}", new_frametime);
        write(&frame_limit.0, new_frametime);
    }
}

/// Autodetect frame time using winit. Must be called from the main thread, use NonSendMarker in the
/// calling system to ensure this.
#[cfg(not(target_arch = "wasm32"))]
fn detect_frametime(windows: impl Iterator<Item = Entity>, _: NonSendMarker) -> Option<Duration> {
    let best_framerate = {
        windows
            .filter_map(|e| {
                bevy_winit::WINIT_WINDOWS.with_borrow(|winit| {
                    winit
                        .get_window(e)?
                        .current_monitor()?
                        .refresh_rate_millihertz()
                })
            })
            .min()? as f64
            / 1000.0
            - 0.5 // Winit only provides integer refresh rate values. We need to round down to handle the worst case scenario of a rounded refresh rate.
    };

    let best_frametime = Duration::from_secs_f64(1.0 / best_framerate);
    Some(best_frametime)
}

/// Holds frame time measurements for framepacing diagnostics
#[derive(Clone, Debug, Default, Resource)]
pub struct FramePaceStats {
    frametime: Arc<Mutex<Duration>>,
    oversleep: Arc<Mutex<Duration>>,
}

/// Reads a value shared with another thread. A panic on the other thread does not make the value
/// invalid, so a poisoned lock is read anyway.
fn read<T: Clone>(shared: &Mutex<T>) -> T {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn write<T>(shared: &Mutex<T>, value: T) {
    *shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = value;
}

/// The time before a deadline that [`sleep_until`] spins for instead of sleeping.
#[cfg(not(target_arch = "wasm32"))]
const SPIN_TIME: Duration = Duration::from_micros(200);

/// Waits until `deadline`, to within a few microseconds.
///
/// Some platforms let a thread sleep longer than it asked, by up to a share of the time asked for;
/// on macOS a sleep can run half as long again. So this sleeps for a quarter of the time left,
/// checks the clock, and repeats, then spins through the last [`SPIN_TIME`].
#[cfg(not(target_arch = "wasm32"))]
fn sleep_until(deadline: Instant) {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left <= SPIN_TIME {
            break;
        }
        std::thread::sleep((left - SPIN_TIME) / 4);
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
    }
}

/// Waits until the frame limit has passed since the end of the previous frame.
///
/// A frame that already took longer than the limit does not wait, and the next frame is measured
/// from its end, so the time lost is not made up with a short frame. Frame limiting should not be
/// used in a browser, which limits frames itself.
#[allow(unused_variables)]
fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
    settings: Res<FramepaceSettingsProxy>,
) {
    let limit = read(&target_frametime.0);
    let frame_time = timer.sleep_end.elapsed();
    let deadline = timer.sleep_end + limit;
    #[cfg(not(target_arch = "wasm32"))]
    if settings.is_enabled() {
        sleep_until(deadline);
    }
    let sleep_end = Instant::now();
    timer.sleep_end = sleep_end;
    write(&stats.frametime, frame_time);
    write(
        &stats.oversleep,
        sleep_end.saturating_duration_since(deadline),
    );
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Runs one frame per entry of `work`, each spending that long before the limiter runs, and
    /// returns the time from the end of each frame's limiter to the end of the next one's.
    fn frame_times(limit: Duration, work: &[Duration]) -> Vec<Duration> {
        let mut world = World::new();
        world.insert_resource(FrameTimer::default());
        world.insert_resource(FrametimeLimit(Arc::new(Mutex::new(limit))));
        world.insert_resource(FramePaceStats::default());
        world.insert_resource(FramepaceSettingsProxy {
            limiter: Arc::new(Mutex::new(Limiter::Manual(limit))),
        });
        world.run_system_once(framerate_limiter).unwrap();
        let mut last = Instant::now();
        work.iter()
            .map(|work| {
                std::thread::sleep(*work);
                world.run_system_once(framerate_limiter).unwrap();
                let now = Instant::now();
                let frame = now - last;
                last = now;
                frame
            })
            .collect()
    }

    // On macOS a thread sleep runs up to half as long again as it asked, which a short spin at the
    // end cannot make up, so every limited frame ran long.
    #[test]
    fn a_wait_ends_within_a_millisecond_of_its_deadline() {
        for request in [1, 3, 6, 10, 14, 33] {
            for _ in 0..10 {
                let deadline = Instant::now() + ms(request);
                sleep_until(deadline);
                let late = Instant::now() - deadline;
                assert!(
                    late < ms(1),
                    "a {request} ms wait ended {late:?} after its deadline"
                );
            }
        }
    }

    // A frame that ran long, from a slow sleep or a wait for the display, made the limiter cut the
    // next frame's sleep by the whole overrun, so limited frames alternated long and short.
    #[test]
    fn a_long_frame_is_not_followed_by_a_short_one() {
        let limit = ms(16);
        let mut work = vec![ms(2); 12];
        work[5] = ms(24);
        let frames = frame_times(limit, &work);
        for (index, frame) in frames.iter().enumerate().skip(1) {
            if index == 5 {
                continue;
            }
            assert!(
                *frame >= limit - ms(1) && *frame <= limit + ms(1),
                "frame {index} took {frame:?} against a limit of {limit:?}: {frames:?}"
            );
        }
    }
}
