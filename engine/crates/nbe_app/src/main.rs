//! The Neural Business Engine — renderer + navigation.
//!
//! Loads the graph from SQLite (`--db <path>`, default `brain.db`) and renders it as two distinct
//! **networks** floating far apart in space, like separate constellations:
//!   * **Business** — CRM clients + their financial ledger, one interconnected web. Per-client
//!     revenue (summed from connected ledger entries) and renewal dates surface in the sidebar.
//!   * **Research** — the knowledge base, off in the distance, growing its own web as notes link
//!     to topic hubs.
//! Nodes are glowing neurons; edges are hollow, translucent glass tubes with travelling sparks.
//! Camera: left-drag orbits, right/middle-drag pans, scroll flies forward/back, Esc unfocuses.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
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

    /// Base emissive hue (pre activation/intensity).
    fn base_color(self) -> (f32, f32, f32) {
        match self {
            Kind::Client => (0.12, 0.6, 1.0),     // cyan
            Kind::Ledger => (1.0, 0.6, 0.12),     // amber
            Kind::Knowledge => (0.2, 1.0, 0.45),  // green
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
        .insert_resource(ClearColor(Color::srgb(0.008, 0.015, 0.04)))
        .insert_resource(DbPath(db_path_from_args()))
        .insert_resource(CameraTarget::default())
        .insert_resource(NodeRegistry::default())
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
                update_hud,
                animate_breath,
                animate_sparks,
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

#[derive(Component)]
struct Breath {
    base: f32,
    phase: f32,
    speed: f32,
}

#[derive(Component)]
struct Spark {
    path: usize,
    t: f32,
    speed: f32,
    rng: u64,
}

#[derive(Resource, Default)]
struct EdgePaths(Vec<Vec<Vec3>>);

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
    let (hr, hg, hb) = (1.0, 0.72, 0.32); // warm amber-white hot
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

/// A translucent tube swept along `points` — the "clear glass filament" look from the references.
/// Builds a ring of `sides` vertices per path point and stitches consecutive rings into a sleeve.
fn tube_mesh(points: &[Vec3], radius: f32, sides: usize) -> Mesh {
    let usages = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    if points.len() < 2 {
        return Mesh::new(PrimitiveTopology::TriangleList, usages);
    }
    let rings = points.len();
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(rings * sides);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(rings * sides);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(rings * sides);
    let mut indices: Vec<u32> = Vec::with_capacity((rings - 1) * sides * 6);

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
        // A pair of axes perpendicular to the tangent to spin the ring around.
        let up = if t.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let n0 = t.cross(up).normalize();
        let b0 = t.cross(n0).normalize();
        for s in 0..sides {
            let a = s as f32 / sides as f32 * std::f32::consts::TAU;
            let dir = n0 * a.cos() + b0 * a.sin();
            positions.push((p + dir * radius).to_array());
            normals.push(dir.to_array());
            uvs.push([ri as f32 / (rings - 1) as f32, s as f32 / sides as f32]);
        }
    }
    for ri in 0..rings - 1 {
        for s in 0..sides {
            let s2 = (s + 1) % sides;
            let a = (ri * sides + s) as u32;
            let b = (ri * sides + s2) as u32;
            let c = ((ri + 1) * sides + s) as u32;
            let d = ((ri + 1) * sides + s2) as u32;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, usages)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
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
        Bloom::NATURAL,
        // A touch of cool ambient so the glass tubes aren't pure black where unlit.
        AmbientLight {
            color: Color::srgb(0.5, 0.65, 1.0),
            brightness: 60.0,
            ..default()
        },
        Transform::from_translation(cam.eye()).looking_at(cam.focus, Vec3::Y),
        cam,
    ));
}

// ---- scene build -----------------------------------------------------------------------

fn load_graph(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<NodeRegistry>,
    db_path: Res<DbPath>,
) {
    let (center, radius) = build_scene(
        &mut commands,
        meshes.as_mut(),
        materials.as_mut(),
        registry.as_mut(),
        &db_path.0,
    )
    .unwrap_or((Vec3::ZERO, 200.0));
    spawn_camera(&mut commands, center, radius);
}

