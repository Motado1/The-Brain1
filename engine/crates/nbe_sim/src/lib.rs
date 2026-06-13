//! `nbe_sim` — activation rules and action-potential propagation (the simulation brain).
//!
//! Two pure, deterministic pieces, both verifiable without a GPU:
//!
//! * [`activation`] maps a neuron's business facets (CRM renewal, ledger urgency, recent
//!   knowledge recall) to an activation value in `[0, 1]` — the renderer turns this into node
//!   scale + glow.
//! * [`pulse`] advances "action potentials" along edges, reports arrivals, and propagates an
//!   activation bump to the destination — the renderer draws these as travelling sparks.

pub mod activation;
pub mod pulse;

pub use activation::{activation_value, ActivationInputs};
pub use pulse::{advance_pulses, apply_arrivals, decay, fire, ready_to_fire, Pulse, SimEdge};
