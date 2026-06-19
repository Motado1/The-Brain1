use std::collections::VecDeque;

use bevy::prelude::*;

#[derive(Resource)]
pub(crate) struct DbPath(pub(crate) String);

// ---- components / helpers --------------------------------------------------------------

#[derive(Component)]
pub(crate) struct OrbitCamera {
    pub(crate) focus: Vec3,
    pub(crate) radius: f32,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

impl OrbitCamera {
    pub(crate) fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.focus + Vec3::new(self.radius * cp * sy, self.radius * sp, self.radius * cp * cy)
    }
}

#[derive(Component)]
pub(crate) struct HudText;

/// A camera-facing quad — the soft glow halo behind a neuron.
#[derive(Component)]
pub(crate) struct Billboard;

#[derive(Component)]
pub(crate) struct Breath {
    /// Per-axis base scale (non-uniform → slightly misshapen, organic cell bodies, not round stars).
    pub(crate) base: Vec3,
    pub(crate) phase: f32,
    pub(crate) speed: f32,
}

/// Travelling pulse of light along an edge — spawned when a neuron fires and propagates to its
/// neighbour. Replaces the old random-looping spark.
#[derive(Component)]
pub(crate) struct Pulse {
    pub(crate) edge: usize,
    pub(crate) t: f32,
    pub(crate) speed: f32,
    pub(crate) target: usize,
    pub(crate) energy: f32,
    /// Set once the crest reaches the target node — energy is deposited and the channel rests, but
    /// the pulse lingers (no longer advancing) while `fade` ramps its glow down, so the light eases
    /// *into* the node instead of snapping off. Despawns when `fade` completes.
    pub(crate) arrived: bool,
    pub(crate) fade: f32,
}

/// A drifting dust mote, for ambient depth (the bokeh specks in the references).
#[derive(Component)]
pub(crate) struct Mote {
    pub(crate) vel: Vec3,
    pub(crate) center: Vec3,
    pub(crate) radius: f32,
}

/// Links a spawned neuron entity back to its slot in the `BrainGraph`.
#[derive(Component)]
pub(crate) struct Neuron(pub(crate) usize);

/// Integrate-and-fire state for a neuron.
#[derive(Component)]
pub(crate) struct Firing {
    pub(crate) accumulator: f32,
    pub(crate) intensity: f32,
}

/// Cached base look + the handles a firing flare needs to drive.
#[derive(Component)]
pub(crate) struct NodeViz {
    pub(crate) base_emissive: LinearRgba,
    pub(crate) base_radius: f32,
    pub(crate) halo: Entity,
    pub(crate) mat: Handle<StandardMaterial>,
    pub(crate) phase: f32,
    pub(crate) twinkle: f32,
}

/// The addressable graph the animation systems read: who connects to whom, along which path.
#[derive(Resource, Default)]
pub(crate) struct BrainGraph {
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) edges: Vec<GraphEdge>,
}

pub(crate) struct GraphNode {
    pub(crate) entity: Entity,
    pub(crate) activation: f32,
    pub(crate) threshold: f32,
    pub(crate) out: Vec<usize>, // edge indices leaving this node
}

pub(crate) struct GraphEdge {
    pub(crate) target: usize,
    /// The undirected connection this directed edge belongs to (both directions share one channel,
    /// so locking it enforces the directional mutex).
    pub(crate) channel: usize,
}

/// Traffic state for the network's connections — one `Channel` per undirected connection. A channel
/// hosts at most one pulse at a time (single occupancy); further requests queue (capped). Because
/// both directions of a connection share its channel, A→B in flight blocks B→A (directional mutex).
#[derive(Resource, Default)]
pub(crate) struct EdgeTraffic {
    pub(crate) channels: Vec<Channel>,
}

#[derive(Default)]
pub(crate) struct Channel {
    pub(crate) busy: bool,
    pub(crate) queue: VecDeque<usize>, // directed-edge indices waiting to traverse
    /// Seconds the connection must rest after a pulse is absorbed before it can carry another — so a
    /// signal is fully received and there's a beat before the reply travels back (no ping-pong).
    pub(crate) cooldown: f32,
}

/// Links a connection's tube to its traffic `channel`, its forward directed-edge index (so a pulse's
/// `t` maps to uv direction correctly), and its `DendriteMaterial` so the wave system can drive the
/// travelling Gaussian surge along it.
#[derive(Component)]
pub(crate) struct ConnectionWave {
    pub(crate) channel: usize,
    pub(crate) fwd_edge: usize,
    pub(crate) mat: Handle<crate::shaders::DendriteMaterial>,
}

/// Channels with an active wave last frame, so the wave system only resets ones that just went idle
/// (rather than re-uploading every connection material every frame).
#[derive(Resource, Default)]
pub(crate) struct WaveActive(pub(crate) std::collections::HashSet<usize>);

/// A soma's dendrite tree, driven so a firing neuron's light surges outward through its branches.
/// `t` is the wave centre (uv along the tree, 0 = root); it restarts at 0 when the `soma` fires and
/// advances out past the tips, then idles. `prev_intensity` detects that firing rising edge.
#[derive(Component)]
pub(crate) struct DendriteWave {
    pub(crate) soma: Entity,
    pub(crate) mat: Handle<crate::shaders::DendriteMaterial>,
    pub(crate) t: f32,
    pub(crate) prev_intensity: f32,
}

/// A data-anatomy element (session bead, package/research twig) that only materialises at close
/// range: its rendered scale ramps `0 → base_scale` as the global LOD zoom-detail crosses
/// `[start, full]` (cubic-smoothed), so the finest data nodes fade in on deep zoom instead of
/// cluttering the galactic view. Driven by `lod::apply_lod_reveal`.
#[derive(Component)]
pub(crate) struct LodReveal {
    pub(crate) base_scale: f32,
    pub(crate) start: f32,
    pub(crate) full: f32,
}

/// Tags every entity the scene builder spawns (nodes, edges, sparks) so a rebuild can despawn the
/// whole graph and re-create it from the DB — the camera/HUD (untagged) persist.
#[derive(Component)]
pub(crate) struct SceneItem;

/// Marks the procedural soma membrane mesh. If a Blender-authored `assets/soma.glb` is present, the
/// `apply_soma_gltf` system swaps these for the imported model; otherwise the procedural soma stays.
#[derive(Component)]
pub(crate) struct ProceduralSoma;

/// Marks an imported glTF soma body (so tools/reloads can find them).
#[derive(Component)]
pub(crate) struct SomaBody;

/// Handle to the optional Blender soma model (`assets/soma.glb`, first scene). `None` until the
/// startup loader sets it; the swap only fires once the asset is fully loaded, so a missing file
/// simply leaves the procedural soma in place.
#[derive(Resource, Default)]
pub(crate) struct SomaGltf {
    pub(crate) scene: Option<Handle<Scene>>,
}

/// Cross-system signal + status line for button actions that change the DB and need a redraw.
#[derive(Resource, Default)]
pub(crate) struct SceneControl {
    pub(crate) reload: bool,
    pub(crate) status: String,
}
