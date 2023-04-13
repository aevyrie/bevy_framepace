//! Adds diagnostic logging and a cursor for debugging.

use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic},
    prelude::*,
};

/// Adds [`Diagnostics`] data from `bevy_framepace`
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::diagnostic_system);

        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_FRAME_PRESENT_TIME, "framepace::frame_present_time", 128)
                .with_suffix("ms"),
        );
        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_APP_FRAMETIME, "framepace::app_frametime", 128)
                .with_suffix("ms"),
        );
        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_FRAMETIME, "framepace::frametime", 128)
                .with_suffix("ms"),
        );
        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_TARGET_DELTA, "framepace::target_delta", 128)
                .with_suffix("µs"),
        );
    }
}

impl DiagnosticsPlugin {
    /// [`DiagnosticId`] for the frame present time
    pub const FRAMEPACE_FRAME_PRESENT_TIME: DiagnosticId =
        DiagnosticId::from_u128(123575298475829475892479520402093848);
    /// [`DiagnosticId`] for the app frametime
    pub const FRAMEPACE_APP_FRAMETIME: DiagnosticId =
        DiagnosticId::from_u128(9760976976096896098696969690690689696);
    /// [`DiagnosticId`] for the frametime
    pub const FRAMEPACE_FRAMETIME: DiagnosticId =
        DiagnosticId::from_u128(8021378406439507683279787892187089153);
    /// [`DiagnosticId`] for failures to meet frame time target
    pub const FRAMEPACE_TARGET_DELTA: DiagnosticId =
        DiagnosticId::from_u128(978023490268634078905367093342937);

    /// Updates diagnostic data from measurements
    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time>,
        stats: Res<crate::FramePaceStats>,
    ) {
        if time.delta_seconds_f64() == 0.0 {
            return;
        }

        let frame_present_time_millis = stats.get_last_frame_present_time().as_secs_f64() * 1_000_f64;
        let app_frametime_millis = stats.get_last_app_frametime().as_secs_f64() * 1_000_f64;
        let frametime_millis = stats.get_last_frame_time().as_secs_f64() * 1_000_f64;
        let error_micros = stats.get_last_target_delta() * 1_000_000_f64;

        diagnostics.add_measurement(Self::FRAMEPACE_FRAME_PRESENT_TIME, || frame_present_time_millis);
        diagnostics.add_measurement(Self::FRAMEPACE_APP_FRAMETIME, || app_frametime_millis);
        diagnostics.add_measurement(Self::FRAMEPACE_FRAMETIME, || frametime_millis);
        diagnostics.add_measurement(Self::FRAMEPACE_TARGET_DELTA, || error_micros);
    }
}
