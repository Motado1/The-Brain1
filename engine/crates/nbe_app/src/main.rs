//! The Neural Business Engine — renderer + navigation.
//!
//! Loads the graph from SQLite (`--db <path>`, default `brain.db`) and renders it as two distinct
//! **networks** floating far apart in space, like separate constellations:
//!   * **Business** — CRM clients + their financial ledger, one interconnected web. Per-client
//!     revenue (summed from connected ledger entries) and renewal dates surface in the sidebar.
//!   * **Research** — the knowledge base, off in the distance, growing its own web as notes link
//!     to topic hubs.
//!
//! Nodes are glowing neurons that fire and flare from their activation; firing propagates pulses
//! of light along the glass-tube edges to neighbours, a real cascade. Ambient twinkle + drifting
//! dust keep it alive. Camera: left-drag orbits, right/middle-drag pans, scroll flies, Esc unfocuses.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::image::Image;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};
use bevy::render::view::Hdr;
use bevy::render::RenderPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use nbe_cli::datetime::format_date;
use nbe_cli::money::format_cents;
use nbe_geometry::{edge_curve, CurveParams};

#[derive(Resource)]
struct DbPath(String);

// ---- business panel (live reports from the hub) ----------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum BizTab {
    Agenda,
    Sessions,
    Renewals,
    Forecast,
    Revenue,
    Retention,
}

impl BizTab {
    const ALL: [BizTab; 6] = [
        BizTab::Agenda,
        BizTab::Sessions,
        BizTab::Renewals,
        BizTab::Forecast,
        BizTab::Revenue,
        BizTab::Retention,
    ];

    fn label(self) -> &'static str {
        match self {
            BizTab::Agenda => "Agenda",
            BizTab::Sessions => "Sessions",
            BizTab::Renewals => "Renewals",
            BizTab::Forecast => "Forecast",
            BizTab::Revenue => "Revenue",
            BizTab::Retention => "Retention",
        }
    }
}

#[derive(Resource)]
struct BusinessPanel {
    tab: BizTab,
    text: String,
}

impl Default for BusinessPanel {
    fn default() -> Self {
        Self {
            tab: BizTab::Agenda,
            text: String::new(),
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open the DB read-only and render the chosen report to text (reuses the CLI `ops` handlers).
fn run_report(path: &str, tab: BizTab) -> String {
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => return format!("cannot open {path}: {e}"),
    };
    let now = now_unix();
    let res = match tab {
        BizTab::Agenda => nbe_cli::ops::agenda(&db, 7, now),
        BizTab::Sessions => nbe_cli::ops::report_sessions(&db, now),
        BizTab::Renewals => nbe_cli::ops::report_renewals(&db, 30, now),
        BizTab::Forecast => nbe_cli::ops::report_forecast(&db, 6, now),
        BizTab::Revenue => nbe_cli::ops::report_revenue(&db),
        BizTab::Retention => nbe_cli::ops::report_retention(&db),
    };
    res.unwrap_or_else(|e| format!("error: {e}"))
}

// ---- networks + node kinds -------------------------------------------------------------

/// A self-contained graph, rendered far from its sibling so each reads as its own constellation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Network {
    Business,
    Research,
}

impl Network {
    const ALL: [Network; 2] = [Network::Business, Network::Research];

    fn label(self) -> &'static str {
        match self {
            Network::Business => "Business — Clients & Ledger",
            Network::Research => "Research — Knowledge",
        }
    }

    /// World-space centre. Research sits far away so it hangs in the sky like a distant cluster.
    fn center(self) -> Vec3 {
        match self {
            Network::Business => Vec3::ZERO,
            Network::Research => Vec3::new(850.0, 130.0, -380.0),
        }
    }

    /// Ellipsoid extents the network's nodes fill.
    fn radii(self) -> Vec3 {
        match self {
            Network::Business => Vec3::new(95.0, 72.0, 95.0),
            Network::Research => Vec3::new(60.0, 48.0, 60.0),
        }
    }
}

/// What a node *is* — drives colour and size. Kind determines which network it lives in.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Client,
    Ledger,
    Knowledge,
}

impl Kind {
    fn network(self) -> Network {
        match self {
            Kind::Knowledge => Network::Research,
            Kind::Client | Kind::Ledger => Network::Business,
        }
    }

    /// Base emissive hue (pre activation/intensity) — warm amber / honey / red family.
    fn base_color(self) -> (f32, f32, f32) {
        match self {
            Kind::Client => (1.0, 0.55, 0.13),    // honey amber
            Kind::Ledger => (1.0, 0.22, 0.08),    // deep red
            Kind::Knowledge => (1.0, 0.78, 0.32), // warm gold
        }
    }

    /// Base node radius — clients are the big "neurons".
    fn base_size(self) -> f32 {
        match self {
            Kind::Client => 1.0,
            Kind::Ledger => 0.45,
            Kind::Knowledge => 0.6,
        }
    }

    /// How far out in the network's shell this kind tends to sit (0 = core, 1 = surface).
    fn shell(self) -> f32 {
        match self {
            Kind::Client => 0.3,
            Kind::Ledger => 0.65,
            Kind::Knowledge => 0.45,
        }
    }
}

const EDGE_WEIGHT_MIN: f64 = 0.55;

// ---- "alive" layer tuning (iterate on these from a screenshot) -------------------------
/// How fast activation charges a neuron toward its threshold (higher = fires more often).
const FIRE_RATE: f32 = 0.4;
/// Flare brightness decay rate (higher = snappier flash).
const FIRE_DECAY: f32 = 3.0;
/// Peak emissive multiplier at full flare.
const FLARE_GAIN: f32 = 5.0;
/// How much a fired neuron's halo swells at peak.
const HALO_SWELL: f32 = 0.8;
/// Energy a propagation pulse deposits in its target (threshold ~0.5, so it takes coincidence to
/// re-fire — cascades stay sparse and fade).
const PULSE_ENERGY: f32 = 0.22;
/// Max pulses one fire emits (caps hub blow-ups).
const MAX_PULSES_PER_FIRE: usize = 5;
/// Per-network dust mote count.
const MOTES_PER_NETWORK: usize = 70;