/// Rebuild the graph from the DB when a button has changed it: despawn the old scene and re-create
/// it. The camera persists (untagged), so the view stays put while new neurons pop in.
fn apply_reload(
    mut control: ResMut<SceneControl>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    registry: &mut NodeRegistry,
    path: &str,
) -> Option<(Vec3, f32)> {
    registry.nodes.clear();
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
    for (&network, ids) in &groups {
        for id in ids {
            let kind = *kind_of.get(id).unwrap_or(&Kind::Knowledge);
            let a = *activation.get(id).unwrap_or(&0.0) as f32;
            let r = kind.base_size() + a * 0.45;
            let p = pos[id];
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.02, 0.06, 0.14),
                emissive: node_emissive(kind, a),
                perceptual_roughness: 0.4,
                ..default()
            });
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(p).with_scale(Vec3::splat(r)),
                Breath {
                    base: r,
                    phase: rand_unit(id, 9) * std::f32::consts::TAU,
                    speed: 0.6 + rand01(id, 10) * 0.8,
                },
                SceneItem,
            ));
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

    // Edges: hollow glass tubes (stronger ones only), reused by the sparks. Only ever drawn
    // between two nodes in the *same* network — no lines stretch across the void.
    // Clear glass: low roughness gives a sharp specular streak under the lights (the round-tube
    // cue), a faint blue tint + low alpha let you see through, and a gentle emissive keeps a glow.
    let edge_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.8, 1.0, 0.18),
        emissive: LinearRgba::rgb(0.12, 0.4, 0.8),
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
    let spawn_tube = |commands: &mut Commands,
                          meshes: &mut Assets<Mesh>,
                          edge_paths: &mut Vec<Vec<Vec3>>,
                          curve: Vec<Vec3>| {
        commands.spawn((
            Mesh3d(meshes.add(tube_mesh(&curve, 0.3, 8))),
            MeshMaterial3d(edge_mat.clone()),
            Transform::default(),
            SceneItem,
        ));
        edge_paths.push(curve);
    };

    let mut edge_paths: Vec<Vec<Vec3>> = Vec::new();
    for e in &snap.edges {
        if e.weight < EDGE_WEIGHT_MIN {
            continue;
        }
        let (Some(&a), Some(&b)) = (pos.get(&e.source_id), pos.get(&e.target_id)) else {
            continue;
        };
        let net_a = kind_of.get(&e.source_id).map(|k| k.network());
        let net_b = kind_of.get(&e.target_id).map(|k| k.network());
        if net_a != net_b {
            continue;
        }
        let curve = edge_curve(a, b, e.weight as f32, &curve_params, hash_u64(&e.id));
        spawn_tube(commands, meshes, &mut edge_paths, curve);
    }

    // The Research network's native links all point across to clients/ledger (dropped above), so
    // it arrives as loose dust. Weave a proximity web — each note to its 2 nearest neighbours — so
    // it reads as its own constellation. (Real Add-Research notes link to topic hubs over time.)
    if let Some(rids) = groups.get(&Network::Research) {
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
                spawn_tube(commands, meshes, &mut edge_paths, curve);
            }
        }
    }

    // Travelling light: a slow, soft pulse of light coursing down each pathway (not a hard dot).
    // Spawned as an elongated emissive glow stretched along the tube; bloom turns it into a streak.
    if !edge_paths.is_empty() {
        let pulse_mat = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(2.5, 4.5, 8.0),
            alpha_mode: AlphaMode::Add,
            ..default()
        });
        let pulse_count = (edge_paths.len() / 2).clamp(16, 220);
        let mut s = 0x9E37_79B9u64;
        for k in 0..pulse_count {
            let path = (lcg(&mut s) * edge_paths.len() as f32) as usize % edge_paths.len();
            let t = lcg(&mut s);
            let speed = 0.05 + lcg(&mut s) * 0.1; // slowish
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(pulse_mat.clone()),
                Transform::from_translation(sample_path(&edge_paths[path], t)),
                Spark {
                    path,
                    t,
                    speed,
                    rng: s ^ (k as u64 + 1),
                },
                SceneItem,
            ));
        }
    }
    commands.insert_resource(EdgePaths(edge_paths));

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

fn animate_sparks(
    time: Res<Time>,
    paths: Res<EdgePaths>,
    mut query: Query<(&mut Spark, &mut Transform)>,
) {
    if paths.0.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (mut spark, mut transform) in &mut query {
        spark.t += spark.speed * dt;
        if spark.t >= 1.0 {
            spark.t -= 1.0;
            spark.path = (lcg(&mut spark.rng) * paths.0.len() as f32) as usize % paths.0.len();
            spark.speed = 0.05 + lcg(&mut spark.rng) * 0.1;
        }
        let path = &paths.0[spark.path];
        let t = spark.t;
        let a = sample_path(path, (t - 0.015).max(0.0));
        let b = sample_path(path, (t + 0.015).min(1.0));
        transform.translation = sample_path(path, t);
        let dir = (b - a).normalize_or_zero();
        if dir.length_squared() > 1e-6 {
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
        }
        // Fade in/out toward the node ends so the pulse swells mid-pathway, like a light surge.
        let env = (t * std::f32::consts::PI).sin().max(0.0);
        let glow = 0.2 + 0.8 * env;
        transform.scale = Vec3::new(0.45 * glow, 0.45 * glow, 3.0 * glow);
    }
}

/// Two directional lights from opposite sides — they don't brighten the scene much (nodes are
/// emissive) but give the glass tubes a specular streak so they read as round 3D tubes.
fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.75, 0.85, 1.0),
            illuminance: 4500.0,
            ..default()
        },
        Transform::from_xyz(1.0, 1.2, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.8, 0.6),
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
