// ---- "alive" layer tuning (iterate on these from a screenshot) -------------------------
/// Baseline charge every neuron gets regardless of need — keeps the whole brain quietly alive even
/// where nothing is urgent (threshold ~0.5, so ~0.018/s ≈ a fire every ~28s at rest: calm).
pub(crate) const FIRE_BASE: f32 = 0.018;
/// Extra charge scaled by a neuron's activation (real need) — urgent neurons fire more often
/// (activation ~1.0 → ~0.16/s ≈ a fire every ~3s).
pub(crate) const FIRE_NEED: f32 = 0.14;
/// Flare brightness decay rate (higher = snappier flash).
pub(crate) const FIRE_DECAY: f32 = 2.2;
/// Peak emissive multiplier at full flare (kept modest so a fire is a gentle pulse, not a blast).
pub(crate) const FLARE_GAIN: f32 = 2.2;
/// How much a fired neuron's halo swells at peak.
pub(crate) const HALO_SWELL: f32 = 0.4;
/// Energy a propagation pulse deposits in its target (threshold ~0.5, so it takes coincidence to
/// re-fire — cascades stay sparse and fade).
pub(crate) const PULSE_ENERGY: f32 = 0.16;
/// Max pulses one fire emits (caps hub blow-ups; kept low so the scene stays calm).
pub(crate) const MAX_PULSES_PER_FIRE: usize = 3;
/// Per-network dust mote count — dense, like the bokeh-filled reference images.
pub(crate) const MOTES_PER_NETWORK: usize = 360;
/// How many pulses may queue on a single busy connection before further ones are dropped — keeps
/// the flow deliberate without letting backlogs build.
pub(crate) const QUEUE_CAP: usize = 2;

// ---- soma membrane (Fresnel rim) shader -------------------------------------------------
/// Rim sharpness (higher = thinner cell-wall line).
pub(crate) const RIM_POWER: f32 = 2.5;
/// Rim glow brightness.
pub(crate) const RIM_INTENSITY: f32 = 1.6;
/// Rim opacity (centre stays clear).
pub(crate) const RIM_ALPHA: f32 = 0.85;

// ---- granular soma body (Phase-1 geometry: textured mass + root junctions) --------------
/// Icosphere subdivision level for the soma mesh (higher = finer, more triangles). 3 ≈ 642 verts —
/// enough resolution for the noise bumps to read without exploding draw cost.
pub(crate) const SOMA_SUBDIV: u8 = 3;
/// Bump depth as a fraction of radius — how lumpy/granular the silhouette is.
pub(crate) const SOMA_BUMP: f32 = 0.18;
/// Number of distinct displaced soma meshes in the shared pool (picked per-node by hash) so the
/// cells vary without a unique mesh per neuron.
pub(crate) const SOMA_VARIANTS: usize = 6;
/// Root flare: each filament end widens to this fraction of the soma radius where it meets the body
/// (Target 2 — "tree root gripping the soil"). Was 0.16 when the filament merely kissed the surface.
pub(crate) const ROOT_FLARE: f32 = 0.42;
/// How far inside the soma surface a filament starts, as a fraction of radius, so its flare embeds
/// into the body (fuses) instead of floating against it. 1.0 = exactly on the surface.
pub(crate) const ROOT_EMBED: f32 = 0.82;
/// Billboard scale of the additive glow dot placed at each filament→soma junction (Target 3 —
/// light compounds where roots meet the surface). Slightly larger than the mid-filament beads.
pub(crate) const JUNCTION_GLOW: f32 = 0.85;

// ---- glowing filament/dendrite tubes (FilamentMaterial: Fresnel + gradient + flow) ------
// The SOMA is the light source. A connection should glow only where it touches a soma and fade to
// near-dark along its length — so light reads as coming *from* the cell body, not the wire. Hence
// low base intensity, a sharp rim (thick tube faces stay clear/glassy), and a steep end-biased
// gradient with a very dark middle.
/// Rim sharpness for the tubes. High so a thick tube's camera-facing surface stays clear and only
/// the grazing silhouette glows (glassy), instead of the whole pipe reading as a solid bright tube.
pub(crate) const FIL_RIM_POWER: f32 = 2.4;
/// Base glow brightness of a strand — kept well below the soma so the cell body dominates.
pub(crate) const FIL_INTENSITY: f32 = 0.55;
/// Speed the light bands travel along a strand (uv lengths per second).
pub(crate) const FIL_FLOW_SPEED: f32 = 0.10;
/// How pronounced the flowing bands are (0 = steady glow, 1 = strong pulsing). Subtle so light
/// doesn't "suddenly appear" — a gentle travelling shimmer, not a strobe.
pub(crate) const FIL_FLOW_STRENGTH: f32 = 0.15;
/// Length-gradient power (higher = glow hugs the soma ends more tightly, darker middle).
pub(crate) const FIL_GRAD_POWER: f32 = 2.8;
/// Connection (soma↔soma) glow: bright at both root ends, near-dark mid — both ends are cell bodies
/// and the light spills from them onto the connection base, fading out between.
pub(crate) const EDGE_GLOW_END: f32 = 1.0;
pub(crate) const EDGE_GLOW_MID: f32 = 0.05;
/// Dendrite glow: bright at the root (uv.x=0, on the soma), fading to a near-invisible tip (uv.x=1).
pub(crate) const DEND_GLOW_ROOT: f32 = 0.9;
pub(crate) const DEND_GLOW_TIP: f32 = 0.02;
/// Organic radius wobble on the tubes (fraction of radius) so strands aren't perfectly smooth pipes.
pub(crate) const TUBE_WOBBLE: f32 = 0.22;