// ---- navigation registry + camera target ----------------------------------------------

struct NodeInfo {
    name: String,
    kind: Kind,
    network: Network,
    pos: Vec3,
    /// Summed income from connected ledger entries (clients only).
    revenue_cents: Option<i64>,
    /// Client renewal date (unix seconds).
    renewal: Option<i64>,
}

#[derive(Resource, Default)]
struct NodeRegistry {
    nodes: Vec<NodeInfo>,
    galaxy_center: Vec3,
    galaxy_radius: f32,
    total_revenue_cents: i64,
}

impl NodeRegistry {
    /// Focus + radius to frame a whole network.
    fn network_view(&self, network: Network) -> (Vec3, f32) {
        let pts: Vec<Vec3> = self
            .nodes
            .iter()
            .filter(|n| n.network == network)
            .map(|n| n.pos)
            .collect();
        if pts.is_empty() {
            return (network.center(), 200.0);
        }
        let center = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
        let radius = pts
            .iter()
            .map(|p| p.distance(center))
            .fold(0.0_f32, f32::max)
            .max(20.0);
        (center, radius * 1.4)
    }
}

/// When set, the camera smoothly flies to this (focus, radius).
#[derive(Resource, Default)]
struct CameraTarget(Option<(Vec3, f32)>);

/// True while the mouse pointer is over an egui panel — suppresses camera input so scrolling a
/// sidebar list doesn't also zoom the 3D view.
#[derive(Resource, Default)]
struct UiPointer {
    over: bool,
}

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
        .add_plugins(EguiPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::srgb(0.022, 0.012, 0.008)))
        .insert_resource(DbPath(db_path_from_args()))
        .insert_resource(CameraTarget::default())
        .insert_resource(NodeRegistry::default())
        .insert_resource(BrainGraph::default())
        .insert_resource(BusinessPanel::default())
        .insert_resource(SceneControl::default())
        .insert_resource(UiPointer::default())
        .add_systems(Startup, (load_graph, setup_hud, spawn_lights))
        .add_systems(
            EguiPrimaryContextPass,
            (sidebar_ui, business_panel_ui, sync_ui_pointer).chain(),
        )
        .add_systems(
            Update,
            (
                orbit_camera,
                update_dof,
                face_camera,
                update_hud,
                animate_breath,
                fire_scheduler,
                fire_render,
                advance_pulses,
                drift_motes,
                apply_reload,
            ),
        )
        .run();
}

// ---- components / helpers --------------------------------------------------------------

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

/// A camera-facing quad — the soft glow halo behind a neuron.
#[derive(Component)]
struct Billboard;

#[derive(Component)]
struct Breath {
    base: f32,
    phase: f32,
    speed: f32,
}

/// Travelling pulse of light along an edge — spawned when a neuron fires and propagates to its
/// neighbour. Replaces the old random-looping spark.
#[derive(Component)]
struct Pulse {
    edge: usize,
    t: f32,
    speed: f32,
    target: usize,
    energy: f32,
}

/// A drifting dust mote, for ambient depth (the bokeh specks in the references).
#[derive(Component)]
struct Mote {
    vel: Vec3,
    center: Vec3,
    radius: f32,
}

/// Links a spawned neuron entity back to its slot in the `BrainGraph`.
#[derive(Component)]
struct Neuron(usize);

/// Integrate-and-fire state for a neuron.
#[derive(Component)]
struct Firing {
    accumulator: f32,
    intensity: f32,
}

/// Cached base look + the handles a firing flare needs to drive.
#[derive(Component)]
struct NodeViz {
    base_emissive: LinearRgba,
    base_radius: f32,
    halo: Entity,
    mat: Handle<StandardMaterial>,
    phase: f32,
    twinkle: f32,
}

/// The addressable graph the animation systems read: who connects to whom, along which path.
#[derive(Resource, Default)]
struct BrainGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

struct GraphNode {
    entity: Entity,
    activation: f32,
    threshold: f32,
    out: Vec<usize>, // edge indices leaving this node
}

struct GraphEdge {
    path: Vec<Vec3>,
    target: usize,
}

/// Shared mesh/material for spawning propagation pulses at runtime.
#[derive(Resource)]
struct PulseAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Tags every entity the scene builder spawns (nodes, edges, sparks) so a rebuild can despawn the
/// whole graph and re-create it from the DB — the camera/HUD (untagged) persist.
#[derive(Component)]
struct SceneItem;

/// Cross-system signal + status line for button actions that change the DB and need a redraw.
#[derive(Resource, Default)]
struct SceneControl {
    reload: bool,
    status: String,
}

