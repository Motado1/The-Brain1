# Pending visual / interaction verification

## 🟢 NEW — colorless glass tubes, lit only by soma + pulse (2026-06-25)
The connectors + dendrites no longer wear the network hue at rest. They're now **colorless clear
glass** (faint neutral Fresnel rim, `TUBE_GLASS_RIM` = 0.12) and take colour only from two sources,
both the **soma's** hue (never their own): **(1)** soma proximity — the cell body's light bleeds into
the tube near its soma end(s) and fades along `u` (`SOMA_BLEED_FALLOFF` = 0.4 of the length); **(2)**
the travelling pulse flood. Dendrites light at the root (u=0); connectors light at **both** ends
(both touch a soma). New `ends` uniform on `DendriteMaterial` + rewritten `pulse_wave.wgsl`.
- [ ] **Idle wiring is clear/colorless** — away from somas and with no pulse, tubes read as faint
      glass outlines, NOT a glowing orange web.
- [ ] **Colour pools at the cell bodies** — near each soma the tube ends glow that soma's hue (amber
      Business / blue Research) and fade to clear toward the middle of a run.
- [ ] **Pulses still flood** the tube with light as they travel (unchanged).
- [ ] Tune: `TUBE_GLASS_RIM` (idle visibility), `SOMA_BLEED_FALLOFF` (how far the soma light reaches),
      `TUBE_RIM_INTENSITY` (bleed brightness) — all in `tuning.rs`. WGSL is GPU-validated at runtime,
      so this is the first run that could surface a shader error — watch the console on launch.

## 🟢 NEW — cosmic LOD: dendrite tier reveal (2026-06-23)
Dendrites now split into a **trunk tier** (always drawn) and a **fine tier** (depth ≥ 1) that fades/
culls by zoom. `apply_dendrite_lod` hides the fine thicket in the galactic view and blooms it back in
on approach (rim-alpha ease `DEND_LOD_START..FULL` = 0.30..0.62 of global zoom; HUD shows band/zoom).
- [ ] **Zoom way out** → each brain reads as glowing somas joined by clean **trunk limbs only**; the
      fine fractal thicket is gone (not just faint — fully culled).
- [ ] **Fly back in** → the finer branches smoothly bloom in (no hard pop), thicket fully present by
      the Micro band.
- [ ] **Firing still sweeps the whole tree** when zoomed in (both tiers share the surge — no seam at
      the trunk→branch join, wave continuous root→tip).
- [ ] Tune `DEND_LOD_START` / `DEND_LOD_FULL` (tuning.rs) if the reveal feels too early/late for the
      scene scale.

## 🟢 NEW — Blender soma variants (6× `assets/models/soma_0N.glb`) (2026-06-25)
The procedural icosphere soma is **replaced** by six Blender-exported GLB cell bodies. Each neuron
gets one variant (picked by id hash, `pick_variant`), spawned as a `SceneRoot` at the node, scaled by
activation, breathing. The glowing nucleus + halo billboards and firing are unchanged (separate
entities reading through the glass). Loaded at startup by `soma_assets::setup_soma_assets` (ordered
before `load_graph`). **No procedural fallback** — missing files leave only the nucleus/halo glow.
- Files: `engine/crates/nbe_app/assets/models/soma_01.glb … soma_06.glb` (committed). Export spec in
  `assets/README.md`.
- [ ] **Run it** → neuron cell bodies are the Blender glass membranes (Fresnel rim, dark translucent
      centre), correctly sized/centred on each node, with the bright nucleus reading through.
- [ ] **Variety** → distinct shapes are visibly distributed across nodes (not all identical), stable
      across reloads.
- [ ] **Scale/orientation sanity** → no giant/tiny/offset somas (confirms radius-1.0-at-origin export
      + applied transforms). If any look wrong, it's an export-transform issue, not the engine.
- [ ] **Firing still glows/flares** from the nucleus through the membrane; soma breathes.
- ✅ CONFIRMED on RTX (2026-06-25): GLB variants load + render (oblate/egg shapes visible; distinct
  per node). Earlier "looks unchanged" was a **stale checkout** running the old procedural soma.
