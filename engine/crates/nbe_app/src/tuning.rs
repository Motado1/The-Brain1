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

// ---- tube membrane (Fresnel rim) shader — connectors + dendrites -------------------------
// The connectors + dendrites (DendriteMaterial / pulse_wave.wgsl) wear a glass cell-wall matching the
// imported soma GLBs: a clear see-through centre with a glowing Fresnel rim, AlphaMode::Blend.
/// Rim sharpness (higher = thinner cell-wall line, clearer see-through centre). Shared by all tubes.
pub(crate) const RIM_POWER: f32 = 2.5;

// ---- volumetric tube material (glass membrane + pulse fill — pulse_wave.wgsl / DendriteMaterial) ---
// A thin tube is "all rim" (no broad camera-facing face), so it's dimmed + more transparent than a big
// surface would be, to read as a soft translucent glass line. (RIM_POWER is shared.)
/// Tube rim glow brightness.
pub(crate) const TUBE_RIM_INTENSITY: f32 = 0.9;
/// Tube rim opacity — lower = more see-through glass.
pub(crate) const TUBE_RIM_ALPHA: f32 = 0.45;
/// Emissive multiplier applied where a pulse is active — massive (HDR) so the flooded segment blooms.
/// Drop if the travelling crest blows out to white.
pub(crate) const DEND_PULSE_EMISSIVE: f32 = 5.0;
/// Faint colorless (white) Fresnel-rim level shown on an idle tube so the clear-glass wiring is still
/// visible when far from any soma and no pulse is passing. Raise for more visible idle tubes, drop
/// toward 0 to make the wiring nearly invisible until lit.
pub(crate) const TUBE_GLASS_RIM: f32 = 0.12;
/// How far the soma's light bleeds along a tube from its soma end (fraction of the tube's length, u).
/// The cell body's hue fills the tube up to here, then fades to colorless glass.
pub(crate) const SOMA_BLEED_FALLOFF: f32 = 0.4;

// ---- soma membrane skin (engine override of the imported GLB material) -------------------
// The imported GLB ships opaque-white materials; `apply_soma_skin` re-skins each soma mesh in-engine
// with the same glass Fresnel membrane the tubes wear, tinted per network, so the body reads as
// glowing bioluminescent glass with the nucleus shining through — and CRM/Research stay colour-themed.
/// Soma rim glow brightness.
pub(crate) const SOMA_RIM_INTENSITY: f32 = 1.5;
/// Soma rim opacity — low so the membrane stays clearly translucent (nucleus glows through the centre).
pub(crate) const SOMA_RIM_ALPHA: f32 = 0.42;
/// Connector funnel mouth: the wide radius (fraction of soma radius) where the tube fuses into the
/// cell body — the membrane flowing out into the tunnel. With ROOT_EMBED ~0.92 this mouth lands on
/// the membrane surface, so it reads as continuous rather than a pipe poking into a sphere.
pub(crate) const ROOT_FLARE: f32 = 0.022;
/// Slender connector body radius (absolute) between the two funnels — the tunnel itself. Kept very
/// thin so the additive strand reads as a glowing *line* (thin twigs already read right today).
pub(crate) const CONN_BODY: f32 = 0.018;
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
pub(crate) const DEND_ROOT_R: f32 = 0.028;
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

// Fine-dendrite reveal window on the global zoom detail (0 = galactic, 1 = micro): the finer branches
// (depth ≥ 1) are fully culled below `START`, fade/bloom in across `START..FULL`, and are solid beyond.
// The trunks (depth 0) are always drawn, so the galactic view stays clean limbs-only, and the fractal
// thicket grows back in on approach. Tune from a screenshot alongside the MICRO/GALACTIC distances.
/// Zoom detail at which the finer dendrites begin to appear (still galactic-ish below this).
pub(crate) const DEND_LOD_START: f32 = 0.30;
/// Zoom detail at which the finer dendrites are fully present.
pub(crate) const DEND_LOD_FULL: f32 = 0.62;

// Profile "planet" nodes — the ~5 small aspect billboards at a client/note sun's dendrite tips.
// Sized ~1:3 against a sun so the galaxy reads as suns-with-links until you fly in. They bloom in
// *after* the dendrite thicket (START > DEND_LOD_START) and are hidden at galactic zoom.
/// Base billboard scale of a planet nucleus (a present, mid-value aspect); ~⅓ of a sun.
pub(crate) const PLANET_BASE: f32 = 0.30;
/// Planet halo size relative to its nucleus (soft surrounding glow).
pub(crate) const PLANET_HALO_REL: f32 = 2.2;
/// Zoom detail at which planets begin to fade in (past the dendrite thicket reveal).
pub(crate) const PLANET_LOD_START: f32 = 0.45;
/// Zoom detail at which planets are fully present.
pub(crate) const PLANET_LOD_FULL: f32 = 0.75;

// ---- renewal-warning state (M3: the graph reacts to SQLite) ------------------------------
// A client running low on sessions (or with an imminent renewal) shifts into a warning state: its
// nucleus slowly pulses and its hue shifts toward a hot orange-red, so a depleting client visibly
// "asks for attention" without any UI. Urgency 0..1 is computed in build_scene (RenewalWarn).
/// At/below this many sessions remaining on the active package, the warning ramps to full (0 left =
/// full warning, ≥ this many = none). ~11 left reads calm, ~2 left pulses hot.
pub(crate) const WARN_SESSIONS_AT: f32 = 4.0;
/// Days-to-renewal at/under which renewal proximity alone drives the warning to full.
pub(crate) const WARN_RENEWAL_DAYS: f32 = 14.0;
/// Extra nucleus brightness at the peak of a full-urgency warning pulse.
pub(crate) const WARN_GLOW: f32 = 1.1;
/// How far the nucleus hue shifts toward the warning colour at peak (0 = none, 1 = fully).
pub(crate) const WARN_TINT: f32 = 0.7;
/// Warning-pulse speed (sine argument multiplier) — slow, distinct from the fast idle twinkle.
pub(crate) const WARN_PULSE_SPEED: f32 = 2.4;
/// Warning hue (hot orange-red, HDR) the nucleus shifts toward.
pub(crate) const WARN_RGB: (f32, f32, f32) = (1.8, 0.45, 0.12);
