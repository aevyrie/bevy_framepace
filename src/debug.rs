//! Adds diagnostic logging and a cursor for debugging.

use bevy_app::prelude::*;
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::prelude::*;
use bevy_time::prelude::*;

/// Adds [`Diagnostics`] data from `bevy_framepace`
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::diagnostic_system);

        app.register_diagnostic(Diagnostic::new(Self::FRAMEPACE_FRAMETIME).with_suffix("ms"));
        app.register_diagnostic(Diagnostic::new(Self::FRAMEPACE_OVERSLEEP).with_suffix("µs"));
    }
}

impl DiagnosticsPlugin {
    /// [`DiagnosticPath`] for the frametime
    pub const FRAMEPACE_FRAMETIME: DiagnosticPath =
        DiagnosticPath::const_new("framepace/frametime");
    /// [`DiagnosticPath`] for failures to meet frame time target
    pub const FRAMEPACE_OVERSLEEP: DiagnosticPath =
        DiagnosticPath::const_new("framepace/oversleep");

    /// Updates diagnostic data from measurements
    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time>,
        stats: Res<crate::FramePaceStats>,
    ) {
        if time.delta_seconds_f64() == 0.0 {
            return;
        }

        let frametime_millis = stats.frametime.try_lock().unwrap().as_secs_f64() * 1_000_f64;
        let error_micros = stats.oversleep.try_lock().unwrap().as_secs_f64() * 1_000_000_f64;

        diagnostics.add_measurement(&Self::FRAMEPACE_FRAMETIME, || frametime_millis);
        diagnostics.add_measurement(&Self::FRAMEPACE_OVERSLEEP, || error_micros);
    }
}