- **Engine re-skin added** (`apply_soma_skin`): the GLBs ship opaque-white materials, so each soma's
  meshes are swapped in-engine for the glass `SomaMaterial` (Fresnel rim, clear centre) tinted per
  network (amber CRM / blue Research) — `SOMA_RIM_INTENSITY`/`SOMA_RIM_ALPHA` in tuning. Restores the
  glowing translucent membrane + adds per-region tint, keeping the GLB shapes.
  - [ ] Somas now read as **glowing translucent glass** (not opaque white pills); nucleus shines
        through the centre; rim glows the network hue (amber vs blue).
  - [ ] Tune `SOMA_RIM_INTENSITY` (1.5) / `SOMA_RIM_ALPHA` (0.42) if too faint/too solid.
  - Diagnostic `report_soma_load` logs "=== SOMA GLB STATUS ===" (6× LOADED) at startup — can be
    removed once you're confident the pipeline is stable.


## 🟡 VERIFY NEXT — connections as ORGANIC biological processes (thin, bumpy, alive) (2026-06-19)
Owner spec (after glass-tube + glowing-core were both rejected): forget matching the soma exactly —
the connections should just look like **real, alive, organic** neural processes (like the thin
branching dendrite trees already do). The artificial part was the connectors: fat, smooth, straight
rods. Fixes: connectors now use the **same bumpy organic membrane skin as the dendrites**
(`bumped_tube_mesh`, 10-sided, `DEND_BUMP_*`), and ALL primary processes thinned to branch-like widths
(`DEND_ROOT_R` 0.05→0.028, `ROOT_FLARE` 0.03→0.022, `CONN_BODY` 0.025→0.018, connector floor
0.03→0.018) — a thick tube reads as a solid bar; a thin one reads as a living filament. Removed dead
`tube_mesh`. **Built blind — compiles + clippy clean; needs the owner's GPU.**
- [ ] **Connectors read as organic living tissue** — thin, bumpy/irregular, tapering filaments like the
      dendrite branches, NOT fat smooth orange rods. Compare a connector to a nearby dendrite branch:
      they should look like the same kind of process.
- [ ] **No fat solid bars** near the soma. If any process still reads as a solid bar up close, thin it
      more: `DEND_ROOT_R` (trunks), `ROOT_FLARE` / `CONN_BODY` (connectors) in `tuning.rs`.
- [ ] **Still glows + pulses** — thin organic filaments that glow softly and flood with light on a pulse.
- Knobs (`tuning.rs`): `DEND_ROOT_R` / `ROOT_FLARE` / `CONN_BODY` (thinness), `DEND_BUMP_REL` /
  `DEND_BUMP_FREQ` (organic surface lumpiness, shared by dendrites + connectors), `TUBE_RIM_INTENSITY`
  / `TUBE_RIM_ALPHA` (glow/transparency), `Bloom.intensity` in `scene.rs` `spawn_camera`.

<details><summary>earlier: tried matching the soma's glass membrane exactly (superseded)</summary>

## 🟡 connections wear the SOMA'S GLASS MEMBRANE (identical to the cell body) (2026-06-19)
Owner spec (after the additive attempt read as flat opaque bars): the connections should look
**identical to the soma's surface** — a clear, see-through translucent membrane with a glowing Fresnel
rim — NOT additive light strands (additive adds light, can never be see-through). So `DendriteMaterial`
is back to `AlphaMode::Blend` and `pulse_wave.wgsl` now uses the EXACT soma formula + the soma's
`RIM_POWER/RIM_INTENSITY/RIM_ALPHA` constants; the travelling pulse still floods the tube. Geometry
thinned further (a thin tube reads as glass; a fat one reads as a solid band up close). **Built blind —
compiles + clippy clean; the look needs the owner's GPU.**
- [ ] **Connections look identical to the soma** — clear see-through centre (you can see neurons
      *through* them like you can through the soma sphere) with the same glowing rim, same colour and
      brightness as the soma membrane. They will be dimmer than the soma *body* (no inner nucleus glow
      — owner accepted this).
- [ ] **Not too thick** — trunks `DEND_ROOT_R` 0.05, connector mouths `ROOT_FLARE` 0.03. If still too
      fat/solid up close, lower these and/or **`TUBE_RIM_ALPHA`** (more transparent centre).
