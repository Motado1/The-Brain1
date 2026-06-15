// ---- "alive" layer tuning (iterate on these from a screenshot) -------------------------
/// Baseline charge every neuron gets regardless of need — keeps the whole brain alive and firing
/// even where nothing is urgent (threshold ~0.5, so ~0.04/s ≈ a fire every ~12s at rest).
pub(crate) const FIRE_BASE: f32 = 0.04;
/// Extra charge scaled by a neuron's activation (real need) — urgent neurons fire much more often
/// (activation ~1.0 → ~0.3/s ≈ a fire every ~1.5s).
pub(crate) const FIRE_NEED: f32 = 0.28;
/// Flare brightness decay rate (higher = snappier flash).
pub(crate) const FIRE_DECAY: f32 = 3.0;
/// Peak emissive multiplier at full flare.
pub(crate) const FLARE_GAIN: f32 = 5.0;
/// How much a fired neuron's halo swells at peak.
pub(crate) const HALO_SWELL: f32 = 0.8;
/// Energy a propagation pulse deposits in its target (threshold ~0.5, so it takes coincidence to
/// re-fire — cascades stay sparse and fade).
pub(crate) const PULSE_ENERGY: f32 = 0.22;
/// Max pulses one fire emits (caps hub blow-ups).
pub(crate) const MAX_PULSES_PER_FIRE: usize = 5;
/// Per-network dust mote count.
pub(crate) const MOTES_PER_NETWORK: usize = 70;
