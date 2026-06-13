//! The Neural Business Engine — P0 scaffold.
//!
//! Boots a native Bevy window (DirectX 12 on the Windows workstation, the primary
//! `wgpu` backend elsewhere), shows a frame-time HUD, and renders placeholder geometry
//! with an orbit/zoom camera. This is the first runnable gate of the build plan:
//! every later phase (data engine, instanced graph render, activation glow, action
//! potentials) hangs off this shell.
//!
//! The chosen GPU adapter + backend is printed by `bevy_render` at startup, e.g.
//! `AdapterInfo { name: "NVIDIA GeForce RTX 5080", backend: Dx12, .. }` — that log line
//! is the acceptance check for "running on DX12 against the RTX 5080".

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::WindowResolution;

fn main() {
    // Force DX12 on the Windows workstation; let wgpu pick the primary backend elsewhere
    // (Vulkan on this Linux build host) so the project stays buildable cross-platform.
    let backends = if cfg!(target_os = "windows") {
        Some(Backends::DX12)
    } else {
        None
    };

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "The Neural Business Engine".into(),
                        resolution: WindowResolution::new(1600, 900),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends,
                        power_preference: PowerPreference::HighPerformance,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::srgb(0.015, 0.02, 0.04)))
        .add_systems(Startup, (setup_scene, setup_hud))
        .add_systems(Update, (orbit_camera, update_hud))
        .run();
}

/// Simple yaw/pitch/zoom orbit camera. Reused by the graph view in P2.
#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 25.0,
            yaw: 0.6,
            pitch: 0.5,
        }
    }
}

impl OrbitCamera {
    /// World-space eye position derived from the spherical orbit parameters.
    fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.focus + Vec3::new(self.radius * cp * sy, self.radius * sp, self.radius * cp * cy)
    }
}

#[derive(Component)]
struct HudText;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    let cam = OrbitCamera::default();
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));

    // Key light
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Placeholder grid of cubes — stand-in for the future GPU-instanced neuron field.
    let mesh = meshes.add(Cuboid::new(0.6, 0.6, 0.6));
    let n: i32 = 6;
    for x in 0..n {
        for z in 0..n {
            let h = (x + z) as f32 / (2.0 * n as f32);
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2 + h, 0.5, 1.0 - h),
                    emissive: LinearRgba::rgb(0.06, 0.18, 0.4),
                    ..default()
                })),
                Transform::from_xyz(
                    (x as f32 - n as f32 / 2.0) * 1.5,
                    0.0,
                    (z as f32 - n as f32 / 2.0) * 1.5,
                ),
            ));
        }
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("booting…"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudText,
    ));
}

fn update_hud(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<HudText>>) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    for mut text in &mut query {
        text.0 = format!("{fps:5.0} FPS   |   {frame_ms:5.2} ms/frame");
    }
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    // Drag with the left mouse button to rotate.
    let mut drag = Vec2::ZERO;
    if mouse_buttons.pressed(MouseButton::Left) {
        for ev in motion.read() {
            drag += ev.delta;
        }
    } else {
        motion.clear();
    }

    // Scroll to zoom.
    let mut scroll = 0.0;
    for ev in wheel.read() {
        scroll += ev.y;
    }

    for (mut orbit, mut transform) in &mut query {
        orbit.yaw -= drag.x * 0.005;
        orbit.pitch = (orbit.pitch + drag.y * 0.005).clamp(-1.4, 1.4);
        orbit.radius = (orbit.radius - scroll * 1.5).clamp(3.0, 120.0);
        *transform = Transform::from_translation(orbit.eye()).looking_at(orbit.focus, Vec3::Y);
    }
}
