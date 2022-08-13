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
    prelude::*,
    render::{Extract, RenderApp, RenderStage},
    winit::WinitWindows,
};
use std::time::{Duration, Instant};

/// Adds framepacing and framelimiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FramepaceSettings>()
            .init_resource::<FrametimeLimit>()
            .add_system_to_stage(CoreStage::Update, get_display_refresh_rate);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_display_refresh_rate)
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
    /// When enabled, the plugin logs a warning every time the app's frametime exceeds the target
    /// frametime by 100µs.
    pub warn_on_frame_drop: bool,
}
impl FramepaceSettings {
    /// Builds plugin settings with warnings set to `warnings_enabled`.
    pub fn with_warnings(mut self, warnings_enabled: bool) -> Self {
        self.warn_on_frame_drop = warnings_enabled;
        self
    }

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
            warn_on_frame_drop: true,
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
        info!("Frametime limit changed to: {:.2?}", new_frametime);
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

fn extract_display_refresh_rate(
    mut commands: Commands,
    settings: Extract<Res<FramepaceSettings>>,
    framerate_limit: Extract<Res<FrametimeLimit>>,
) {
    commands.insert_resource(settings.to_owned());
    commands.insert_resource(framerate_limit.to_owned());
}

fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepaceSettings>,
    target_frametime: Res<FrametimeLimit>,
) {
    let target_frametime = target_frametime.0;
    let this_render_time = Instant::now().duration_since(timer.render_end);
    let sleep_needed = target_frametime.saturating_sub(this_render_time);

    if settings.limiter.is_enabled() {
        spin_sleep::sleep(sleep_needed);
    }

    frametime_alert(
        Instant::now().duration_since(timer.render_end),
        target_frametime,
        &settings,
    );

    timer.render_end = Instant::now();
}

fn frametime_alert(
    this_frametime: Duration,
    target_frametime: Duration,
    settings: &Res<FramepaceSettings>,
) {
    if this_frametime.saturating_sub(target_frametime) > Duration::from_micros(100)
        && settings.warn_on_frame_drop
        && settings.limiter.is_enabled()
    {
        warn!(
            "[Frame Drop] {:.2?} (+{:.2?})",
            this_frametime,
            this_frametime.saturating_sub(target_frametime),
        );
    }
}
