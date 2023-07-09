//! Adds diagnostic logging and a cursor for debugging.

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic},
    prelude::*,
};

/// Adds [`Diagnostics`] data from `bevy_framepace`
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::diagnostic_system);

        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_FRAMETIME, "framepace::frametime", 128)
                .with_suffix("ms"),
        );
        app.register_diagnostic(
            Diagnostic::new(Self::FRAMEPACE_OVERSLEEP, "framepace::oversleep", 128)
                .with_suffix("µs"),
        );
    }
}

impl DiagnosticsPlugin {
    /// [`DiagnosticId`] for the frametime
    pub const FRAMEPACE_FRAMETIME: DiagnosticId =
        DiagnosticId::from_u128(8021378406439507683279787892187089153);
    /// [`DiagnosticId`] for failures to meet frame time target
    pub const FRAMEPACE_OVERSLEEP: DiagnosticId =
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

        let frametime_millis = stats.frametime.try_lock().unwrap().as_secs_f64() * 1_000_f64;
        let error_micros = stats.oversleep.try_lock().unwrap().as_secs_f64() * 1_000_000_f64;

        diagnostics.add_measurement(Self::FRAMEPACE_FRAMETIME, || frametime_millis);
        diagnostics.add_measurement(Self::FRAMEPACE_OVERSLEEP, || error_micros);
    }
}

/// Marks the entity to use for the framepace debug cursor.
#[derive(Component, Debug, Reflect)]
pub struct DebugCursor;

/// Marks the camera to use for rendering the framepace debug cursor.
#[derive(Component, Debug, Reflect)]
pub struct DebugCursorCamera;

/// Adds a simple debug cursor for quickly testing latency.
pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup_cursor)
            .add_systems(Update, Self::update_cursor);
    }
}

impl CursorPlugin {
    /// Spawns the [`DebugCursorCamera`] and [`DebugCursor`] entities.
    pub fn setup_cursor(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        commands.spawn((
            Camera2dBundle {
                camera: Camera {
                    order: 100,
                    ..Default::default()
                },
                camera_2d: Camera2d {
                    clear_color: ClearColorConfig::None,
                },
                ..Default::default()
            },
            DebugCursorCamera,
        ));
        commands.spawn((
            bevy::sprite::MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(10.0).into()).into(),
                material: materials.add(ColorMaterial::from(Color::GREEN)),
                transform: Transform::from_translation(Vec3::new(-100., 0., 0.)),
                ..default()
            },
            DebugCursor,
        ));
    }

    /// Updates the position of the [`DebugCursor`].
    pub fn update_cursor(
        windows: Query<&Window>,
        mut cursor: Query<&mut Transform, With<DebugCursor>>,
    ) {
        if let Some(pos) = windows.single().cursor_position() {
            let offset = -Vec2::new(windows.single().width(), windows.single().height()) / 2.0;
            cursor.single_mut().translation = (pos + offset).extend(0.0);
        }
    }
}
