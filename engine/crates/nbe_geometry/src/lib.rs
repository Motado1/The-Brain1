//! `nbe_geometry` — deterministic organic geometry for the neuron/mycelial visual.
//!
//! Pure CPU math (only `glam`) that turns node positions and edges into the curves and
//! filaments the Bevy renderer tessellates on the workstation. Two families, matching the
//! agreed **hybrid** art direction:
//!
//! * [`curve`] — organic edge curves (subtle bow/sag/jitter) whose samples also serve as
//!   dotted particle-trail positions.
//! * [`tendril`] — fine, tapering, branching **mycelial** filaments, freestanding or sprouting
//!   along an edge.
//!
//! Everything is seed-deterministic, so visuals are stable run-to-run. `Vec3` is re-exported
//! from `glam` and is the same type Bevy uses.

pub mod curve;
mod rng;
pub mod tendril;

pub use glam::Vec3;

pub use curve::{edge_curve, CurveParams};
pub use tendril::{grow_tendrils, sprout_along, Tendril, TendrilParams};
