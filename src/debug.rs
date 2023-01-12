use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
};

/// Adds [`Diagnostics`] data from `bevy_framepace`
pub struct FramePaceDiagnosticsPlugin;

impl Plugin for FramePaceDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::setup_system)
            .add_startup_system(Self::setup_cursor)
            .add_system(Self::update_cursor)
            .add_system(Self::diagnostic_system);
    }
}

impl FramePaceDiagnosticsPlugin {
    /// [`DiagnosticId`] for the frametime
    pub const FRAMEPACE_FRAMETIME: DiagnosticId =
        DiagnosticId::from_u128(8021378406439507683279787892187089153);
    /// [`DiagnosticId`] for failures to meet frame time target
    pub const FRAMEPACE_OVERSLEEP: DiagnosticId =
        DiagnosticId::from_u128(978023490268634078905367093342937);

    /// Initial setup for framepace diagnostics
    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(
            Diagnostic::new(Self::FRAMEPACE_FRAMETIME, "framepace::frametime", 128)
                .with_suffix("ms"),
        );
        diagnostics.add(
            Diagnostic::new(Self::FRAMEPACE_OVERSLEEP, "framepace::oversleep", 128)
                .with_suffix("µs"),
        );
    }

    /// Updates diagnostic data from measurements
    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
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

    ///
    pub fn setup_cursor(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        commands.spawn(Camera2dBundle {
            camera: Camera {
                priority: 100,
                ..Default::default()
            },
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None,
            },
            ..Default::default()
        });
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

    ///
    pub fn update_cursor(
        windows: Res<Windows>,
        mut cursor: Query<&mut Transform, With<DebugCursor>>,
    ) {
        if let Some(pos) = windows.primary().cursor_position() {
            let offset = -Vec2::new(windows.primary().width(), windows.primary().height()) / 2.0;
            cursor.single_mut().translation = (pos + offset).extend(0.0);
        }
    }
}

///
#[derive(Component)]
pub struct DebugCursor;
