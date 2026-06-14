//! The Neural Business Engine — renderer.
//!
//! Loads the graph from the SQLite database (`--db <path>`, default `brain.db`), computes the
//! layered ANN layout, and renders it as the hybrid neuron/mycelial visual: activation-driven
//! glowing nodes (white-hot when hot) and organic curved synapse edges, with HDR bloom. The
//! GPU adapter/backend is logged by `bevy_render` at startup (DX12 on the workstation).

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::PrimitiveTopology;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::view::Hdr;
use bevy::render::RenderPlugin;
use bevy::window::WindowResolution;

use nbe_data::Layer;
use nbe_geometry::{edge_curve, CurveParams};
use nbe_layout::{layered_layout, LayoutNode, LayoutParams};

#[derive(Resource)]
struct DbPath(String);

fn db_path_from_args() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--db" {
            if let Some(v) = args.next() {
                return v;
            }
        }
    }
    "brain.db".to_string()
}

fn main() {
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
        .insert_resource(ClearColor(Color::srgb(0.008, 0.015, 0.04)))
        .insert_resource(DbPath(db_path_from_args()))
        .add_systems(Startup, (load_graph, setup_hud))
        .add_systems(Update, (orbit_camera, update_hud))
        .run();
}

/// Yaw/pitch/zoom orbit camera.
#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

impl OrbitCamera {
    fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.focus + Vec3::new(self.radius * cp * sy, self.radius * sp, self.radius * cp * cy)
    }
}

#[derive(Component)]
struct HudText;

// ---- deterministic per-id depth (gives the flat layered graph a bit of 3D / bokeh) ----

fn hash_u64(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// Spread/scatter of the neuron cloud. Layers advance left->right by LAYER_GAP, but each layer's
// nodes are scattered through a 3D volume (radius SPREAD) so it reads as an organic web rather
// than rigid vertical columns.
const LAYER_GAP: f32 = 30.0;
const SPREAD: f32 = 38.0;
const X_JITTER: f32 = 8.0;

// Connection culling — fewer, cleaner, node-to-node edges (matches the reference web).
const EDGE_WEIGHT_MIN: f64 = 0.55;
const MAX_EDGE_LEN: f32 = 52.0;

/// Deterministic value in [-1, 1] from an id + salt.
fn rand_unit(id: &str, salt: u64) -> f32 {
    let mut h = hash_u64(id) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 32;
    let u = (h >> 11) as f32 / (1u64 << 53) as f32; // [0,1)
    u * 2.0 - 1.0
}

/// Emissive colour (HDR, can exceed 1.0 for bloom): hue by layer, white-hot by activation.
fn node_emissive(activation: f32, column: u32, last: u32) -> LinearRgba {
    let base = if column == 0 {
        (0.10, 1.0, 0.55) // input — green-cyan
    } else if column == last {
        (1.0, 0.55, 0.12) // output — amber
    } else {
        (0.14, 0.6, 1.0) // hidden — electric blue
    };
    let t = activation.clamp(0.0, 1.0) * 0.85;
    let toward_white = |c: f32| c + (1.0 - c) * t;
    let intensity = 1.2 + activation * 7.0;
    LinearRgba::rgb(
        toward_white(base.0) * intensity,
        toward_white(base.1) * intensity,
        toward_white(base.2) * intensity,
    )
}

fn bounds(points: &[Vec3]) -> (Vec3, f32) {
    if points.is_empty() {
        return (Vec3::ZERO, 30.0);
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(10.0);
    (center, radius)
}

fn spawn_camera(commands: &mut Commands, focus: Vec3, radius: f32) {
    let cam = OrbitCamera {
        focus,
        radius: radius * 1.35,
        yaw: 0.5,
        pitch: 0.35,
    };
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));
}

