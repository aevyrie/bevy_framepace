use bevy::{
    prelude::*,
    render::{RenderApp, RenderStage},
};
use std::time::{Duration, Instant};

pub struct FramepacePlugin {
    enabled: bool,
    framerate_limit: u64,
    safety_margin: Duration,
}
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .insert_resource(FramepaceSettings {
                enabled: self.enabled,
                framerate_limit: self.framerate_limit,
                safety_margin: self.safety_margin,
            })
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Render, framerate_exact_limiter)
            .add_system_to_stage(RenderStage::Cleanup, framerate_limit_forward_estimator);
    }
}
impl Default for FramepacePlugin {
    fn default() -> Self {
        Self {
            enabled: true,
            framerate_limit: 60,
            safety_margin: Duration::from_micros(500),
        }
    }
}

#[derive(Debug)]
pub struct FramepaceSettings {
    enabled: bool,
    framerate_limit: u64,
    /// How early should we cut the sleep time by, to make sure we have enough time to render our
    /// frame if it takes longer than expected? Increasing this number makes dropped frames less
    /// likely, but increases motion-to-photon latency of user input rendered to screen. Use
    /// `FramepaceSettings::default()` as a starting point.
    safety_margin: Duration,
}

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

/// How long we *think* we should sleep before starting to render the next frame
fn framerate_limit_forward_estimator(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepaceSettings>,
) {
    let render_end = Instant::now();
    let target_frametime = Duration::from_micros(1_000_000 / settings.framerate_limit);
    let last_frametime = render_end.duration_since(timer.post_render_start);
    let last_render_time = last_frametime - timer.exact_sleep;
    let estimated_cpu_time_needed = last_render_time + settings.safety_margin;
    let estimated_sleep_time = target_frametime - target_frametime.min(estimated_cpu_time_needed);
    if settings.enabled {
        spin_sleep::sleep(estimated_sleep_time);
    }
    timer.post_render_start = Instant::now();
}

fn framerate_exact_limiter(mut timer: ResMut<FrameTimer>, settings: Res<FramepaceSettings>) {
    let system_start = Instant::now();
    let target_frametime = Duration::from_micros(1_000_000 / settings.framerate_limit);
    let sleep_needed =
        target_frametime - target_frametime.min(system_start.duration_since(timer.render_start));
    if settings.enabled {
        spin_sleep::sleep(sleep_needed);
    }
    timer.render_start = Instant::now();
    timer.exact_sleep = timer.render_start.duration_since(system_start);
}