fn hash_u64(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Deterministic value in [-1, 1] from an id + salt.
fn rand_unit(id: &str, salt: u64) -> f32 {
    let mut h = hash_u64(id) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 29)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 32;
    (h >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0
}

fn rand01(id: &str, salt: u64) -> f32 {
    rand_unit(id, salt) * 0.5 + 0.5
}

/// Evenly distributed direction on the unit sphere (Fibonacci).
fn fib_dir(i: usize, n: usize) -> Vec3 {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * ((i as f32 + 0.5) / n as f32);
    let r = (1.0 - y * y).max(0.0).sqrt();
    let th = i as f32 * golden;
    Vec3::new(r * th.cos(), y, r * th.sin())
}

/// Place a node inside its network's ellipsoid, biased toward its kind's shell radius.
fn net_pos(network: Network, kind: Kind, i: usize, n: usize, id: &str) -> Vec3 {
    let dir = fib_dir(i, n);
    let jitter = Vec3::new(rand_unit(id, 12), rand_unit(id, 13), rand_unit(id, 14)) * 4.0;
    let shell = kind.shell();
    let rr = shell + (1.0 - shell) * rand01(id, 11);
    network.center() + (dir * rr) * network.radii() + jitter
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn sample_path(points: &[Vec3], t: f32) -> Vec3 {
    match points.len() {
        0 => Vec3::ZERO,
        1 => points[0],
        n => {
            let tt = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = tt.floor() as usize;
            let f = tt - i as f32;
            let j = (i + 1).min(n - 1);
            points[i].lerp(points[j], f)
        }
    }
}

/// Emissive (HDR) for a node: kind hue at rest, shifting to a warm amber hotspot as it activates.
fn node_emissive(kind: Kind, activation: f32) -> LinearRgba {
    let (br, bg, bb) = kind.base_color();
    let (hr, hg, hb) = (1.0, 0.92, 0.7); // white-hot honey core
    let t = activation.clamp(0.0, 1.0);
    let mix = |b: f32, h: f32| b + (h - b) * t * 0.9;
    let intensity = 0.9 + activation * 7.0;
    LinearRgba::rgb(
        mix(br, hr) * intensity,
        mix(bg, hg) * intensity,
        mix(bb, hb) * intensity,
    )
}

fn bounds(points: &[Vec3]) -> (Vec3, f32) {
    if points.is_empty() {
        return (Vec3::ZERO, 200.0);
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(20.0);
    (center, radius)
}

/// Accumulating mesh buffers so many tube strips can share one mesh (one draw call per neuron's
/// whole dendrite tree, say).
#[derive(Default)]
struct TubeBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl TubeBuilder {
    /// Sweep a tube along `points` with a per-point `radii` profile (lets tubes taper / bulge).
    fn add(&mut self, points: &[Vec3], radii: &[f32], sides: usize) {
        if points.len() < 2 {
            return;
        }
        let rings = points.len();
        let base = self.positions.len() as u32;
        for (ri, &p) in points.iter().enumerate() {
            let tangent = if ri == 0 {
                points[1] - points[0]
            } else if ri == rings - 1 {
                points[ri] - points[ri - 1]
            } else {
                points[ri + 1] - points[ri - 1]
            }
            .normalize_or_zero();
            let t = if tangent.length_squared() < 1e-6 {
                Vec3::Z
            } else {
                tangent
            };
            let up = if t.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            let n0 = t.cross(up).normalize();
            let b0 = t.cross(n0).normalize();
            let r = radii[ri.min(radii.len() - 1)];
            for s in 0..sides {
                let a = s as f32 / sides as f32 * std::f32::consts::TAU;
                let dir = n0 * a.cos() + b0 * a.sin();
                self.positions.push((p + dir * r).to_array());
                self.normals.push(dir.to_array());
                self.uvs
                    .push([ri as f32 / (rings - 1) as f32, s as f32 / sides as f32]);
            }
        }
        for ri in 0..rings - 1 {
            for s in 0..sides {
                let s2 = (s + 1) % sides;
                let a = base + (ri * sides + s) as u32;
                let b = base + (ri * sides + s2) as u32;
                let c = base + ((ri + 1) * sides + s) as u32;
                let d = base + ((ri + 1) * sides + s2) as u32;
                self.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }

    fn build(self) -> Mesh {
        let usages = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
        Mesh::new(PrimitiveTopology::TriangleList, usages)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Single tapered tube as its own mesh.
fn tube_mesh(points: &[Vec3], radii: &[f32], sides: usize) -> Mesh {
    let mut b = TubeBuilder::default();
    b.add(points, radii, sides);
    b.build()
}

/// Connection profile: fat at both endpoints (where it meets the somas), pinched in the middle —
/// the organic dendrite-junction look rather than an even pipe.
fn connection_radii(n: usize, base: f32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            base * (0.4 + 0.6 * (2.0 * t - 1.0).abs())
        })
        .collect()
}

/// Dendrite profile: thick at the root, tapering smoothly to a hair-thin tip.
fn dendrite_radii(n: usize, base: f32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            base * (1.0 - t).powf(1.5) + 0.012
        })
        .collect()
}

/// Grow a small tree of wandering, tapering filaments out of a neuron — the dendrites that make it
/// read as a living cell instead of a dot. Returns one merged mesh.
fn dendrite_mesh(node: Vec3, count: usize, node_r: f32, seed: u64) -> Mesh {
    let mut s = seed | 1;
    let mut builder = TubeBuilder::default();
    for _ in 0..count {
        let segs = 5 + (lcg(&mut s) * 3.0) as usize;
        let seg_len = node_r * (1.4 + lcg(&mut s) * 1.2);
        let mut dir =
            Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5).normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            dir = Vec3::Y;
        }
        let mut p = node;
        let mut pts = vec![p];
        for _ in 0..segs {
            let jitter =
                Vec3::new(lcg(&mut s) - 0.5, lcg(&mut s) - 0.5, lcg(&mut s) - 0.5) * 0.55;
            dir = (dir + jitter).normalize_or_zero();
            p += dir * seg_len;
            pts.push(p);
        }
        let radii = dendrite_radii(pts.len(), node_r * 0.28);
        builder.add(&pts, &radii, 5);
    }
    builder.build()
}

/// A soft round radial-gradient texture (white core fading to transparent) for the glow halos.
fn glow_texture() -> Image {
    const N: usize = 64;
    let mut data = vec![0u8; N * N * 4];
    let c = (N as f32 - 1.0) * 0.5;
    for y in 0..N {
        for x in 0..N {
            let d = (((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt()) / c;
            // Smooth falloff, squared for a tighter hot core.
            let a = (1.0 - d).clamp(0.0, 1.0).powf(2.2);
            let i = (y * N + x) * 4;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
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
        // Far plane reaches the distant Research network so it stays visible from Business.
        Projection::from(PerspectiveProjection {
            far: 20_000.0,
            near: 0.1,
            ..default()
        }),
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.3,
            ..Bloom::NATURAL
        },
        // Warm dark haze so distant neurons melt into amber depth rather than a hard cutoff.
        DistanceFog {
            color: Color::srgb(0.05, 0.022, 0.012),
            falloff: FogFalloff::ExponentialSquared { density: 0.00055 },
            ..default()
        },
        // Gentle depth-of-field: the focused cluster stays crisp, far/near soften like the refs.
        DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: cam.radius,
            aperture_f_stops: 2.8,
            ..default()
        },
        // A touch of warm ambient so the glass tubes aren't pure black where unlit.
        AmbientLight {
            color: Color::srgb(1.0, 0.7, 0.45),
            brightness: 60.0,
            ..default()
        },
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));
}

// ---- scene build -----------------------------------------------------------------------

/// Recompute and persist every neuron's activation from its facets (idempotent). Best-effort —
/// a missing/locked DB just leaves the stored values in place.
fn recompute_activations(path: &str) {
    match nbe_data::Db::open(path, None) {
        Ok(mut db) => {
            if let Err(e) = nbe_cli::ops::recompute_activation(&mut db, now_unix()) {
                warn!("recompute activation failed: {e}");
            }
        }
        Err(e) => warn!("recompute: cannot open '{path}': {e}"),
    }
}

fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
) {
    let (center, radius) = build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
        images.as_mut(),
        registry.as_mut(),
        &db_path.0,
    )
    .unwrap_or((Vec3::ZERO, 200.0));
    spawn_camera(&mut commands, center, radius);
}

