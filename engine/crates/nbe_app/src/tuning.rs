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
/// Max pulses one fire emits (caps hub blow-ups; kept low so the scene stays calm).
pub(crate) const MAX_PULSES_PER_FIRE: usize = 3;
/// Per-network dust mote count — dense, like the bokeh-filled reference images.
pub(crate) const MOTES_PER_NETWORK: usize = 360;
/// How many pulses may queue on a single busy connection before further ones are dropped — keeps
/// the flow deliberate without letting backlogs build.
pub(crate) const QUEUE_CAP: usize = 2;

// ---- soma membrane (Fresnel rim) shader — SHARED by the soma AND the tubes ---------------
// The connectors + dendrites (DendriteMaterial / pulse_wave.wgsl) wear the EXACT same glass cell-wall
// as the soma so they look identical to the cell body: a clear see-through centre with a glowing
// Fresnel rim, AlphaMode::Blend. These three constants drive both.
/// Rim sharpness (higher = thinner cell-wall line, clearer see-through centre).
pub(crate) const RIM_POWER: f32 = 2.5;
/// Rim glow brightness.
pub(crate) const RIM_INTENSITY: f32 = 1.6;
/// Rim opacity (centre stays clear). Lower = more translucent (more see-through), so a tube reads as
/// glass rather than a solid band up close — drop this for the tubes if they still look too solid.
pub(crate) const RIM_ALPHA: f32 = 0.68;

// ---- volumetric tube material (soma-glass membrane + pulse fill — pulse_wave.wgsl / DendriteMaterial)
// The tubes use the soma's Fresnel formula but with their OWN rim intensity/alpha: a thin tube is
// "all rim" (no broad camera-facing face like the big soma sphere), so at the soma's full RIM_INTENSITY
// it reads as a bright solid bar. Dimmed + more transparent here so a thin tube reads as a soft
// translucent glass line — the soma's *surface* look at a tube's apparent brightness. (RIM_POWER is
// still shared.) Raise toward the soma's RIM_INTENSITY(1.6)/RIM_ALPHA(0.68) for brighter/solider tubes.
/// Tube rim glow brightness (vs the soma's RIM_INTENSITY 1.6).
pub(crate) const TUBE_RIM_INTENSITY: f32 = 0.9;
/// Tube rim opacity (vs the soma's RIM_ALPHA 0.68) — lower = more see-through glass.
pub(crate) const TUBE_RIM_ALPHA: f32 = 0.45;
/// Emissive multiplier applied where a pulse is active — massive (HDR) so the flooded segment blooms.
/// Drop if the travelling crest blows out to white.
pub(crate) const DEND_PULSE_EMISSIVE: f32 = 5.0;

// ---- granular soma body (Phase-1 geometry: textured mass + root junctions) --------------
/// Icosphere subdivision level for the soma mesh (higher = finer, more triangles). 3 ≈ 642 verts —
/// enough resolution for the noise bumps to read without exploding draw cost.
pub(crate) const SOMA_SUBDIV: u8 = 3;
/// Bump depth as a fraction of radius — how lumpy/granular the silhouette is. Pushed up so the cell
/// body reads as a textured granular mass (reference look), not a smooth ball.
pub(crate) const SOMA_BUMP: f32 = 0.28;
/// Soma body radius between connections (fraction of r) — shrunk below 1 so the connection bulges
/// stand out as star-shaped points (the membrane flowing out into each process).
pub(crate) const SOMA_BASE: f32 = 0.8;
/// How far the membrane bulges outward toward each connection (fraction of r) — the process length.
pub(crate) const SOMA_PROCESS_REACH: f32 = 0.32;
/// Sharpness of each connection lobe (cosine power, >1 = tighter/pointier process toward the link).
pub(crate) const SOMA_PROCESS_TIGHTNESS: f32 = 3.0;
/// Connector funnel mouth: the wide radius (fraction of soma radius) where the tube fuses into the
/// cell body — the membrane flowing out into the tunnel. With ROOT_EMBED ~0.92 this mouth lands on
/// the membrane surface, so it reads as continuous rather than a pipe poking into a sphere.
pub(crate) const ROOT_FLARE: f32 = 0.03;
/// Slender connector body radius (absolute) between the two funnels — the tunnel itself. Kept very
/// thin so the additive strand reads as a glowing *line* (thin twigs already read right today).
pub(crate) const CONN_BODY: f32 = 0.025;
/// Fraction of a connector's length each end funnel occupies (the rest is the slender tunnel body).
pub(crate) const ROOT_FLARE_ZONE: f32 = 0.15;
/// Concavity of the funnel neck (>1 = fat only right at the membrane, necking fast to the body).
pub(crate) const ROOT_FLARE_POW: f32 = 2.6;
/// Where a connector roots, as a fraction of the soma radius (1.0 = the membrane surface). Kept near
/// the surface so connectors attach to the *clear membrane shell* and flow out from there — not dive
/// down into the glowing core. Slightly under 1.0 so the flared base fuses into the shell, no gap.
pub(crate) const ROOT_EMBED: f32 = 0.92;

// ---- branching dendrites (fractal tree, reference-neuron structure) ---------------------
/// Where a dendrite trunk begins, as a fraction of the soma radius — matched to SOMA_BASE so the
/// trunk roots into the membrane shell between connection bulges (not deep at the core, not floating).
pub(crate) const DEND_EMBED: f32 = 0.8;
/// Trunk *base* width as a fraction of the soma radius — where the dendrite tree leaves the soma,
/// then a concave taper (DEND_ROOT_TAPER_POW) necks it down. Kept THIN (filament era): a fat trunk is
/// mostly camera-facing surface, so the additive core lights its whole width and reads as a solid
/// shaded band up close (the old glass-tube fat sizing). Thin trunks read as glowing lines like the
/// twigs. Raise if the close-up trunks look too spindly; lower if they read as solid tubes.
pub(crate) const DEND_ROOT_R: f32 = 0.05;
/// Concavity of the trunk's base→tip taper (>1 = stays thin along its length but flares sharply at
/// the soma, the tree-branch fillet). 1.0 would be a plain cone.
pub(crate) const DEND_ROOT_TAPER_POW: f32 = 2.6;
/// Surface-noise depth on the dendrite membrane, as a fraction of the local tube radius (the bumpy
/// organic skin, continuous with the soma's SOMA_BUMP). Proportional so thin twigs aren't shredded.
pub(crate) const DEND_BUMP_REL: f32 = 0.35;
/// Spatial frequency of that membrane bump noise (smaller = larger, soma-like lumps).
pub(crate) const DEND_BUMP_FREQ: f32 = 0.6;
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
