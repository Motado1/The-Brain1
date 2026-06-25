use bevy::prelude::*;

use crate::domain::*;

// ---- navigation registry + camera target ----------------------------------------------

pub(crate) struct NodeInfo {
    /// Full DB entity id (for detail lookup via `ops::show`).
    pub(crate) id: String,
    /// The neuron's body entity (for hover/selection highlight).
    pub(crate) entity: Entity,
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) network: Network,
    pub(crate) pos: Vec3,
    /// Summed income from connected ledger entries (clients only).
    pub(crate) revenue_cents: Option<i64>,
    /// Client renewal date (unix seconds).
    pub(crate) renewal: Option<i64>,
    /// The node's five profile "planet" aspects (same data woven onto its dendrite tips), surfaced in
    /// the detail panel so the planets are readable as text too. Empty for nodes without anatomy.
    pub(crate) aspects: Vec<crate::anatomy::Aspect>,
}

/// Cursor-pick state: which node the pointer is over, which is selected, and the cached detail text
/// for the selected node. Indices are into `NodeRegistry::nodes`.
#[derive(Resource, Default)]
pub(crate) struct Picker {
    pub(crate) hovered: Option<usize>,
    pub(crate) selected: Option<usize>,
    /// Entities mirroring the indices above, for cheap highlight comparison in the render system.
    pub(crate) hovered_entity: Option<Entity>,
    pub(crate) selected_entity: Option<Entity>,
    /// Cursor position when the left button went down — to tell a click from an orbit-drag.
    pub(crate) press: Option<Vec2>,
    /// Cached `ops::show` output for the selected node.
    pub(crate) detail: String,
}

#[derive(Resource, Default)]
pub(crate) struct NodeRegistry {
    pub(crate) nodes: Vec<NodeInfo>,
    pub(crate) galaxy_center: Vec3,
    pub(crate) galaxy_radius: f32,
    pub(crate) total_revenue_cents: i64,
}

impl NodeRegistry {
    /// Focus + radius to frame a whole network.
    pub(crate) fn network_view(&self, network: Network) -> (Vec3, f32) {
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

/// When set, the camera smoothly flies to this (focus, radius). Used by the *zoom* path, which
/// re-targets continuously each scroll frame (exponential approach). Discrete, cinematic fly-tos
/// (sidebar buttons, Esc unfocus, omni-search) go through `CameraGlide` instead.
#[derive(Resource, Default)]
pub(crate) struct CameraTarget(pub(crate) Option<(Vec3, f32)>);

/// A fixed-duration cinematic camera glide with a cubic ease-in/out, distinct from the continuous
/// zoom lerp. Set `request` to a destination `(focus, radius)`; `orbit_camera` captures the current
/// camera as the start and eases over `GLIDE_SECS`. Manual drag/scroll cancels it.
#[derive(Resource, Default)]
pub(crate) struct CameraGlide {
    /// A newly requested destination; consumed next frame to begin a flight.
    pub(crate) request: Option<(Vec3, f32)>,
    /// The in-progress flight, if any.
    pub(crate) flight: Option<Flight>,
}

pub(crate) struct Flight {
    pub(crate) start_focus: Vec3,
    pub(crate) start_radius: f32,
    pub(crate) end_focus: Vec3,
    pub(crate) end_radius: f32,
    /// Normalised progress 0→1; scaled by `dur` seconds.
    pub(crate) t: f32,
    pub(crate) dur: f32,
}

impl CameraGlide {
    /// Request a cinematic glide to frame a node at `focus` from `radius` away.
    pub(crate) fn to(&mut self, focus: Vec3, radius: f32) {
        self.request = Some((focus, radius));
    }
}

/// Cubic ease-in/out on a clamped 0..1 input: starts and ends at zero velocity for a smooth,
/// cinematic acceleration/deceleration (the "fly across the void" feel). `f(0)=0`, `f(1)=1`,
/// `f(0.5)=0.5`, strictly increasing.
pub(crate) fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Omni-search command-overlay state (Ctrl+P / Cmd+K). `open` toggles the centred floating bar;
/// `query` is the live text; `selected` is the highlighted result row (arrow-key navigable).
#[derive(Resource, Default)]
pub(crate) struct OmniSearch {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) selected: usize,
}

/// True while the mouse pointer is over an egui panel — suppresses camera input so scrolling a
/// sidebar list doesn't also zoom the 3D view.
#[derive(Resource, Default)]
pub(crate) struct UiPointer {
    pub(crate) over: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_endpoints_midpoint_and_monotonic() {
        assert!(ease_in_out_cubic(0.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
        // Clamps out-of-range inputs.
        assert!(ease_in_out_cubic(-1.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(2.0) - 1.0).abs() < 1e-6);
        // Strictly increasing across the curve.
        let mut prev = -1.0;
        for i in 0..=20 {
            let v = ease_in_out_cubic(i as f32 / 20.0);
            assert!(v > prev, "not monotonic at {i}: {v} <= {prev}");
            prev = v;
        }
    }
}
