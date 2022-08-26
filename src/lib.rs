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

use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
    render::{Extract, RenderApp, RenderStage},
    utils::Instant,
    winit::WinitWindows,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

/// Adds framepacing and framelimiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FramepaceSettings>()
            .init_resource::<FrametimeLimit>()
            .init_resource::<FramePaceStats>()
            .add_system_to_stage(CoreStage::Update, get_display_refresh_rate)
            .add_plugin(FramePaceDiagnosticsPlugin);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_resources)
            .add_system_to_stage(
                RenderStage::Cleanup,
                // We need this system to run at the end, immediately before the event loop restarts
                framerate_limiter.exclusive_system().at_end(),
            );
    }
}

/// Framepacing plugin configuration.
#[derive(Debug, Clone)]
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

#[derive(Debug, Default, Clone)]
struct FrametimeLimit(Duration);

#[derive(Debug)]
struct FrameTimer {
    render_end: Instant,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            render_end: Instant::now(),
        }
    }
}

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

fn detect_frametime(winit: NonSend<WinitWindows>, windows: Res<Windows>) -> Option<Duration> {
    let monitor = winit
        .get_window(windows.get_primary()?.id())?
        .current_monitor()?;
    // We need to subtract 0.5 here because winit only reports framerate to the nearest integer. To
    // prevent frames building up, adding latency, we need to use the most conservative possible
    // refresh rate that could round up to the integer value reported by winit.
    let best_framerate = bevy::winit::get_best_videomode(&monitor).refresh_rate() as f64 - 0.5;
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
#[derive(Clone, Debug)]
pub struct FramePaceStats {
    oversleep: Arc<Mutex<VecDeque<Duration>>>,
    frametime: Arc<Mutex<Duration>>,
    error: Arc<Mutex<f64>>,
}

impl Default for FramePaceStats {
    fn default() -> Self {
        Self {
            oversleep: Arc::new(Mutex::new(VecDeque::from([Duration::ZERO; 240]))),
            frametime: Default::default(),
            error: Default::default(),
        }
    }
}

fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepaceSettings>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
) {
    let target_frametime = target_frametime.0;
    let system_start = Instant::now();
    let this_render_time = system_start.duration_since(timer.render_end);

    let mut oversleep_lock = stats.oversleep.try_lock().unwrap();
    let oversleep_max = oversleep_lock.iter().max().copied().unwrap_or_default();

    let sleep_needed = target_frametime.saturating_sub(this_render_time);
    let sleep_needed_coarse = sleep_needed.saturating_sub(oversleep_max);

    let sleep_start = Instant::now();
    if settings.limiter.is_enabled() && sleep_needed_coarse > Duration::ZERO {
        std::thread::sleep(sleep_needed_coarse);
    }

    let this_oversleep = Instant::now()
        .duration_since(sleep_start)
        .saturating_sub(sleep_needed_coarse);

    if settings.limiter.is_enabled() {
        while Instant::now().duration_since(system_start) < sleep_needed {}
    }

    oversleep_lock.pop_back();
    oversleep_lock.push_front(this_oversleep);

    let frame_time = Instant::now().duration_since(timer.render_end);
    *stats.frametime.try_lock().unwrap() = frame_time;
    *stats.error.try_lock().unwrap() = frame_time.as_secs_f64() - target_frametime.as_secs_f64();

    timer.render_end = Instant::now();
}

/// Adds [`Diagnostics`] data from `bevy_framepace`
pub struct FramePaceDiagnosticsPlugin;

impl Plugin for FramePaceDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::diagnostic_system);
    }
}

impl FramePaceDiagnosticsPlugin {
    /// [`DiagnosticId`] for the frametime
    pub const FRAMEPACE_FRAMETIME: DiagnosticId =
        DiagnosticId::from_u128(8021378406439507683279787892187089153);
    /// [`DiagnosticId`] for oversleep
    pub const FRAMEPACE_OVERSLEEP: DiagnosticId =
        DiagnosticId::from_u128(7873478903246724896826890280382389054);
    /// [`DiagnosticId`] for failures to meet frame time target
    pub const FRAMEPACE_ERROR: DiagnosticId =
        DiagnosticId::from_u128(978023490268634078905367093342937);

    /// Initial setup for framepace diagnostics
    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(
            Diagnostic::new(Self::FRAMEPACE_FRAMETIME, "framepace::frametime", 20)
                .with_suffix("ms"),
        );
        diagnostics.add(
            Diagnostic::new(Self::FRAMEPACE_OVERSLEEP, "framepace::os_oversleep", 20)
                .with_suffix("µs"),
        );
        diagnostics
            .add(Diagnostic::new(Self::FRAMEPACE_ERROR, "framepace::error", 20).with_suffix("ns"));
    }

    /// Updates diagnostic data from measurements
    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        time: Res<Time>,
        stats: Res<FramePaceStats>,
    ) {
        if time.delta_seconds_f64() == 0.0 {
            return;
        }

        let frametime_millis = stats.frametime.try_lock().unwrap().as_secs_f64() * 1000.0;
        let oversleep_lock = stats.oversleep.try_lock().unwrap();
        let oversleep_micros = oversleep_lock
            .front()
            .map(|v| v.as_secs_f64())
            .unwrap_or(0.0)
            * 1000000.0;
        let error_nanos = *stats.error.try_lock().unwrap() * 1000000000.0;

        diagnostics.add_measurement(Self::FRAMEPACE_FRAMETIME, || frametime_millis);
        diagnostics.add_measurement(Self::FRAMEPACE_OVERSLEEP, || oversleep_micros);
        diagnostics.add_measurement(Self::FRAMEPACE_ERROR, || error_nanos);
    }
}
