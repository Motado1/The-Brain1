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
/// Continuous pulse wave: fraction of a connection's length the energy crest travels per second.
pub(crate) const PULSE_SPEED: f32 = 0.3;
/// Gaussian half-width (sigma) of the wave as a fraction of the path — a small, tight crest.
pub(crate) const PULSE_WIDTH: f32 = 0.07;
/// Emissive amplitude at the wave crest (HDR, so bloom blurs the surge into a bleeding wave).
pub(crate) const PULSE_WAVE_AMP: f32 = 2.6;
/// Rest a connection takes after absorbing a pulse before it may carry the next one (seconds). The
/// pause between a signal arriving and the reply heading back.
pub(crate) const CHANNEL_COOLDOWN: f32 = 2.0;
/// After a connection pulse reaches the target node, the crest stops advancing and its glow eases
/// out over this many seconds (smoothstep) — so the light flows *into* the node instead of snapping
/// off. Kept well under `CHANNEL_COOLDOWN` so the connection is still resting while it fades.
pub(crate) const PULSE_FADE_TIME: f32 = 0.5;
/// Dendrite surge speed: when a soma fires, the crest travels out through its tree at this rate
/// (uv/sec, uv normalised root→tip). Fast — the tree is short, so the surge feels snappy.
pub(crate) const DEND_WAVE_SPEED: f32 = 1.2;
/// Emissive amplitude of the dendrite surge crest. Kept below PULSE_WAVE_AMP so the hair-thin twigs
/// don't blow out into bloom.
pub(crate) const DEND_WAVE_AMP: f32 = 1.8;
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
/// Rim opacity (centre stays clear). Lower = more translucent membrane, so the glowing core reads
/// *through* the cell body (the reference look) instead of a flat opaque shell.
pub(crate) const RIM_ALPHA: f32 = 0.68;

// Dendrites share the soma's Fresnel "cell-wall" shader (same translucent, rim-lit membrane look),
// but a touch softer — a thin branch is almost all grazing-angle surface, so a sharper rim + lower
// intensity keeps the dense tree glowing gently instead of blowing out into solid bright threads.
// Connectors + dendrites now wear the EXACT same clear Fresnel membrane as the soma (RIM_POWER /
// RIM_INTENSITY / RIM_ALPHA above) — no separate rim params, no sheen — so they read as the same
// translucent cell-wall, just brightening where a pulse flows through (pulse_wave.wgsl).

// ---- granular soma body (Phase-1 geometry: textured mass + root junctions) --------------
/// Icosphere subdivision level for the soma mesh (higher = finer, more triangles). 3 ≈ 642 verts —
/// enough resolution for the noise bumps to read without exploding draw cost.
pub(crate) const SOMA_SUBDIV: u8 = 3;
/// Bump depth as a fraction of radius — how lumpy/granular the silhouette is. Pushed up so the cell
/// body reads as a textured granular mass (reference look), not a smooth ball.
pub(crate) const SOMA_BUMP: f32 = 0.28;
/// Number of distinct displaced soma meshes in the shared pool (picked per-node by hash) so the
/// cells vary without a unique mesh per neuron.
pub(crate) const SOMA_VARIANTS: usize = 6;
/// Connector funnel mouth: the wide radius (fraction of soma radius) where the tube fuses into the
/// cell body — the membrane flowing out into the tunnel. With ROOT_EMBED ~0.92 this mouth lands on
/// the membrane surface, so it reads as continuous rather than a pipe poking into a sphere.
pub(crate) const ROOT_FLARE: f32 = 0.24;
/// Slender connector body radius (absolute) between the two funnels — the tunnel itself.
pub(crate) const CONN_BODY: f32 = 0.07;
/// Fraction of a connector's length each end funnel occupies (the rest is the slender tunnel body).
pub(crate) const ROOT_FLARE_ZONE: f32 = 0.15;
/// Concavity of the funnel neck (>1 = fat only right at the membrane, necking fast to the body).
pub(crate) const ROOT_FLARE_POW: f32 = 2.6;
/// Where a connector roots, as a fraction of the soma radius (1.0 = the membrane surface). Kept near
/// the surface so connectors attach to the *clear membrane shell* and flow out from there — not dive
/// down into the glowing core. Slightly under 1.0 so the flared base fuses into the shell, no gap.
pub(crate) const ROOT_EMBED: f32 = 0.92;
/// Billboard scale of the additive glow dot placed at each filament→soma junction (Target 3 —
/// light compounds where roots meet the surface). Slightly larger than the mid-filament beads.
pub(crate) const JUNCTION_GLOW: f32 = 0.85;

// ---- branching dendrites (fractal tree, reference-neuron structure) ---------------------
/// Where a dendrite trunk begins, as a fraction of the soma radius. Near the membrane shell (like
/// connectors' ROOT_EMBED) so dendrites attach to the *clear membrane* and flow out from it — not
/// start deep at the glowing core. Slightly under 1.0 so the flared base fuses into the shell.
pub(crate) const DEND_EMBED: f32 = 0.9;
/// Trunk *base* width as a fraction of the soma radius — wide where it fuses with the soma, then a
/// concave taper (DEND_ROOT_TAPER_POW) necks it down fast so only the fillet at the base is fat.
pub(crate) const DEND_ROOT_R: f32 = 0.24;
/// Concavity of the trunk's base→tip taper (>1 = stays thin along its length but flares sharply at
/// the soma, the tree-branch fillet). 1.0 would be a plain cone.
pub(crate) const DEND_ROOT_TAPER_POW: f32 = 2.6;
/// How many times a dendrite splits into finer children (recursion depth) — the fractal tree.
pub(crate) const DEND_BRANCH_DEPTH: u32 = 3;
/// Each branch ends at this fraction of its start width; its children continue from there, so the
/// tree is continuous thick-trunk → hair-thin twigs.
pub(crate) const DEND_BRANCH_TAPER: f32 = 0.55;

// ---- navigation (cinematic camera glide + omni-search fly-to) ---------------------------
/// Duration of a cinematic camera glide (sidebar / search / Esc fly-to), seconds. Cubic ease-in/out.
pub(crate) const GLIDE_SECS: f32 = 1.1;
/// Orbit radius the omni-search lands at when it flies you to a node.
pub(crate) const OMNI_FLY_RADIUS: f32 = 20.0;

// ---- level-of-detail (macro→micro cosmic scaling) ---------------------------------------
// Anchored on the camera: detail_for_distance() maps these to a 0..1 cubic-smoothed detail value.
// Tune from a screenshot — they depend on the scene's overall scale (network radii ~ cbrt(n)*32).
/// Camera distance at/under which we're in deep **Micro** focus (finest twigs + terminal data full).
pub(crate) const LOD_MICRO_DIST: f32 = 60.0;
/// Camera distance at/beyond which we're in **Galactic** view (somas + primary trunks only).
pub(crate) const LOD_GALACTIC_DIST: f32 = 900.0;