- [ ] **Beads removed** — the bright 2.6×-HDR additive "beads of light" + junction glows that used to
      sit on every connection are gone (the soma's membrane has none); the connection is now just the
      glass tube. Also deleted the dead unused `themes` membrane/edge/dendrite materials.
- [ ] **Tube rim decoupled + dimmed from the soma** — a thin tube is "all rim", so at the soma's full
      `RIM_INTENSITY` it read as a bright solid bar. Tubes now use `TUBE_RIM_INTENSITY` 0.9 /
      `TUBE_RIM_ALPHA` 0.45 (vs soma 1.6 / 0.68) → a soft translucent glass line. Raise toward the
      soma values for brighter/solider tubes; the soma is unaffected.
- [ ] **Pulse still floods light along the tube** (data pulse on connectors; firing surge root→tip on
      dendrites) — timing/cooldown unchanged, only the resting look.
- [ ] **⚠ runtime WGSL (naga on GPU)** — if tubes render pink/black/invisible, send the console error.
- Knobs (`tuning.rs`): **`RIM_ALPHA`** (transparency — lower = more see-through; shared with the soma),
  `RIM_POWER`/`RIM_INTENSITY` (rim sharpness/brightness, shared with the soma), `DEND_PULSE_EMISSIVE`
  (pulse flood), `ROOT_FLARE` / `CONN_BODY` / `DEND_ROOT_R` (tube thinness);
  `Bloom.intensity` in `scene.rs` `spawn_camera`. (Note: tubes now share the soma's RIM_* — changing
  them changes both. Old per-row note about `color.a` profile flag no longer applies.)
</details>

## ⏳⏳⏳ NEWEST — `DendriteMaterial`: hollow membrane that fills with light on pulse
Rework (owner spec): ONE color-agnostic volumetric tube material for connectors + dendrites. At rest
a **hollow Fresnel-outlined membrane** (lit opaque rim, ~7% see-through centre — a clear "vein"); a
travelling pulse **overrides** the hollow centre, forcing alpha→1.0 + a massive emissive flood so the
tube **physically fills with light** where the pulse passes. The separate inner "wire" core mesh was
removed (owner disliked it); dendrites are now a single bumpy volumetric tube like connectors.
- [ ] **Tubes rest as clear hollow glass** with a lit outline (NOT a solid amber fill).
- [ ] **Pulse fills the tube** with intense light as it travels (connectors: data pulse; dendrites:
      firing surge root→tip).
- [ ] **Color-agnostic**: CRM tubes amber, Research tubes indigo (automatic via `base_color`).
- [ ] **⚠ runtime WGSL (naga on GPU)** — if tubes render pink/black, send the console error.
- Knobs (`tuning.rs`): `DEND_FRESNEL_POWER`=3.0 (rim sharpness vs hollow centre), `DEND_EDGE_EMISSIVE`
  =1.8 (rim glow), `DEND_CENTER_ALPHA`=0.07 (how see-through the centre is), `DEND_PULSE_EMISSIVE`=5.0
  (pulse fill brightness — drop first if it blooms to white). Material renamed `PulseWaveMaterial`→
  `DendriteMaterial`.
- Known follow-ups if it reads wrong: soma still uses `SomaMaterial` (soft rim) so the soma↔dendrite
  seam may differ — soften via `DEND_FRESNEL_POWER`/`DEND_EDGE_EMISSIVE` toward `RIM_POWER`/`RIM_INTENSITY`.

## ⏳⏳⏳ NEWEST — pulse WAVE (Slice 1) + channel cooldown — runtime WGSL
Replaced the discrete dot pulse with a continuous Gaussian wave gliding along the connection tubes
(`PulseWaveMaterial` / `pulse_wave.wgsl`), plus a per-channel rest so signals don't ping-pong.
- [x] **Wave travels along connections** — owner confirmed the pulsing looks good.
- [x] **Travel speed** — `PULSE_SPEED`=0.3, owner: "perfect".
- [x] **Wave length / crest size** — `PULSE_WIDTH`=0.07 (small tight crest), owner-confirmed.
- [x] **Absorb → pause → reply rhythm** — `CHANNEL_COOLDOWN`=2s, no ping-pong (owner-requested).
- [ ] **⚠ Shader validates at RUNTIME (naga), not at build.** If connection tubes render
      pink/black/invisible, it's a WGSL error — send the console `naga`/shader error.
- [ ] **Slice 2 — dendrite surge.** When a soma fires, light should travel root→tip out through its
      dendrite tree (one smooth wave sweeping the whole tree in spatial order, not every branch
      flashing at once). Knobs: `DEND_WAVE_SPEED` (travel rate), `DEND_WAVE_AMP` (crest brightness)
      in `tuning.rs`. Same `pulse_wave.wgsl` (already validated) — low shader risk; the unknown is the
      uv-by-distance look + trigger timing.
- [ ] **Soma spacing + connector fusion.** Clusters should be more spread out (less clutter) and
      connections should root *into* the cell bodies (no fat tube stabbing straight through a soma,
      no surface gap). Knobs: `density_radii` in `geometry.rs` (raise the 45/34 vector for more
      spread); `ROOT_EMBED` in `tuning.rs` (lower = deeper fuse). This was the owner's "looks off".
- [ ] **Soma + connector look pass (from owner reference images — thin tendrils + granular core).**
      Owner wants connectors as **thin tapering tendrils** (not fat trumpets) and somas as a **granular
      textured mass with a glowing core through a translucent membrane**. Changed: `ROOT_FLARE`
      0.42→0.15 + connection waist 0.045→0.04 (thin connectors); `SOMA_BUMP` 0.18→0.28 (granular
      silhouette); `RIM_ALPHA` 0.85→0.68 (translucent membrane, core glows through). All in `tuning.rs`
      except the waist (scene.rs `axon_radii` call). *Distant neurons already read well; the test is the
      foreground close-up.* If connectors still too fat → lower `ROOT_FLARE`; still washed/opaque →
      lower `RIM_ALPHA` + shrink the nucleus/halo billboard scales (`r*1.6` / `r*3.5` in scene.rs);
      want richer warm colour → the theme is desaturated, deepen it in `theme_rgb` (scene.rs ~400).
      *Deeper granular surface (smoky micro-crevice light-scatter) needs the planned soma.wgsl noise
      (runtime-WGSL, not built).*
- [ ] **Connectors = branches off a trunk (not pinched pipes) + deeper red.** Replaced the symmetric
      flare-waist-flare profile (`axon_radii`) with `branch_radii`: a monotonic concave taper, full
      where it roots into the *bigger* soma (trunk), easing toward the other end — stays substantial
      through the middle (kills the thread-thin waist the owner flagged). Knobs: `ROOT_FLARE`=0.16
      (rooted base size), `BRANCH_TIP_RATIO`=0.6 (how thin the far end gets; raise→more even, lower→
      more taper) in `tuning.rs`. Red deepened: `theme_rgb` Business 1.0,0.55,0.15 → 1.0,0.30,0.07.
      *Still wanted (owner): connectors+somas reading more as one continuous trunk→branch system —
      next lever is curve flow (`curve_params` bow 0.1/sag 0.03 in scene.rs ~636) and/or fusing the
      connector base into the soma surface more seamlessly.*
- [ ] **NEXT Slice 3** (not built): radius "pump" via vertex displacement — owner already said yes.

## ⏳⏳⏳ NEWEST — granular-soma Phase 1 (geometry; no shader, low risk)
Goal (from the owner's reference images + Gemini's "anatomy of the real node"): somas read as a
**textured granular mass** with filaments **erupting like roots** from the surface, light compounding
at the junctions — the **hybrid** look (still glowing from within, not a fully opaque rock).
- [ ] **Lumpy soma silhouette** — somas are now displaced icospheres (a 6-mesh pool, picked per node),
      so outlines are bumpy/granular and the Fresnel rim ripples across the contours instead of a
      smooth ball. *Knobs:* `SOMA_BUMP` (lump depth), `SOMA_SUBDIV` (resolution), `SOMA_VARIANTS`
      (pool size) in `tuning.rs`; mesh in `geometry.rs::displaced_sphere`.
- [ ] **Root flaring at junctions** — filaments start *inside* the soma (`ROOT_EMBED`) and flare wide
      (`ROOT_FLARE`) where they meet it, so they look fused to the body, not kissing the surface.
      *Knobs:* `ROOT_EMBED` / `ROOT_FLARE` in `tuning.rs`; logic in `scene.rs` web loop + `axon_radii`.
- [ ] **Junction glow** — an additive glow dot sits at each filament→soma anchor; on a well-connected
      node these compound into a bright root cluster. *Knob:* `JUNCTION_GLOW` in `tuning.rs`.
- *Next (NOT built — Phase 2, gated on this looking right):* procedural 3D noise in `soma.wgsl` for
  smoky micro-crevice light-scatter on the surface (runtime-WGSL → naga-error risk, like Phase 2/3).

## ⏳⏳ shader-overhaul plan, verify in order (commits `a737743`, `ddb87f7`)
- [ ] **Phase 1 (`a737743`)** — beads of light along filaments (3/edge); thick gray streaks gone
      (background fibers now hair-thin); cooler deep-blue atmosphere (fog + ClearColor). Knobs:
      bead count/scale in `scene.rs` web loop; `DistanceFog.color`/`ClearColor`.
- [ ] **Phase 2 (`ddb87f7`) — soma Fresnel "cell-wall" shader (FIRST custom shader).** Somas should
      show a glowing rim at the silhouette + clear centre, nucleus glowing within. Knobs: `RIM_POWER`
      / `RIM_INTENSITY` / `RIM_ALPHA` in `tuning.rs`.
      **⚠ Shader is validated at RUNTIME, not at build.** If somas render wrong/invisible/pink, it's
      a WGSL error — check the console log for a `naga`/shader error and send it; that's the fix
      signal. (Shader: `crates/nbe_app/src/soma.wgsl`, material in `shaders.rs`.)
- [ ] **Phase 3 (not built yet)** — UV-scroll "flowing light" fiber shader. Gated on Phase 2 looking
      right (don't stack two unverified runtime-shaders).

> **Purpose:** the agent builds blind on headless Linux; the owner verifies the GUI on Windows.
> This file pins **exactly what still needs a desktop run** so backend work can continue without
> losing track. Work the checklist top-to-bottom in one session, tick items, and note what to tune.
> Keep this file current: when something is confirmed good, move it to **Verified**; when a build
> changes the look, add a new item.

**Run it:**
```powershell
git pull origin main
cargo run -p nbe_app --release -- --db brain.db
```

---

## ⏳ PENDING — verify these on the desktop

### Camera
- [ ] **Free-flight zoom (distance-aware)**: scroll flies the pivot toward whatever's under the
      cursor. Point AT a soma → eases toward it and stops gently close up (no overshoot, no hard
      wall); point at open space past a soma → flies briskly through. Rotation eases off when zoomed
      in close so a soma you're next to doesn't whip past. *Tune:* `orbit_camera` scroll `f` step
      `scroll*0.18`, radius ease `scroll*0.12`, floor `0.4`; rotation `sens = 0.005 * (radius/12)
      .clamp(0.3,1.0)`.
- [ ] **Zoom-to-cursor pivot** (commit `dc329bd`): zoom into a cluster, then left-drag — does it
      orbit *what you zoomed into* (not a stale far point)? This was the reported bug.
      *Tune:* `systems.rs orbit_camera` — `scroll * 0.18` (radius rate), `scroll * 0.35` (pull toward
      node); cone in `pick_index` `perp < along*0.05 + 3.0`.
- [ ] **Smooth depth-of-field** (`8ed125a`): zoom/fly softens fore/background into bokeh smoothly,
      no snapping. *Tune:* `update_dof` lerp rate `* 3.0`.

### Picking & selection (the interaction foundation — verify before building the visual frontend)
- [ ] **Hover highlight**: pointing at a neuron brightens it (and swells its halo). Too subtle / too
      strong? *Tune:* `fire_render` boosts — hovered `(1.5, 1.25)`.
- [ ] **Click-to-select**: a stationary left-click selects a neuron (strong highlight) and opens the
      **detail panel** (bottom-left) with its info. *Tune:* selected boost `(2.4, 1.6)`.
- [ ] **Click vs orbit feel**: does selecting vs rotating feel natural, or is clicking too grabby /
      too fussy? *Tune:* cone `perp < along*0.05 + 3.0` (width); click-vs-drag distance `< 5.0`
      in `pick_node`.
- [ ] **Detail panel**: readable, well-placed (LEFT_BOTTOM), closes to deselect. Glassmorphic
      styling not done yet.

### Synaptic flow
- [ ] **Pulse traffic rules** (`0ecba78`): pulses look calm/deliberate; you should **never** see two
      pulses overlapping on one fiber. *Tune:* `tuning.rs QUEUE_CAP`, pulse speed in `fire_scheduler`
      / `spawn_pulse` (`0.12 + 0.12·rand`).
- [ ] **Firing pace & brightness** (`198b996`): calm, soft glow — not a busy/blinding "star field".
      *Tune:* `tuning.rs` FIRE_BASE / FIRE_NEED / FIRE_DECAY / FLARE_GAIN / HALO_SWELL /
      PULSE_ENERGY / MAX_PULSES_PER_FIRE.

### Node & network look
- [ ] **Glow-from-inside neurons** (`70ddff7`): translucent membrane body with a brighter core
      inside — light contained in a cell, not a bare dot. Membrane too opaque / too invisible?
      *Tune:* `scene.rs` membrane `srgba(r*0.5,…,0.22)`; core size `shape * 0.5`.
- [ ] **Soma→axon taper** (`375f1e6`): edges visibly grow out of the cell body (flared at the soma,
      thin waist). *Tune:* `axon_radii(.., ri*0.5, rj*0.5, 0.1)` — flare factor `0.5`, waist `0.1`.
- [ ] **Misshapen cells**: slightly irregular/oriented, not perfect spheres. *Tune:* `shape` scale
      `0.78 + rand*0.5` in `scene.rs`.
- [ ] **Networks identical except hue** (`bb2c879`, `6e14a47`): Business = warm amber, Research =
      blue-purple; same density/texture, themed edges/membranes/dendrites/pulses. *Tune:* `scene.rs
      theme_rgb` — Business `(1.0,0.55,0.15)`, Research `(0.5,0.42,1.0)`; density `geometry.rs
      density_radii` `(32.6, 24.8, 32.6)`.
- [ ] **Fewer CRM nodes**: ledger folded into clients → Business ≈ one neuron per client. (Demo seed
      still has many clients; real `brain.db` will be sparse.)
- [ ] **Background depth fibers** (`8ed125a`): faint hair-thin layer adds depth without clutter.
      *Tune:* `scene.rs` fiber count `200`, alpha `0.05`; `geometry.rs background_fibers_mesh` thin
      `0.0016`.
- [ ] **Distance fog vs purple Research**: fog is warm-amber tinted — does it clash with the purple
      cluster in the distance? *Tune:* `scene.rs spawn_camera` `DistanceFog.color`.

---

## 🔒 BLOCKED — backend built, but needs the visual frontend before it can be verified
These are tested headless (logic) but have **no UI yet** to trigger/observe them. Verify once the
Phase B3 frontend (hover ring + action buttons) emits the events. (Backend: commit `bff35a3`.)
- [ ] **Hover action buttons** appear after ~450ms linger (`InteractionState` / `HOVER_THRESHOLD`).
- [ ] **Sprout / Link / Edit / Dissolve** buttons fire `UiRequest*` → `ops::sprout/link/delete`.
- [ ] **Apoptosis**: deleting a node fades it over 800ms then despawns; it stops firing immediately.
- [ ] **Financial scale**: client neuron size reflects earned revenue (`TargetVisualScale`) — not yet
      wired into the live transform (Breath drives scale today); needs the visual layer to apply it.

---

## ✅ VERIFIED (move items here once confirmed good on the desktop)
- Earlier renderer baseline (organic neurons, dendrites, two networks, sidebar fly-to, Add Research
  button) — confirmed via screenshots in earlier sessions.
- **2026-06-16, from desktop screenshots:**
  - **Camera zoom-to-cursor + rotate** — owner: "zooming and camera controls are much better." ✓
  - **Glow-from-inside neurons** (membrane + core) — read as cells, not dots (see amber-core note). ✓
  - **Soma→axon taper** — filaments visibly grow out of the cell bodies. ✓
  - **Two networks, distinct hues** — Business warm amber, Research blue-purple, same structure. ✓
  - **Spread-out networks + background depth fibers** — organic, not a dense ball. ✓
  - *Still pending (needs interaction, not visible in a still): picking/selection/detail panel,
    pulse-traffic flow, smooth DoF.*

---

## ⏳ NEW — verify next desktop run (commit `eadc3e3`)
- [ ] **Amber cores no longer blow out to white** — white-hot mix cut 0.7→0.25 in `geometry.rs
      node_emissive`, so amber blooms orange. Purple should be ~unchanged. Confirm amber keeps a
      visible membrane ring like the purple does.
- [ ] **Tissue is lit, not inherently coloured** — membranes/edges/dendrites/background fibers are
      now near-neutral translucent; their colour should come from the cores' bloom + neutral scene
      lights, not baked-in orange. **Risk (built blind):** could read too dark / washed if the
      neutral lighting is too weak. Knobs: tissue `emissive r*0.04..0.05` in `scene.rs` themes loop;
      `spawn_lights` directional illuminance; `AmbientLight` brightness `50` in `spawn_camera`.

## Notes / open tuning questions for the owner
- Keep the warm-amber + blue-purple palette (cyan/teal suggestion was rejected).
- Decide flare/waist strength on the soma→axon taper once seen.
