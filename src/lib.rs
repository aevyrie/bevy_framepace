use bevy::{
    prelude::*,
    render::{RenderApp, RenderStage, RenderWorld},
    winit::WinitWindows,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin {
    enabled: bool,
    framerate_limit: FramerateLimit,
    /// How early should we cut the sleep time by, to make sure we have enough time to render our
    /// frame if it takes longer than expected? Increasing this number makes dropped frames less
    /// likely, but increases motion-to-photon latency of user input rendered to screen. Use
    /// `FramepacePlugin::default()` as a starting point.
    safety_margin: Duration,
}
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.clone())
            .insert_resource(MeasuredFramerateLimit(0))
            .add_system_to_stage(CoreStage::Update, measure_refresh_rate);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_refresh_rate)
            .add_system_to_stage(RenderStage::Render, framerate_exact_limiter)
            .add_system_to_stage(RenderStage::Cleanup, framerate_limit_forward_estimator);
    }
}
impl Default for FramepacePlugin {
    fn default() -> Self {
        Self {
            enabled: true,
            framerate_limit: FramerateLimit::Auto,
            safety_margin: Duration::from_micros(500),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FramerateLimit {
    /// Uses the window's refresh rate to set the framerate limit
    Auto,
    /// Set a manual framerate limit. Note this should be <= to the window's refresh rate.
    Manual(u64),
}
#[derive(Debug, Clone, Component)]
pub struct MeasuredFramerateLimit(u64);

#[derive(Debug)]
struct FrameTimer {
    post_render_start: Instant,
    render_start: Instant,
    exact_sleep: Duration,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            post_render_start: Instant::now(),
            render_start: Instant::now(),
            exact_sleep: Duration::from_millis(0),
        }
    }
}

fn measure_refresh_rate(
    settings: Res<FramepacePlugin>,
    winit: Res<WinitWindows>,
    windows: Res<Windows>,
    mut meas_limit: ResMut<MeasuredFramerateLimit>,
) {
    match settings.framerate_limit {
        FramerateLimit::Auto => {
            let measurement = winit
                .get_window(windows.get_primary().unwrap().id())
                .unwrap()
                .current_monitor()
                .unwrap()
                .video_modes()
                .last()
                .unwrap()
                .refresh_rate() as u64;
            if measurement != meas_limit.0 {
                info!("Detected refresh rate is: {} fps", measurement);
                *meas_limit = MeasuredFramerateLimit(measurement);
            }
        }
        FramerateLimit::Manual(fps) => {
            if fps != meas_limit.0 {
                info!("Detected refresh rate is: {} fps", fps);
                *meas_limit = MeasuredFramerateLimit(fps);
            }
        }
    }
}

fn extract_refresh_rate(
    settings: Res<FramepacePlugin>,
    framerate_limit: Res<MeasuredFramerateLimit>,
    mut r_world: ResMut<RenderWorld>,
) {
    r_world.insert_resource(framerate_limit.clone());
    r_world.insert_resource(settings.clone());
}

/// How long we *think* we should sleep before starting to render the next frame
fn framerate_limit_forward_estimator(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepacePlugin>,
    refresh_rate: Res<MeasuredFramerateLimit>,
) {
    let framerate_limit = refresh_rate.0;
    let render_end = Instant::now();
    let target_frametime = Duration::from_micros(1_000_000 / framerate_limit);
    let last_frametime = render_end.duration_since(timer.post_render_start);
    let last_render_time = last_frametime - timer.exact_sleep;
    let estimated_cpu_time_needed = last_render_time + settings.safety_margin;
    let estimated_sleep_time = target_frametime - target_frametime.min(estimated_cpu_time_needed);
    if settings.enabled {
        spin_sleep::sleep(estimated_sleep_time);
    }
    timer.post_render_start = Instant::now();
}

fn framerate_exact_limiter(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepacePlugin>,
    refresh_rate: Res<MeasuredFramerateLimit>,
) {
    let framerate_limit = refresh_rate.0;
    let system_start = Instant::now();
    let target_frametime = Duration::from_micros(1_000_000 / framerate_limit);
    let this_frametime = system_start.duration_since(timer.render_start);
    let sleep_needed = target_frametime - target_frametime.min(this_frametime);
    let sleep_needed_safe =
        sleep_needed.max(Duration::from_micros(200)) - Duration::from_micros(200);
    if settings.enabled {
        spin_sleep::sleep(sleep_needed_safe);
    }
    if sleep_needed.is_zero() {
        warn!("Frame dropped. Frametime: {:?}", this_frametime);
    }
    timer.render_start = Instant::now();
    timer.exact_sleep = timer.render_start.duration_since(system_start);
}