/// Rebuild the graph from the DB when a button has changed it: despawn the old scene and re-create
/// it. The camera persists (untagged), so the view stays put while new neurons pop in.
#[allow(clippy::too_many_arguments)]
fn apply_reload(
    mut control: ResMut<SceneControl>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
    old: Query<Entity, With<SceneItem>>,
) {
    if !control.reload {
        return;
    }
    control.reload = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
        images.as_mut(),
        registry.as_mut(),
        &db_path.0,
    );
}

/// Load the DB and spawn both networks (nodes, edges, sparks), all tagged `SceneItem`. Returns the
/// framing bounds `(center, radius)` of the **Business** network for the initial camera, or `None`
/// if the DB is missing/empty.
fn build_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    registry: &mut NodeRegistry,
    path: &str,
) -> Option<(Vec3, f32)> {
    registry.nodes.clear();
    // Make activation honest before we read it: recompute each neuron's value from its real facet
    // urgency (recency, invoice status, renewal proximity) so firing rate reflects what matters.
    recompute_activations(path);
    let db = match nbe_data::Db::open(path, None) {
        Ok(d) => d,
        Err(e) => {
            warn!("could not open '{path}' ({e}); empty scene.");
            return None;
        }
    };
    let snap = match nbe_data::snapshot::export(&db) {
        Ok(s) => s,
        Err(e) => {
            warn!("could not read '{path}' ({e})");
            return None;
        }
    };
    if snap.entities.is_empty() {
        warn!("no data in '{path}'. Seed with:  nbe --db {path} seed");
        return None;
    }
    info!("loaded {} entities, {} edges", snap.entities.len(), snap.edges.len());

    // Kind + display name per entity (priority Client > Knowledge > Ledger).
    let mut kind_of: HashMap<String, Kind> = HashMap::new();
    let mut name_of: HashMap<String, String> = HashMap::new();
    for c in &snap.crm {
        kind_of.insert(c.entity_id.clone(), Kind::Client);
        name_of.insert(
            c.entity_id.clone(),
            c.contact.clone().unwrap_or_else(|| "client".into()),
        );
    }
    for k in &snap.knowledge {
        kind_of.entry(k.entity_id.clone()).or_insert(Kind::Knowledge);
        let title = k.body_md.lines().next().unwrap_or("note").trim_start_matches("# ");
        name_of.entry(k.entity_id.clone()).or_insert_with(|| title.to_string());
    }
    for l in &snap.ledger {
        kind_of.entry(l.entity_id.clone()).or_insert(Kind::Ledger);
        name_of
            .entry(l.entity_id.clone())
            .or_insert_with(|| format!("{} [{}]", format_cents(l.amount_cents), l.invoice_status));
    }

    let activation: HashMap<String, f64> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.clone(), a.value))
        .collect();

    // Financial roll-up: per-client revenue from connected (non-expense) ledger entries, plus a
    // grand total. This is the "combine financial into the client network" surfacing.
    let ledger_amt: HashMap<&str, (i64, bool)> = snap
        .ledger
        .iter()
        .map(|l| (l.entity_id.as_str(), (l.amount_cents, l.is_expense)))
        .collect();
    let client_ids: HashSet<&str> = snap.crm.iter().map(|c| c.entity_id.as_str()).collect();
    let renewal_of: HashMap<&str, Option<i64>> =
        snap.crm.iter().map(|c| (c.entity_id.as_str(), c.renewal_date)).collect();
    let mut revenue_of: HashMap<String, i64> = HashMap::new();
    for e in &snap.edges {
        if client_ids.contains(e.source_id.as_str()) {
            if let Some(&(amt, exp)) = ledger_amt.get(e.target_id.as_str()) {
                if !exp {
                    *revenue_of.entry(e.source_id.clone()).or_default() += amt;
                }
            }
        }
        if client_ids.contains(e.target_id.as_str()) {
            if let Some(&(amt, exp)) = ledger_amt.get(e.source_id.as_str()) {
                if !exp {
                    *revenue_of.entry(e.target_id.clone()).or_default() += amt;
                }
            }
        }
    }
    registry.total_revenue_cents = snap
        .ledger
        .iter()
        .filter(|l| !l.is_expense)
        .map(|l| l.amount_cents)
        .sum();

    // Group entity ids by network (sorted for deterministic positions).
    let mut groups: HashMap<Network, Vec<String>> = HashMap::new();
    for e in &snap.entities {
        let kind = *kind_of.get(&e.id).unwrap_or(&Kind::Knowledge);
        groups.entry(kind.network()).or_default().push(e.id.clone());
    }
    for ids in groups.values_mut() {
        ids.sort();
    }

    // Position each network's nodes inside its ellipsoid.
    let mut pos: HashMap<String, Vec3> = HashMap::new();
    for (&network, ids) in &groups {
        let n = ids.len().max(1);
        for (i, id) in ids.iter().enumerate() {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            pos.insert(id.clone(), net_pos(network, kind, i, n, id));
        }
    }

    // Spawn nodes + build the navigation registry.
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    let halo_quad = meshes.add(Rectangle::new(1.0, 1.0));
    let glow = images.add(glow_texture());
    let halo_for = |kind: Kind, materials: &mut Assets<StandardMaterial>| {
        let (r, g, b) = kind.base_color();
        materials.add(StandardMaterial {
            // base_color > 1 → HDR halo that blooms; multiplied by the radial-gradient texture.
            base_color: Color::LinearRgba(LinearRgba::new(r * 2.6, g * 2.6, b * 2.6, 1.0)),
            base_color_texture: Some(glow.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            ..default()
        })
    };
    let halo_mats = [
        (Kind::Client, halo_for(Kind::Client, materials)),
        (Kind::Ledger, halo_for(Kind::Ledger, materials)),
        (Kind::Knowledge, halo_for(Kind::Knowledge, materials)),
    ];
    // Warm translucent glass for the dendrite filaments.
    let dendrite_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.55, 0.22, 0.16),
        emissive: LinearRgba::rgb(0.5, 0.17, 0.04),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.1,
        metallic: 0.0,
        ..default()
    });

    // Per-entity firing threshold (defaults 0.5).
    let threshold_of: HashMap<&str, f32> = snap
        .activations
        .iter()
        .map(|a| (a.entity_id.as_str(), a.threshold as f32))
        .collect();

    let mut graph = BrainGraph::default();
    let mut index: HashMap<String, usize> = HashMap::new();

    for (&network, ids) in &groups {
        for id in ids {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            let act = *activation.get(id).unwrap_or(&0.0) as f32;
            let thr = threshold_of.get(id.as_str()).copied().unwrap_or(0.5);
            let r = kind.base_size() + act * 0.45;
            let p = pos[id];
            let base_emissive = node_emissive(kind, act);
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.03, 0.01),
                emissive: base_emissive,
                perceptual_roughness: 0.4,
                ..default()
            });
            // Soft glow halo (camera-facing) — spawned first so the neuron can flare it.
            let halo_mat = halo_mats
                .iter()
                .find(|(k, _)| *k == kind)
                .map(|(_, h)| h.clone())
                .unwrap();
            let halo = commands
                .spawn((
                    Mesh3d(halo_quad.clone()),
                    MeshMaterial3d(halo_mat),
                    Transform::from_translation(p).with_scale(Vec3::splat(r * 5.5)),
                    Billboard,
                    SceneItem,
                ))
                .id();

            let idx = graph.nodes.len();
            let node = commands
                .spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(r)),
                    Breath {
                        base: r,
                        phase: rand_unit(id, 9) * std::f32::consts::TAU,
                        speed: 0.6 + rand01(id, 10) * 0.8,
                    },
                    Neuron(idx),
                    // Random initial charge so neurons don't all fire in lockstep.
                    Firing {
                        accumulator: rand01(id, 7) * thr,
                        intensity: 0.0,
                    },
                    NodeViz {
                        base_emissive,
                        base_radius: r,
                        halo,
                        mat: mat.clone(),
                        phase: rand_unit(id, 8) * std::f32::consts::TAU,
                        twinkle: 0.08 + rand01(id, 15) * 0.08,
                    },
                    SceneItem,
                ))
                .id();

            graph.nodes.push(GraphNode {
                entity: node,
                activation: act,
                threshold: thr,
                out: Vec::new(),
            });
            index.insert(id.clone(), idx);

            // Radiating dendrites — clients sprout the most, ledger entries none (keeps it clean).
            let dcount = match kind {
                Kind::Client => 5,
                Kind::Knowledge => 4,
                Kind::Ledger => 0,
            };
            if dcount > 0 {
                commands.spawn((
                    Mesh3d(meshes.add(dendrite_mesh(p, dcount, r, hash_u64(id)))),
                    MeshMaterial3d(dendrite_mat.clone()),
                    Transform::default(),
                    SceneItem,
                ));
            }
            registry.nodes.push(NodeInfo {
                name: name_of.get(id).cloned().unwrap_or_else(|| id[..8].to_string()),
                kind,
                network,
                pos: p,
                revenue_cents: (kind == Kind::Client).then(|| revenue_of.get(id).copied().unwrap_or(0)),
                renewal: if kind == Kind::Client {
                    renewal_of.get(id.as_str()).copied().flatten()
                } else {
                    None
                },
            });
        }
    }

    // Edges: tapered glass tubes (stronger ones only). Only ever drawn between two nodes in the
    // *same* network — no lines stretch across the void. Clear amber glass: low roughness gives a
    // sharp specular streak under the lights (the round-tube cue), a faint warm tint + low alpha
    // let you see through, emissive keeps a glow.
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.62, 0.28, 0.18),
        emissive: LinearRgba::rgb(0.8, 0.32, 0.08),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.06,
        metallic: 0.0,
        reflectance: 0.7,
        ..default()
    });
    let curve_params = CurveParams {
        samples: 14,
        bow: 0.1,
        sag: 0.03,
        jitter: 0.015,
        seed: 0x00E6,
    };
    let add_tube = |commands: &mut Commands, meshes: &mut Assets<Mesh>, curve: &[Vec3]| {
        let radii = connection_radii(curve.len(), 0.32);
        commands.spawn((
            Mesh3d(meshes.add(tube_mesh(curve, &radii, 8))),
            MeshMaterial3d(edge_mat.clone()),
            Transform::default(),
            SceneItem,
        ));
    };
    // Wire a path into the graph as directed src→dst (and the reverse if undirected) for propagation.
    fn wire(graph: &mut BrainGraph, si: usize, di: usize, curve: &[Vec3], directed: bool) {
        let e1 = graph.edges.len();
        graph.edges.push(GraphEdge {
            path: curve.to_vec(),
            target: di,
        });
        graph.nodes[si].out.push(e1);
        if !directed {
            let mut rev = curve.to_vec();
            rev.reverse();
            let e2 = graph.edges.len();
            graph.edges.push(GraphEdge { path: rev, target: si });
            graph.nodes[di].out.push(e2);
        }
    }

    for e in &snap.edges {
        if e.weight < EDGE_WEIGHT_MIN {
            continue;
        }
        let (Some(&a), Some(&b)) = (pos.get(&e.source_id), pos.get(&e.target_id)) else {
            continue;
        };
        let (Some(&si), Some(&di)) = (index.get(&e.source_id), index.get(&e.target_id)) else {
            continue;
        };
        let net_a = kind_of.get(&e.source_id).map(|k| k.network());
        let net_b = kind_of.get(&e.target_id).map(|k| k.network());
        if net_a != net_b {
            continue;
        }
        let curve = edge_curve(a, b, e.weight as f32, &curve_params, hash_u64(&e.id));
        add_tube(commands, meshes, &curve);
        wire(&mut graph, si, di, &curve, e.directed);
    }

    // The Research network's native links all point across to clients/ledger (dropped above), so
    // it arrives as loose dust. Weave a proximity web — each note to its 2 nearest neighbours — so
    // it reads as its own constellation. (Real Add-Research notes link to topic hubs over time.)
    if let Some(rids) = groups.get(&Network::Research) {
        let ridx: Vec<usize> = rids.iter().map(|id| index[id]).collect();
        let rpos: Vec<Vec3> = rids.iter().map(|id| pos[id]).collect();
        let mut linked: HashSet<(usize, usize)> = HashSet::new();
        for i in 0..rpos.len() {
            let mut nearest: Vec<(f32, usize)> = (0..rpos.len())
                .filter(|&j| j != i)
                .map(|j| (rpos[i].distance_squared(rpos[j]), j))
                .collect();
            nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(_, j) in nearest.iter().take(2) {
                let key = (i.min(j), i.max(j));
                if !linked.insert(key) {
                    continue;
                }
                let seed = (key.0 as u64) << 20 ^ key.1 as u64;
                let curve = edge_curve(rpos[i], rpos[j], 0.7, &curve_params, seed);
                add_tube(commands, meshes, &curve);
                wire(&mut graph, ridx[i], ridx[j], &curve, false);
            }
        }
    }

    // Pulse asset shared by all propagation pulses (spawned at runtime when neurons fire).
    let pulse_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(9.0, 5.0, 1.6),
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    commands.insert_resource(PulseAssets {
        mesh: sphere.clone(),
        material: pulse_material,
    });

    // Ambient dust motes drifting in each network's volume (the bokeh specks).
    let mote_mat = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(0.6, 0.34, 0.16, 1.0)),
        base_color_texture: Some(glow.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let mut ms = 0xD1B5_4A32u64;
    for network in Network::ALL {
        let center = network.center();
        let radius = network.radii().max_element() * 1.3;
        for _ in 0..MOTES_PER_NETWORK {
            let dir = Vec3::new(
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
            );
            let p = center + dir * radius;
            let vel = Vec3::new(
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
                lcg(&mut ms) - 0.5,
            ) * 2.0;
            let sz = 0.6 + lcg(&mut ms) * 1.2;
            commands.spawn((
                Mesh3d(halo_quad.clone()),
                MeshMaterial3d(mote_mat.clone()),
                Transform::from_translation(p).with_scale(Vec3::splat(sz)),
                Billboard,
                Mote {
                    vel,
                    center,
                    radius,
                },
                SceneItem,
            ));
        }
    }

    commands.insert_resource(graph);

    let all: Vec<Vec3> = pos.values().copied().collect();
    let (center, radius) = bounds(&all);
    registry.galaxy_center = center;
    registry.galaxy_radius = radius;
    // Frame the Business network for the opening shot (Research is a distant cluster behind it).
    Some(registry.network_view(Network::Business))
}

