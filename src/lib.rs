use bevy::{
    prelude::*,
    render::{Extract, RenderApp, RenderStage},
    winit::WinitWindows,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin {
    pub framerate_limit: FramerateLimitParam,
    pub warn_on_frame_drop: bool,
    pub sleep_safety_margin: Duration,
}
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.clone())
            .init_resource::<FramerateLimit>()
            .add_system_to_stage(CoreStage::Update, get_display_refresh_rate);
        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .add_system_to_stage(RenderStage::Extract, extract_display_refresh_rate)
            .add_system_to_stage(RenderStage::Cleanup, framerate_limiter);
    }
}
impl Default for FramepacePlugin {
    fn default() -> Self {
        Self {
            framerate_limit: FramerateLimitParam::Auto,
            warn_on_frame_drop: true,
            sleep_safety_margin: Duration::from_micros(0),
        }
    }
}
impl FramepacePlugin {
    pub fn framerate(fps: u16) -> Self {
        Self {
            framerate_limit: FramerateLimitParam::Manual(fps),
            ..Default::default()
        }
    }
    pub fn without_warnings(mut self) -> Self {
        self.warn_on_frame_drop = false;
        self
    }
    pub fn disable(&mut self) {
        self.framerate_limit = FramerateLimitParam::Off;
    }
    pub fn enable_auto(&mut self) {
        self.framerate_limit = FramerateLimitParam::Auto;
    }
    pub fn enable_manual(&mut self, framerate: u16) {
        self.framerate_limit = FramerateLimitParam::Manual(framerate);
    }
}

#[derive(Debug, Clone)]
pub enum FramerateLimitParam {
    /// Uses the window's refresh rate to set the framerate limit
    Auto,
    /// Set a manual framerate limit. Note this should be <= to the window's refresh rate.
    Manual(u16),
    Off,
}
impl FramerateLimitParam {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, FramerateLimitParam::Off)
    }
}

#[derive(Debug, Default, Clone, Component)]
pub struct FramerateLimit {
    fps: u16,
}

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
    settings: Res<FramepacePlugin>,
    winit: NonSend<WinitWindows>,
    windows: Res<Windows>,
    mut frame_limit: ResMut<FramerateLimit>,
) {
    if !settings.is_changed() && !winit.is_changed() {
        return;
    }
    let pre = "[Update]";
    let update = match settings.framerate_limit {
        FramerateLimitParam::Auto => {
            let monitor = winit
                .get_window(windows.get_primary().unwrap().id())
                .unwrap()
                .current_monitor()
                .unwrap();
            let best_framerate = bevy::winit::get_best_videomode(&monitor).refresh_rate();
            if best_framerate != frame_limit.fps {
                info!("{pre} Detected refresh rate is: {} fps", best_framerate);
                Some(best_framerate)
            } else {
                None
            }
        }
        FramerateLimitParam::Manual(framerate) => {
            if framerate != frame_limit.fps {
                info!("{pre} Manual refresh rate is: {} fps", framerate);
                Some(framerate)
            } else {
                None
            }
        }
        FramerateLimitParam::Off => {
            info!("{pre} Frame limit disabled");
            None
        }
    };
    if let Some(fps) = update {
        *frame_limit = FramerateLimit { fps };
    }
}

fn extract_display_refresh_rate(
    mut commands: Commands,
    settings: Extract<Res<FramepacePlugin>>,
    framerate_limit: Extract<Res<FramerateLimit>>,
) {
    commands.insert_resource(settings.to_owned());
    commands.insert_resource(framerate_limit.to_owned());
}

fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    settings: Res<FramepacePlugin>,
    refresh_rate: Res<FramerateLimit>,
) {
    let framerate_limit = refresh_rate.fps;
    let system_start = Instant::now();
    let target_frametime = Duration::from_secs_f64(1.0 / framerate_limit as f64);
    let this_frametime = system_start.duration_since(timer.render_end);
    let sleep_needed = target_frametime.saturating_sub(this_frametime);
    let sleep_actual = sleep_needed.saturating_sub(settings.sleep_safety_margin);
    if settings.framerate_limit.is_enabled() {
        spin_sleep::sleep(sleep_actual);
    }
    let updated_frametime = Instant::now().duration_since(timer.render_end);
    frametime_alert(updated_frametime, target_frametime, &settings);
    timer.render_end = Instant::now();
}

fn frametime_alert(
    this_frametime: Duration,
    target_frametime: Duration,
    settings: &Res<FramepacePlugin>,
) {
    if this_frametime > target_frametime + Duration::from_micros(10)
        && settings.warn_on_frame_drop
        && settings.framerate_limit.is_enabled()
    {
        warn!(
            "[Frame Drop] {:.2?} (+{})",
            this_frametime,
            format!(
                "{:.2}μs",
                (this_frametime - target_frametime).as_nanos() as f64 / 1000.,
            ),
        );
    }
}
