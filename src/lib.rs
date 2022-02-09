use bevy::{
    prelude::*,
    render::{RenderApp, RenderStage, RenderWorld},
    winit::WinitWindows,
};
use ringbuffer::{ConstGenericRingBuffer, RingBufferExt, RingBufferWrite};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin {
    pub enabled: bool,
    pub framerate_limit: FramerateLimit,
    pub warn_on_frame_drop: bool,
    /// How early should we cut the sleep time by, to make sure we have enough time to render our
    /// frame if it takes longer than expected? Increasing this number makes dropped frames less
    /// likely, but increases motion-to-photon latency of user input rendered to screen. Use
    /// `FramepacePlugin::default()` as a starting point.
    pub safety_margin: Duration,
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
            warn_on_frame_drop: true,
            safety_margin: Duration::from_micros(200),
        }
    }
}
impl FramepacePlugin {
    pub fn framerate(fps: u64) -> Self {
        Self {
            framerate_limit: FramerateLimit::Manual(fps),
            ..Default::default()
        }
    }
    pub fn without_warnings(mut self) -> Self {
        self.warn_on_frame_drop = false;
        self
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

const FRAMETIME_SAMPLES: usize = 32;

#[derive(Debug)]
struct FrameTimer {
    frame_time_history: ConstGenericRingBuffer<Duration, FRAMETIME_SAMPLES>,
    post_render_start: Instant,
    render_start: Instant,
    exact_sleep: Duration,
}
impl Default for FrameTimer {
    fn default() -> Self {
        let mut frame_time_history = ConstGenericRingBuffer::default();
        frame_time_history.fill(Duration::from_millis(1));
        FrameTimer {
            frame_time_history,
            post_render_start: Instant::now(),
            render_start: Instant::now(),
            exact_sleep: Duration::ZERO,
        }
    }
}

fn measure_refresh_rate(
    settings: Res<FramepacePlugin>,
    winit: Res<WinitWindows>,
    windows: Res<Windows>,
    mut meas_limit: ResMut<MeasuredFramerateLimit>,
) {
    if !settings.is_changed() && !winit.is_changed() {
        return;
    }
    let update = match settings.framerate_limit {
        FramerateLimit::Auto => {
            let modes = winit
                .get_window(windows.get_primary().unwrap().id())
                .unwrap()
                .current_monitor()
                .unwrap()
                .video_modes();
            let best = modes.map(|f| f.refresh_rate() as u64).max();
            if let Some(framerate) = best {
                if framerate != meas_limit.0 {
                    Some(framerate)
                } else {
                    None
                }
            } else {
                None
            }
        }
        FramerateLimit::Manual(framerate) => {
            if framerate != meas_limit.0 {
                Some(framerate)
            } else {
                None
            }
        }
    };
    if let Some(fps) = update {
        *meas_limit = MeasuredFramerateLimit(fps);
        info!("Detected refresh rate is: {} fps", fps);
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
    timer.frame_time_history.push(last_frametime);
    // let avg_frametime =
    //     timer.frame_time_history.iter().sum::<Duration>() / FRAMETIME_SAMPLES as u32;
    let max_frametime = timer
        .frame_time_history
        .iter()
        .max()
        .cloned()
        .unwrap_or_else(|| Duration::from_millis(100));
    let last_render_time = max_frametime - max_frametime.min(timer.exact_sleep);
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
    if this_frametime > target_frametime && settings.warn_on_frame_drop {
        warn!(
            "Frame dropped. Frametime: {:.2?} (+{})",
            this_frametime,
            format!(
                "{:.2}ms",
                (this_frametime - target_frametime).as_micros() as f32 / 1000.0
            ),
        );
    }
    //let sleep_needed = target_frametime - target_frametime.min(this_frametime);
    //let sleep_needed = sleep_needed - sleep_needed.min(settings.safety_margin);
    if settings.enabled {
        //spin_sleep::sleep(sleep_needed);
    }
    timer.render_start = Instant::now();
    timer.exact_sleep = timer.render_start.duration_since(system_start);
}