// ---- UI + camera -----------------------------------------------------------------------

fn sidebar_ui(
    mut contexts: EguiContexts,
    registry: Res<NodeRegistry>,
    mut target: ResMut<CameraTarget>,
    mut control: ResMut<SceneControl>,
    db_path: Res<DbPath>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::left("nav")
        .default_width(270.0)
        .show(ctx, |ui| {
            ui.heading("The Brain");
            if ui.button("➕ Add Research").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown", "txt"])
                    .pick_file()
                {
                    let mut reload = false;
                    let status = match nbe_data::Db::open(&db_path.0, None) {
                        Ok(mut db) => {
                            match nbe_cli::ops::note_import(&mut db, &file, &[], None, "draft", now_unix()) {
                                Ok(msg) => {
                                    reload = true;
                                    msg
                                }
                                Err(e) => format!("import failed: {e}"),
                            }
                        }
                        Err(e) => format!("cannot open db: {e}"),
                    };
                    control.status = status;
                    control.reload = reload;
                }
            }
            if !control.status.is_empty() {
                ui.label(&control.status);
            }
            if ui.button("Galaxy view").clicked() {
                target.0 = Some((registry.galaxy_center, registry.galaxy_radius * 1.35));
            }
            ui.small("left-drag orbit · right-drag pan · scroll fly · Esc unfocus");
            ui.separator();

            for network in Network::ALL {
                let count = registry.nodes.iter().filter(|n| n.network == network).count();
                egui::CollapsingHeader::new(format!("{} ({count})", network.label()))
                    .default_open(network == Network::Business)
                    .show(ui, |ui| {
                        if ui.button("→ go to network").clicked() {
                            let (c, r) = registry.network_view(network);
                            target.0 = Some((c, r));
                        }
                        if network == Network::Business {
                            ui.label(format!(
                                "Total revenue: {}",
                                format_cents(registry.total_revenue_cents)
                            ));
                            client_list(ui, &registry, &mut target);
                        } else {
                            knowledge_list(ui, &registry, &mut target);
                        }
                    });
            }
        });
}