/// Build a glowing line-strip mesh from a polyline (with normals/uvs so PBR is happy).
fn line_mesh(points: &[Vec3]) -> Mesh {
    let positions: Vec<[f32; 3]> = points.iter().map(|v| v.to_array()).collect();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; points.len()];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; points.len()];
    Mesh::new(
        PrimitiveTopology::LineStrip,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    db_path: Res<DbPath>,
) {
    let path = &db_path.0;
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => {
            warn!("could not open '{path}' ({e}); showing empty scene. If it's encrypted, decrypt or pass plaintext.");
            spawn_camera(&mut commands, Vec3::ZERO, 30.0);
            return;
        }
    };
    let snap = match nbe_data::snapshot::export(&db) {
        Ok(s) => s,
        Err(e) => {
            warn!("could not read '{path}' ({e})");
            spawn_camera(&mut commands, Vec3::ZERO, 30.0);
            return;
        }
    };

    if snap.entities.is_empty() {
        warn!("no data in '{path}'. Seed a demo graph with:  nbe --db {path} seed");
        spawn_camera(&mut commands, Vec3::ZERO, 30.0);
        return;
    }
    info!(
        "loaded {} entities, {} edges from {path}",
        snap.entities.len(),
        snap.edges.len()
    );

    // Layer -> column mapping (derive hidden-layer count from the data).
    let mut max_hidden = 0u8;
    for l in &snap.layers {
        if let Some(Layer::Hidden(n)) = Layer::from_db(&l.layer) {
            max_hidden = max_hidden.max(n);
        }
    }
    let last_col = max_hidden as u32 + 1;
    let column = |s: &str| match Layer::from_db(s) {
        Some(Layer::Input) => 0,
        Some(Layer::Hidden(n)) => n as u32 + 1,
        Some(Layer::Output) => last_col,
        None => 0,
    };

    let nodes: Vec<LayoutNode> = snap
        .layers
        .iter()
        .map(|l| LayoutNode {
            id: l.entity_id.clone(),
            column: column(&l.layer),
        })
        .collect();
    let edges: Vec<(String, String)> = snap
        .edges
        .iter()
        .map(|e| (e.source_id.clone(), e.target_id.clone()))
        .collect();

    let layout = layered_layout(&nodes, &edges, &LayoutParams::default());

    // Even "sunflower" distribution of each layer's nodes across a disc (in the y/z plane),
    // with a touch of jitter — organized and clean, but still organic. x advances by layer.
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt()); // ~2.39996 rad
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for np in &layout.nodes {
        *counts.entry(np.column).or_default() += 1;
    }
    let mut seen: HashMap<u32, usize> = HashMap::new();
    let mut pos: HashMap<String, Vec3> = HashMap::new();
    for np in &layout.nodes {
        let i = {
            let e = seen.entry(np.column).or_default();
            let v = *e;
            *e += 1;
            v
        };
        let n = (*counts.get(&np.column).unwrap_or(&1)).max(1);
        let frac = (i as f32 + 0.5) / n as f32;
        let radius = SPREAD * frac.sqrt();
        let angle = i as f32 * golden;
        let jitter = SPREAD * 0.06;
        let x = np.column as f32 * LAYER_GAP + rand_unit(&np.id, 1) * X_JITTER;
        let y = radius * angle.cos() + rand_unit(&np.id, 7) * jitter;
        let z = radius * angle.sin() + rand_unit(&np.id, 8) * jitter;
        pos.insert(np.id.clone(), Vec3::new(x, y, z));
    }
    let activation: HashMap<String, f64> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.clone(), a.value))
        .collect();

    // Nodes: one shared sphere mesh, per-node emissive material + scale.
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    for np in &layout.nodes {
        let a = *activation.get(&np.id).unwrap_or(&0.0) as f32;
        let r = 0.18 + a * 0.45;
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.06, 0.14),
            emissive: node_emissive(a, np.column, last_col),
            perceptual_roughness: 0.4,
            ..default()
        });
        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos[&np.id]).with_scale(Vec3::splat(r)),
        ));
    }

    // Edges: organic curved synapses. We keep only the stronger, shorter, clearly
    // node-to-node connections so the web stays clean (not a crisscross of long lines).
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.4),
        emissive: LinearRgba::rgb(0.05, 0.30, 0.70),
        alpha_mode: AlphaMode::Blend,
        unlit: false,
        ..default()
    });
    let curve_params = CurveParams {
        samples: 18,
        bow: 0.12,
        sag: 0.03,
        jitter: 0.015,
        seed: 0x00E6,
    };
    for e in &snap.edges {
        let (Some(&a), Some(&b)) = (pos.get(&e.source_id), pos.get(&e.target_id)) else {
            continue;
        };
        // Cull weak and overly long connections to declutter the web.
        if e.weight < EDGE_WEIGHT_MIN || (b - a).length() > MAX_EDGE_LEN {
            continue;
        }
        let curve = edge_curve(a, b, e.weight as f32, &curve_params, hash_u64(&e.id));
        let mesh = meshes.add(line_mesh(&curve));
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(edge_mat.clone()), Transform::default()));
    }

    let all: Vec<Vec3> = pos.values().copied().collect();
    let (center, radius) = bounds(&all);
    spawn_camera(&mut commands, center, radius);
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("loading…"),
        TextFont {
            font_size: 15.0,
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
        text.0 = format!("{fps:5.0} FPS   |   {frame_ms:5.2} ms   |   drag: orbit  scroll: zoom");
    }
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let mut drag = Vec2::ZERO;
    if mouse_buttons.pressed(MouseButton::Left) {
        for ev in motion.read() {
            drag += ev.delta;
        }
    } else {
        motion.clear();
    }

    let mut scroll = 0.0;
    for ev in wheel.read() {
        scroll += ev.y;
    }

    for (mut orbit, mut transform) in &mut query {
        orbit.yaw -= drag.x * 0.005;
        orbit.pitch = (orbit.pitch + drag.y * 0.005).clamp(-1.4, 1.4);
        let factor = (1.0 - scroll * 0.08).clamp(0.5, 2.0);
        orbit.radius = (orbit.radius * factor).clamp(3.0, 600.0);
        *transform = Transform::from_translation(orbit.eye()).looking_at(orbit.focus, Vec3::Y);
    }
}
