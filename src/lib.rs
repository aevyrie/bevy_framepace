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

#[cfg(not(target_arch = "wasm32"))]
use bevy::winit::WinitWindows;
use bevy::{
    prelude::*,
    render::{pipelined_rendering::RenderExtractApp, RenderApp, RenderSet},
    utils::Instant,
};

use std::{
    collections::VecDeque,
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(feature = "framepace_debug")]
pub mod debug;

const FRAMEPACE_MAX_FRAME_RECORDS: u16 = 20;
const FRAMEPACE_PID_KP: f32 = 1.;
const FRAMEPACE_PID_KI: f32 = 0.;
const FRAMEPACE_PID_KD: f32 = 0.;

/// Adds framepacing and framelimiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FramepaceSettings>();

        let limit          = FrametimeLimit::default();
        let settings       = FramepaceSettings::default();
        let settings_proxy = FramepaceSettingsProxy::default();
        let stats          = 
            FramePaceStats::new(
                FRAMEPACE_MAX_FRAME_RECORDS,
                FRAMEPACE_PID_KP,
                FRAMEPACE_PID_KI,
                FRAMEPACE_PID_KD
            );

        app.insert_resource(settings)
            .insert_resource(settings_proxy.clone())
            .insert_resource(limit.clone())
            .insert_resource(stats.clone())
            .add_systems(Update, update_proxy_resources);

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, get_display_refresh_rate);

        let Ok(render_extract_app) = app.get_sub_app_mut(RenderExtractApp)
        else {
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
            return;
        };

        render_extract_app.insert_resource(FrameTimer::default())
            .insert_resource(settings_proxy)
            .insert_resource(limit)
            .insert_resource(stats)
            .add_system(
                framerate_limiter
                    .run_if(|settings: Res<FramepaceSettingsProxy>| settings.is_enabled()),
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
        let output = match self {
            Limiter::Auto      => "Auto".into(),
            Limiter::Manual(t) => format!("{:.2} fps", 1.0 / t.as_secs_f32()),
            Limiter::Off       => "Off".into(),
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
fn get_display_refresh_rate(
    settings: Res<FramepaceSettings>,
    winit: NonSend<WinitWindows>,
    windows: Query<Entity, With<Window>>,
    frame_limit: Res<FrametimeLimit>,
) {
    if !(settings.is_changed() || winit.is_changed()) {
        return;
    }
    let new_frametime = match settings.limiter {
        Limiter::Auto =>
            match detect_frametime(winit, windows.iter()) {
                Some(frametime) => frametime,
                None            => return,
            },
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            #[cfg(feature = "framepace_debug")]
            info!("Frame limiter disabled");
            return;
        }
    };

    let Ok(mut limit) = frame_limit.0.try_lock() else { return };
    if new_frametime == *limit { return }

    #[cfg(feature = "framepace_debug")]
    info!("Frametime limit changed to: {:?}", new_frametime);
    *limit = new_frametime;
}

#[cfg(not(target_arch = "wasm32"))]
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
    frametime_record: Arc<Mutex<VecDeque<Duration>>>,
    oversleep_record: Arc<Mutex<VecDeque<Duration>>>,
    oversleep_sum:    Arc<Mutex<f64>>,

    max_frame_records: u16,

    pid_kp: f32,
    pid_ki: f32,
    pid_kd: f32,
}

impl FramePaceStats {
    fn new(max_frame_records: u16, pid_kp: f32, pid_ki: f32, pid_kd: f32) -> FramePaceStats {
        #[cfg(not(target_arch = "wasm32"))]
        if max_frame_records == 0 { panic!("framepace: max frame records is zero!"); }

        let mut stats = FramePaceStats::default();
        stats.max_frame_records = max_frame_records;
        stats.pid_kp            = pid_kp;
        stats.pid_ki            = pid_ki;
        stats.pid_kd            = pid_kd;
        stats
    }