/// Business clients, each expandable to its revenue + renewal date, with a fly-to button.
fn client_list(ui: &mut egui::Ui, registry: &NodeRegistry, target: &mut CameraTarget) {
    let mut items: Vec<(usize, &NodeInfo)> = registry
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == Kind::Client)
        .collect();
    items.sort_by(|(_, a), (_, b)| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    egui::ScrollArea::vertical()
        .max_height(320.0)
        .id_salt("clients")
        .show(ui, |ui| {
            for (idx, ni) in items {
                egui::CollapsingHeader::new(&ni.name)
                    .id_salt(idx)
                    .show(ui, |ui| {
                        ui.label(format!(
                            "Revenue: {}",
                            format_cents(ni.revenue_cents.unwrap_or(0))
                        ));
                        match ni.renewal {
                            Some(d) => ui.label(format!("Renewal: {}", format_date(d))),
                            None => ui.label("Renewal: —"),
                        };
                        if ui.button("→ fly to").clicked() {
                            target.0 = Some((ni.pos, 22.0));
                        }
                    });
            }
        });
}

/// Research notes, each a fly-to button.
fn knowledge_list(ui: &mut egui::Ui, registry: &NodeRegistry, target: &mut CameraTarget) {
    let mut items: Vec<&NodeInfo> = registry
        .nodes
        .iter()
        .filter(|n| n.kind == Kind::Knowledge)
        .collect();
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    egui::ScrollArea::vertical()
        .max_height(320.0)
        .id_salt("knowledge")
        .show(ui, |ui| {
            for ni in items {
                if ui.button(&ni.name).clicked() {
                    target.0 = Some((ni.pos, 18.0));
                }
            }
        });
}

