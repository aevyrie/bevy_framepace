use bevy::{
    prelude::*,
    render::{RenderApp, RenderStage, RenderWorld},
    winit::WinitWindows,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin {
    pub framerate_limit: FramerateLimit,
    pub warn_on_frame_drop: bool,
}
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.clone())
            .insert_resource(MeasuredFramerateLimit(0))
            .add_system_to_stage(CoreStage::Update, measure_refresh_rate);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_refresh_rate)
            .add_system_to_stage(RenderStage::Cleanup, framerate_exact_limiter);
    }
}
impl Default for FramepacePlugin {
    fn default() -> Self {
        Self {
            framerate_limit: FramerateLimit::Auto,
            warn_on_frame_drop: true,
        }
    }
}
impl FramepacePlugin {
    pub fn framerate(fps: u16) -> Self {
        Self {
            framerate_limit: FramerateLimit::Manual(fps),
            ..Default::default()
        }
    }
    pub fn without_warnings(mut self) -> Self {
        self.warn_on_frame_drop = false;
        self
    }
    pub fn disable(&mut self) {
        self.framerate_limit = FramerateLimit::Off;
    }
    pub fn enable_auto(&mut self) {
        self.framerate_limit = FramerateLimit::Auto;
    }
    pub fn enable_manual(&mut self, framerate: u16) {
        self.framerate_limit = FramerateLimit::Manual(framerate);
    }
}

#[derive(Debug, Clone)]
pub enum FramerateLimit {
    /// Uses the window's refresh rate to set the framerate limit
    Auto,
    /// Set a manual framerate limit. Note this should be <= to the window's refresh rate.
    Manual(u16),
    Off,
}
impl FramerateLimit {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, FramerateLimit::Off)
    }
}

#[derive(Debug, Clone, Component)]
pub struct MeasuredFramerateLimit(u16);

#[derive(Debug)]
struct FrameTimer {
    render_start: Instant,
    exact_sleep: Duration,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            render_start: Instant::now(),
            exact_sleep: Duration::ZERO,
        }
    }
}

fn measure_refresh_rate(
    settings: Res<FramepacePlugin>,
    winit: NonSend<WinitWindows>,
    windows: Res<Windows>,
    mut meas_limit: ResMut<MeasuredFramerateLimit>,
) {
    if !settings.is_changed() && !winit.is_changed() {
        return;
    }
    let update = match settings.framerate_limit {
        FramerateLimit::Auto => {
            let monitor = winit
                .get_window(windows.get_primary().unwrap().id())
                .unwrap()
                .current_monitor()
                .unwrap();
            let best_framerate = bevy::winit::get_best_videomode(&monitor).refresh_rate();
            if best_framerate != meas_limit.0 {
                info!("Detected refresh rate is: {} fps", best_framerate);
                Some(best_framerate)
            } else {
                None
            }
        }
        FramerateLimit::Manual(framerate) => {
            if framerate != meas_limit.0 {
                info!("Manual refresh rate is: {} fps", framerate);
                Some(framerate)
            } else {
                None
            }
        }
        FramerateLimit::Off => {
            info!("Disabled");
            None
        }
    };
    if let Some(fps) = update {
        *meas_limit = MeasuredFramerateLimit(fps);
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

fn framerate_exact_limiter(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepacePlugin>,
    refresh_rate: Res<MeasuredFramerateLimit>,
) {
    let framerate_limit = refresh_rate.0;
    let system_start = Instant::now();
    let target_frametime = Duration::from_micros(1_000_000 / framerate_limit as u64);
    let this_frametime = system_start.duration_since(timer.render_start);
    if this_frametime > target_frametime
        && settings.warn_on_frame_drop
        && settings.framerate_limit.is_enabled()
    {
        warn!(
            "Frame dropped! Frametime: {:.2?} (+{})",
            this_frametime,
            format!(
                "{:.2}ms",
                (this_frametime - target_frametime).as_micros() as f32 / 1000.,
            ),
        );
    }
    let sleep_needed = target_frametime.saturating_sub(this_frametime);
    if settings.framerate_limit.is_enabled() {
        spin_sleep::sleep(sleep_needed);
    }
    timer.render_start = Instant::now();
    timer.exact_sleep = timer.render_start.duration_since(system_start);
}