    fn add_frame_stats(&self, new_frametime: Duration, new_oversleep: Duration) {
        let Ok(mut frametime_record) = self.frametime_record.try_lock() else { return };
        let Ok(mut oversleep_record) = self.oversleep_record.try_lock() else { return };
        let Ok(mut oversleep_sum)    = self.oversleep_sum.try_lock()    else { return };
        frametime_record.push_back(new_frametime);
        oversleep_record.push_back(new_oversleep);
        *oversleep_sum = *oversleep_sum + new_oversleep.as_secs_f64();

        while frametime_record.len() > (self.max_frame_records as usize) {
            frametime_record.pop_front();
        }
        while oversleep_record.len() > (self.max_frame_records as usize) {
            *oversleep_sum -= oversleep_record.get(0).unwrap().as_secs_f64();
            oversleep_record.pop_front();
        }
    }

    fn get_oversleep_sum(&self) -> f64 {
        let Ok(oversleep_sum) = self.oversleep_sum.try_lock() else { return 0f64 };
        *oversleep_sum
    }

    fn get_last_oversleep_delta(&self) -> f64 {
        let Ok(oversleep_record) = self.oversleep_record.try_lock() else { return 0f64 };
        match oversleep_record.len()
        {
            0 => 0f64,
            1 => 0f64 - oversleep_record.back().unwrap().as_secs_f64(),
            _ => {
                oversleep_record.get(oversleep_record.len() - 2).unwrap().as_secs_f64() -
                    oversleep_record.back().unwrap().as_secs_f64()
            }
        }
    }

    fn get_requested_sleep_duration(&self, target_frametime: Duration, already_elapsed_time: Duration) -> Duration {
        // remaining_time = target - elapsed
        // sleep_duration = remaining_time - adjustment
        // adjustment = k_p * [previous oversleep] + k_i * [sum(previous oversleeps)] + k_d * [delta(previous two oversleeps)]
        let remaining_time = target_frametime.saturating_sub(already_elapsed_time).as_secs_f64();
        if remaining_time == 0.0 { return Duration::default() }

        let mut adjustment = 0f64;
        adjustment = adjustment + (self.pid_kp as f64) * self.get_last_frame_oversleep().as_secs_f64();
        adjustment = adjustment + (self.pid_ki as f64) * self.get_oversleep_sum();
        adjustment = adjustment + (self.pid_kd as f64) * self.get_last_oversleep_delta();

        let sleep_duration = remaining_time - adjustment;
        if sleep_duration <= 0.0 { return Duration::default() }
        Duration::from_secs_f64(sleep_duration)
    }

    /// Get the frame time of the last frame.
    pub fn get_last_frame_time(&self) -> Duration {
        let Ok(frametime_record)        = self.frametime_record.try_lock() else { return Duration::default() };
        let Some(most_recent_frametime) = frametime_record.back()          else { return Duration::default() };
        *most_recent_frametime
    }

    /// Get the amount of time that the limiter over-slept between the last frame and this frame.
    pub fn get_last_frame_oversleep(&self) -> Duration {
        let Ok(oversleep_record)        = self.oversleep_record.try_lock() else { return Duration::default() };
        let Some(most_recent_oversleep) = oversleep_record.back()          else { return Duration::default() };
        *most_recent_oversleep
    }

    /// Get the average frame time over the recorded frame times.
    pub fn get_avg_frame_time(&self) -> Duration {
        let Ok(frametime_record) = self.frametime_record.try_lock() else { return Duration::default() };

        let mut total_duration = Duration::default();
        for record in &*frametime_record { total_duration = total_duration + *record; }
        if self.max_frame_records == 0 { return Duration::default() }
        total_duration / (self.max_frame_records as u32)
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
fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
    settings: Res<FramepaceSettingsProxy>,
) {
    #[cfg(target_arch = "wasm32")]
    return;

    // sleep the current thread
    let Ok(limit) = target_frametime.0.try_lock() else { return };
    let already_elapsed_time = timer.sleep_end.elapsed();
    let sleep_duration = stats.get_requested_sleep_duration(limit.deref().clone(), already_elapsed_time);
    if settings.is_enabled()
    { if sleep_duration != Duration::default() { spin_sleep::sleep(sleep_duration); } }

    // update stats and timer
    let final_frame_time = timer.sleep_end.elapsed();
    stats.add_frame_stats(final_frame_time, final_frame_time.saturating_sub(*limit));
    timer.sleep_end = Instant::now();
}