/// Right-hand panel: live business reports from the hub, with a tab button per report and a
/// refresh button. Read-only — opens the DB on demand when a tab is clicked.
fn business_panel_ui(
    mut contexts: EguiContexts,
    mut panel: ResMut<BusinessPanel>,
    db_path: Res<DbPath>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    // Populate on first frame.
    if panel.text.is_empty() {
        let tab = panel.tab;
        panel.text = run_report(&db_path.0, tab);
    }
    egui::SidePanel::right("business")
        .default_width(330.0)
        .show(ctx, |ui| {
            ui.heading("Business");
            ui.horizontal_wrapped(|ui| {
                for tab in BizTab::ALL {
                    if ui.selectable_label(panel.tab == tab, tab.label()).clicked() {
                        panel.tab = tab;
                        panel.text = run_report(&db_path.0, tab);
                    }
                }
            });
            if ui.button("⟳ Refresh").clicked() {
                let tab = panel.tab;
                panel.text = run_report(&db_path.0, tab);
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.monospace(&panel.text);
            });
        });
}

/// Mark whether the pointer is over an egui panel, so the camera ignores scroll/drag there.
fn sync_ui_pointer(mut contexts: EguiContexts, mut ui: ResMut<UiPointer>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        ui.over = ctx.is_pointer_over_area() || ctx.wants_pointer_input();
    }
}

#[allow(clippy::too_many_arguments)]
fn orbit_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut target: ResMut<CameraTarget>,
    registry: Res<NodeRegistry>,
    ui: Res<UiPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut OrbitCamera, &mut Transform, &Camera, &GlobalTransform)>,
) {
    let orbiting = mouse_buttons.pressed(MouseButton::Left);
    let panning =
        mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Middle);

    let mut drag = Vec2::ZERO;
    if orbiting || panning {
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

    // Pointer over a panel? Don't let sidebar scrolling/dragging move the camera.
    if ui.over {
        drag = Vec2::ZERO;
        scroll = 0.0;
    }

    // Esc unfocuses to the whole-scene view.
    if keys.just_pressed(KeyCode::Escape) {
        target.0 = Some((registry.galaxy_center, registry.galaxy_radius * 1.35));
    }
    // Manual interaction cancels an active fly-to.
    if drag != Vec2::ZERO || scroll != 0.0 {
        target.0 = None;
    }

    let cursor = windows.single().ok().and_then(|w| w.cursor_position());

    for (mut orbit, mut transform, camera, cam_global) in &mut query {
        if let Some((focus, radius)) = target.0 {
            let k = (time.delta_secs() * 3.5).min(1.0);
            orbit.focus = orbit.focus.lerp(focus, k);
            orbit.radius += (radius - orbit.radius) * k;
            if orbit.focus.distance(focus) < 0.5 && (orbit.radius - radius).abs() < 0.5 {
                target.0 = None;
            }
        }
        if orbiting {
            orbit.yaw -= drag.x * 0.005;
            orbit.pitch = (orbit.pitch + drag.y * 0.005).clamp(-1.4, 1.4);
        }

        let eye = orbit.eye();
        let radius = orbit.radius;
        let forward = (orbit.focus - eye).normalize_or_zero();
        if panning && drag != Vec2::ZERO {
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            orbit.focus += (-right * drag.x + up * drag.y) * (radius * 0.0015);
        }
        if scroll != 0.0 {
            // Dolly toward the point under the cursor: fly along the ray through the mouse, so
            // scrolling targets where you're looking (falls back to screen-center if no cursor).
            let dir = cursor
                .and_then(|c| camera.viewport_to_world(cam_global, c).ok())
                .map(|ray| ray.direction.as_vec3())
                .unwrap_or(forward);
            orbit.focus += dir * scroll * (radius * 0.15 + 4.0);
        }
        orbit.radius = orbit.radius.clamp(1.0, 6000.0);
        *transform = Transform::from_translation(orbit.eye()).looking_at(orbit.focus, Vec3::Y);
    }
}

