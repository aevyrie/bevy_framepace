//! This is a [`bevy`] plugin that adds framepacing and framelimiting to improve input latency and
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

#[cfg(feature = "window")]
use bevy::render::{pipelined_rendering::RenderExtractApp, RenderApp, RenderSet};
#[cfg(all(feature = "window", not(target_arch = "wasm32")))]
use bevy::winit::WinitWindows;
use bevy::{prelude::*, utils::Instant};

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(feature = "framepace_debug")]
pub mod debug;

/// Adds framepacing and framelimiting functionality to your [`App`].
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
        app.add_systems(Update, update_frametime);

        #[cfg(feature = "window")]
        if let Ok(sub_app) = app.get_sub_app_mut(RenderExtractApp) {
            sub_app
                .insert_resource(FrameTimer::default())
                .insert_resource(settings_proxy)
                .insert_resource(limit)
                .insert_resource(stats)
                .add_systems(Main, framerate_limiter);
        } else {
            app.sub_app_mut(RenderApp)
                .insert_resource(FrameTimer::default())
                .insert_resource(settings_proxy)
                .insert_resource(limit)
                .insert_resource(stats)
                .add_systems(
                    bevy::render::Render,
                    framerate_limiter
                        .in_set(RenderSet::Cleanup)
                        .after(World::clear_entities),
                );
        }
        #[cfg(not(feature = "window"))]
        app.insert_resource(FrameTimer::default())
            .add_systems(Last, framerate_limiter);
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

impl FramepaceSettingsProxy {
    fn is_enabled(&self) -> bool {
        self.limiter.try_lock().iter().any(|l| l.is_enabled())
    }
}

fn update_proxy_resources(settings: Res<FramepaceSettings>, proxy: Res<FramepaceSettingsProxy>) {
    if settings.is_changed() {
        if let Ok(mut limiter) = proxy.limiter.try_lock() {
            *limiter = settings.limiter.clone();
        }
    }
}

/// Configures the framelimiting technique for the app.
#[derive(Debug, Default, Clone, Reflect)]
pub enum Limiter {
    /// A sane default for the frametime limit.
    ///
    /// When the `"window"` feature is enabled, this uses the window's refresh rate
    /// to set the frametime limit, updating when the window changes monitors.
    #[default]
    Auto,
    /// Set a fixed manual frametime limit.
    ///
    /// When the `"window"` feature is enabled, this should be greater than the monitor's frametime
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
#[derive(Debug, Default, Clone, Resource)]
struct FrametimeLimit(Arc<Mutex<Duration>>);

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
fn update_frametime(
    settings: Res<FramepaceSettings>,
    #[cfg(feature = "window")] (winit, windows): (
        NonSend<WinitWindows>,
        Query<Entity, With<Window>>,
    ),
    frame_limit: Res<FrametimeLimit>,
) {
    let changed = settings.is_changed();
    #[cfg(feature = "window")]
    let changed = changed || winit.is_changed();

    if !changed {
        return;
    }
    let new_frametime = match settings.limiter {
        Limiter::Auto => {
            #[cfg(feature = "window")]
            match detect_frametime(winit, windows.iter()) {
                Some(frametime) => frametime,
                None => return,
            }
            #[cfg(not(feature = "window"))]
            {
                /// A sane, conservative framerate that shouldn't burden the CPU in a tui.
                const DEFAULT_TERMINAL_FPS: f64 = 15.;
                Duration::from_secs_f64(1.0 / DEFAULT_TERMINAL_FPS)
            }
        }
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            #[cfg(feature = "framepace_debug")]
            info!("Frame limiter disabled");
            return;
        }
    };

    if let Ok(mut limit) = frame_limit.0.try_lock() {
        if new_frametime != *limit {
            #[cfg(feature = "framepace_debug")]
            info!("Frametime limit changed to: {:?}", new_frametime);
            *limit = new_frametime;
        }
    }
}

#[cfg(all(feature = "window", not(target_arch = "wasm32")))]
fn detect_frametime(
    winit: NonSend<WinitWindows>,
    windows: impl Iterator<Item = Entity>,
) -> Option<Duration> {
    let best_framerate = {
        windows
            .filter_map(|e| winit.get_window(e))
            .filter_map(|w| w.current_monitor())
            .map(|monitor| bevy::winit::get_best_videomode(&monitor).refresh_rate_millihertz())
            .min()? as f64
            / 1000.0
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

/// Accurately sleeps until it's time to start the next frame.
///
/// The `spin_sleep` dependency makes it possible to get extremely accurate sleep times across
/// platforms. Using `std::thread::sleep()` will not be precise enough, especially windows. Using a
/// spin lock, even with `std::hint::spin_loop()`, will result in significant power usage.
///
/// `spin_sleep` sleeps as long as possible given the platform's sleep accuracy, and spins for the
/// remainder. The dependency is however not WASM compatible, which is fine, because frame limiting
/// should not be used in a browser; this would compete with the browser's frame limiter.
fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
    settings: Res<FramepaceSettingsProxy>,
) {
    if let Ok(limit) = target_frametime.0.try_lock() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let oversleep = stats
                .oversleep
                .try_lock()
                .as_deref()
                .cloned()
                .unwrap_or_default();
            let sleep_time = limit.saturating_sub(timer.sleep_end.elapsed() + oversleep);
            if settings.is_enabled() {
                spin_sleep::sleep(sleep_time);
            }
        }

        let frame_time_actual = timer.sleep_end.elapsed();
        timer.sleep_end = Instant::now();
        if let Ok(mut frametime) = stats.frametime.try_lock() {
            *frametime = frame_time_actual;
        }
        if let Ok(mut oversleep) = stats.oversleep.try_lock() {
            *oversleep = frame_time_actual.saturating_sub(*limit);
        }
    };
}
