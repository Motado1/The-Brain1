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
    pub(crate) base: f32,
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
    pub(crate) path: Vec<Vec3>,
    pub(crate) target: usize,
}

/// Shared mesh/material for spawning propagation pulses at runtime.
#[derive(Resource)]
pub(crate) struct PulseAssets {
    pub(crate) mesh: Handle<Mesh>,
    pub(crate) material: Handle<StandardMaterial>,
}

/// Tags every entity the scene builder spawns (nodes, edges, sparks) so a rebuild can despawn the
/// whole graph and re-create it from the DB — the camera/HUD (untagged) persist.
#[derive(Component)]
pub(crate) struct SceneItem;

/// Cross-system signal + status line for button actions that change the DB and need a redraw.
#[derive(Resource, Default)]
pub(crate) struct SceneControl {
    pub(crate) reload: bool,
    pub(crate) status: String,
}