fn animate_breath(time: Res<Time>, mut query: Query<(&Breath, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (b, mut transform) in &mut query {
        let s = b.base * (1.0 + 0.12 * (t * b.speed + b.phase).sin());
        transform.scale = Vec3::splat(s);
    }
}

/// Integrate-and-fire: each neuron charges from its activation; when it crosses threshold it
/// flares and emits a pulse of light down each outgoing edge (a synapse firing).
fn fire_scheduler(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    pulse: Option<Res<PulseAssets>>,
    mut commands: Commands,
    mut q: Query<(&Neuron, &mut Firing)>,
) {
    let dt = time.delta_secs();
    for (neuron, mut firing) in &mut q {
        let Some(node) = graph.nodes.get(neuron.0) else {
            continue;
        };
        firing.intensity *= (-dt * FIRE_DECAY).exp();
        // floor keeps even quiet neurons occasionally firing → baseline life.
        firing.accumulator += node.activation.max(0.05) * dt * FIRE_RATE;
        if firing.accumulator >= node.threshold {
            firing.accumulator = 0.0;
            firing.intensity = 1.0;
            if let Some(pulse) = &pulse {
                let mut seed = neuron.0 as u64 ^ time.elapsed().as_nanos() as u64;
                for &e in node.out.iter().take(MAX_PULSES_PER_FIRE) {
                    let edge = &graph.edges[e];
                    let speed = 0.4 + lcg(&mut seed) * 0.4;
                    commands.spawn((
                        Mesh3d(pulse.mesh.clone()),
                        MeshMaterial3d(pulse.material.clone()),
                        Transform::from_translation(edge.path[0]),
                        Pulse {
                            edge: e,
                            t: 0.0,
                            speed,
                            target: edge.target,
                            energy: PULSE_ENERGY,
                        },
                        SceneItem,
                    ));
                }
            }
        }
    }
}

/// Render firing: flare the neuron's emissive + swell its halo, plus a constant gentle twinkle.
fn fire_render(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<(&Firing, &NodeViz)>,
    mut transforms: Query<&mut Transform>,
) {
    let t = time.elapsed_secs();
    for (firing, viz) in &q {
        let twinkle = 1.0 + (t * 1.7 + viz.phase).sin() * viz.twinkle;
        let mul = twinkle + firing.intensity * FLARE_GAIN;
        if let Some(m) = materials.get_mut(&viz.mat) {
            m.emissive = LinearRgba::rgb(
                viz.base_emissive.red * mul,
                viz.base_emissive.green * mul,
                viz.base_emissive.blue * mul,
            );
        }
        if let Ok(mut tr) = transforms.get_mut(viz.halo) {
            tr.scale = Vec3::splat(viz.base_radius * 5.5 * (1.0 + firing.intensity * HALO_SWELL));
        }
    }
}

/// Move propagation pulses along their edge; on arrival, deposit energy into the target neuron
/// (which may tip it over threshold → it fires → the cascade continues), then despawn.
fn advance_pulses(
    time: Res<Time>,
    graph: Res<BrainGraph>,
    mut commands: Commands,
    mut pulses: Query<(Entity, &mut Pulse, &mut Transform)>,
    mut fire: Query<&mut Firing>,
) {
    let dt = time.delta_secs();
    for (ent, mut pulse, mut tr) in &mut pulses {
        let Some(edge) = graph.edges.get(pulse.edge) else {
            commands.entity(ent).despawn();
            continue;
        };
        pulse.t += pulse.speed * dt;
        let tt = pulse.t.min(1.0);
        let path = &edge.path;
        let a = sample_path(path, (tt - 0.02).max(0.0));
        let b = sample_path(path, (tt + 0.02).min(1.0));
        tr.translation = sample_path(path, tt);
        let dir = (b - a).normalize_or_zero();
        if dir.length_squared() > 1e-6 {
            tr.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
        }
        let env = (tt * std::f32::consts::PI).sin().max(0.0);
        let glow = 0.2 + 0.8 * env;
        tr.scale = Vec3::new(0.5 * glow, 0.5 * glow, 3.2 * glow);
        if pulse.t >= 1.0 {
            if let Some(node) = graph.nodes.get(pulse.target) {
                if let Ok(mut f) = fire.get_mut(node.entity) {
                    f.accumulator += pulse.energy;
                }
            }
            commands.entity(ent).despawn();
        }
    }
}

/// Drift the ambient dust motes slowly, steering them back when they leave their network volume.
fn drift_motes(time: Res<Time>, mut q: Query<(&mut Mote, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut mote, mut tr) in &mut q {
        tr.translation += mote.vel * dt;
        let off = tr.translation - mote.center;
        if off.length() > mote.radius {
            let speed = mote.vel.length().max(0.5);
            mote.vel = -off.normalize_or_zero() * speed;
        }
    }
}

/// Turn the glow-halo quads to face the camera each frame.
fn face_camera(
    cam: Query<&GlobalTransform, With<Camera>>,
    mut halos: Query<&mut Transform, With<Billboard>>,
) {
    let Ok(cam) = cam.single() else {
        return;
    };
    let cam_pos = cam.translation();
    for mut t in &mut halos {
        let dir = (cam_pos - t.translation).normalize_or_zero();
        if dir.length_squared() > 1e-6 {
            t.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
        }
    }
}

/// Keep depth-of-field focused at the current orbit distance so the looked-at cluster stays crisp.
fn update_dof(mut query: Query<(&OrbitCamera, &mut DepthOfField)>) {
    for (orbit, mut dof) in &mut query {
        dof.focal_distance = orbit.radius;
    }
}

/// Two directional lights from opposite sides — they don't brighten the scene much (nodes are
/// emissive) but give the glass tubes a specular streak so they read as round 3D tubes.
fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.82, 0.55),
            illuminance: 4500.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.2, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.45, 0.25),
            illuminance: 2000.0,
            ..default()
        },
        Transform::from_xyz(-1.0, -0.5, -0.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));
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
            right: Val::Px(12.0),
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
    for mut text in &mut query {
        text.0 = format!("{fps:5.0} FPS");
    }
}
