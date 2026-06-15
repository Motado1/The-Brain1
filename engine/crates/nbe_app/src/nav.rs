use bevy::prelude::*;

use crate::domain::*;

// ---- navigation registry + camera target ----------------------------------------------

pub(crate) struct NodeInfo {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) network: Network,
    pub(crate) pos: Vec3,
    /// Summed income from connected ledger entries (clients only).
    pub(crate) revenue_cents: Option<i64>,
    /// Client renewal date (unix seconds).
    pub(crate) renewal: Option<i64>,
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

/// When set, the camera smoothly flies to this (focus, radius).
#[derive(Resource, Default)]
pub(crate) struct CameraTarget(pub(crate) Option<(Vec3, f32)>);

/// True while the mouse pointer is over an egui panel — suppresses camera input so scrolling a
/// sidebar list doesn't also zoom the 3D view.
#[derive(Resource, Default)]
pub(crate) struct UiPointer {
    pub(crate) over: bool,
}
