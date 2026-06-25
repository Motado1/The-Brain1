# STATUS / HANDOFF — The Neural Business Engine

> Living handoff so a fresh session can continue without losing context.
> **Branch:** `main` — all work lives here now (the old feature branches were consolidated in and
> removed). Commit & push straight to `main`.

## 🧭 ACTIVE WORK — see `docs/ROADMAP.md` (master plan + progress tracker)
The app's full roadmap (galaxy/solar-system structure → UI shell → data-driven life → finance →
packaging → AI) now lives in **`docs/ROADMAP.md`** with checkboxes. **Current branch for this work:
`claude/soma-dendrite-connections-9oq6rp`** (not `main`).
- **Done + pushed:** soma GLB variants + engine re-skin; colorless glass tubes; **M1 COMPLETE** —
  M1a (`profile` facet, `48b1b24`) + M1b/M1c/M1d landed together: `anatomy.rs` now yields a fixed
  five-aspect list per sun (`AspectKind`/`Aspect`), `scene.rs::embed_planets` weaves those as small
  **planet** billboards at the dendrite tips (per-network `PlanetMats`, `Planet` component for M2),
  and `tuning.rs` carries the `PLANET_*` scale/LOD consts. The old session-bead/package-twig anatomy
  is gone. Tree green: `nbe_data` 13 ✓, `nbe_cli` 34 ✓, `nbe_app` 21 ✓, clippy clean, builds.
- **M2 — UI shell: egui slice landed.** Top **dock** (CRM/Finance/Research/Galaxy camera jumps;
  Finance also flips the Business panel to Forecast); **detail panel** now lists the selected node's
  five profile planets (`NodeInfo.aspects`) and, for clients, hosts one-click **session-log** buttons
  (`UiRequestSessionLog`/`SessionOutcome` → `on_session_log` → `ops::session_log` → reload). `nbe_app`
  22 ✓, clippy clean, builds. **Remaining M2:** floating 3D `WorldButton`s + picking (deferred for
  owner GPU; the action already works from the panel).
- **Resume:** finish M2's 3D world buttons, then M3 (data-driven life). Pending owner GPU verify of
  both the planet look (M1) and the UI shell (M2) — see `docs/VISUAL_VERIFICATION.md`.

## 🟡 AWAITING GPU EYES — connections rebuilt as ADDITIVE GLOWING FILAMENTS (2026-06-19)

**The long-standing "tubes render SOLID" blocker was addressed by starting the connections over** (owner
agreed). Root cause was structural, not a shader bug: the connections were built as **translucent
hollow glass tubes** (soma's Fresnel rim around a see-through centre). A sphere has a big camera-facing
front that stays clear so only its rim glows → translucent; a **cylinder is almost all grazing-angle**,
so Fresnel fills the whole width + front/back walls compound to opaque. Same material, opposite result,
purely from shape. No tuning of a glass-tube shader could fix a geometry-driven fill — and the
reference images aren't glass tubes anyway, they're **thin bright glowing strands**.

**What changed (built blind on Linux, compiles + clippy clean — needs the owner's GPU to verify):**
- **New model = additive emissive filaments.** `pulse_wave.wgsl` rewritten: the strand's camera-FACING
  front is the bright core (`pow(facing, core_power)`, the *inverse* of a Fresnel rim), brightness is
  modulated along the length, and the travelling pulse floods light on top. `DendriteMaterial` is now
  `AlphaMode::Add` (was `Blend`) — additive blending **never occludes or stacks to opaque**, so the
  whole "solid cylinder" class of bug is gone by construction. Soma stays `SomaMaterial`/`Blend`,
  untouched (owner likes it).
- **Length profiles** via `color.a` flag: dendrites (a=1.0) bright at the soma root → dim at the tip;
  connectors (a=0.0) bright at both soma ends → dim mid-span. Floor per profile in `rest.z`.
- **Thinner geometry:** `ROOT_FLARE` 0.12→0.06, `CONN_BODY` 0.05→0.025, `DEND_ROOT_R` 0.34→0.18.
- **Drive systems unchanged:** `drive_pulse_waves` / `drive_dendrite_waves` still just write the `wave`
  uniform — the pulse/firing-surge timing the owner already confirmed is reused as-is.

**Key files / knobs:** `pulse_wave.wgsl` + `DendriteMaterial` (`shaders.rs`); spawns in `scene.rs`
(dendrite ~623, connector ~728); `tuning.rs` → `FILAMENT_CORE_POWER`, `FILAMENT_GLOW` (**drop FIRST if
it blooms pale**), `FILAMENT_TIP_FLOOR`, `FILAMENT_MID_FLOOR`, `DEND_PULSE_EMISSIVE`, `ROOT_FLARE`,
`CONN_BODY`, `DEND_ROOT_R`; `Bloom.intensity` 0.3 in `scene.rs` `spawn_camera` (lower toward 0.18–0.22
if strands blow out). `RIM_*` are now **soma-only**. Dev is **headless — WGSL only validates on the
owner's Windows GPU** (if strands render pink/black/invisible, send the console `naga` error); iterate
via screenshot. See `docs/VISUAL_VERIFICATION.md` for the per-item checklist. **Then the deferred LOD
is next (see below).**

---

## ✅ Done & working (consolidated on `main`)
- **Pulse system:** continuous Gaussian wave along connectors (replaced dot pulses); per-channel
  cooldown (absorb → ~2s → reply, no ping-pong); pulse eases into the node (`Pulse.arrived/fade`).
- **Dendrite surge:** firing neuron's light runs root→tip (uv-by-distance), via `DendriteWave` +
  `drive_dendrite_waves`.
- **Material:** `DendriteMaterial` (renamed from `PulseWaveMaterial`) — one color-agnostic tube
  material; rest = soma membrane formula, pulse overrides (alpha→1 + `DEND_PULSE_EMISSIVE` flood).
  Single tube per dendrite (inner "wire" core removed). ⬆ appearance still unsolved (blocker above).
- **Soma:** six **Blender-exported GLB** cell bodies (`assets/models/soma_01..06.glb`), one per
  neuron picked by id hash (`soma_assets::{SomaAssets, setup_soma_assets, pick_variant}`), spawned as
  a `SceneRoot` scaled by activation + breathing; glowing nucleus + halo billboards read through the
  glass. Replaced the procedural `soma_mesh`/`displaced_sphere`/`network_links` (all removed). No
  procedural fallback. *Awaiting Windows visual check — see VISUAL_VERIFICATION.*
- **UI (ported from the other chat):** Ctrl+P/Cmd+K omni-search, cinematic camera glide, glass theme,
  sidebar fly-to (`ui.rs`, `search.rs`, `nav.rs` CameraGlide).
- **Branch fork RETIRED:** `session-frontier-continue-jap19w` was force-reset to `main`; `main` is the
  single source of truth. Dead branches `claude/claude-md-continuation-v10vu2`,
  `claude/neural-business-engine-arch-h7i8xj` (0 unique commits) deletable anytime.

## ⏭ cosmic **LOD** reveal — dendrite tier reveal DONE (awaiting Windows eyes)
LOD anchor (`lod.rs`: `compute_lod`, `LodState`, `apply_lod_reveal`, `LodReveal` on the embedded
anatomy) is now **wired to cull/bloom dendrite detail by distance**:
- `dendrite_tree` (geometry.rs) splits into a **trunk tier** (depth 0, always drawn) and a **fine
  tier** (depth ≥ 1) as two meshes sharing one continuous uv. Scene spawns them as two entities, each
  with its own `DendriteMaterial` + its own `DendriteWave` keyed to the same soma (so one firing
  sweeps both continuously).
- `lod::apply_dendrite_lod` fades the fine tier's rim alpha `0→full` across `DEND_LOD_START..FULL`
  (0.30..0.62 of global zoom) and **hides it entirely below start** → galactic view = clean glowing
  somas + trunk limbs; the fractal thicket blooms back in on approach. Tested (`reveal_weight`).
- **Still open:** per the original fork note, the **screen-space min-size floor** (somas/specks stay
  visible when far) and possibly tier-revealing the *anatomy beads* in the same ease. Owner should
  confirm the dendrite reveal reads clearly on the RTX (HUD shows band/zoom). See VISUAL_VERIFICATION.

<details><summary>earlier consolidation log (2026-06-18) — kept for reference</summary>

## 🚨 BRANCH FORK — CONSOLIDATION IN PROGRESS (2026-06-18)

**UPDATE — owner A/B-picked and `main` is now the consolidated winner.** After booting both versions
the owner chose, per feature: **1A 2A 3A 5A 8A** (keep main's visuals/dendrite/motes) + **4B** (pulse
eases into node) + **6B** (UI: omni-search/glide/glass) + **dendrites also attach to membrane**;
**7 (LOD) deferred** (owner couldn't tell it worked). All picks are now **landed on `main`**:
- `89cacb4` dendrites attach to membrane shell (DEND_EMBED 0.9).
- `30699aa` ported the UI (nav CameraGlide + omni-search + glass theme + search.rs + glide in
  orbit_camera) WITHOUT the other branch's pulse/dendrite/LOD rewrites.
- `f0a7da8` ported B's pulse ease-into-node (`Pulse.arrived/fade`, `PULSE_FADE_TIME`) on top of main's
  Gaussian wave + cooldown (both kept).
- **Remaining: port the LOD (mesh tiers + `MinScreenSize` floor) and make it verifiable** — it's fused
  into B's `dendrite_tree`, so redo it against main's uv-by-distance dendrites. Then **retire the other
  branch** (`git reset --hard origin/main` on `session-frontier-continue-jap19w`) to kill the fork.
- Dead branches (0 unique commits, deletable): `claude/claude-md-continuation-v10vu2`,
  `claude/neural-business-engine-arch-h7i8xj`.

<details><summary>original fork analysis (kept for reference)</summary>

There were **two active chats on two diverged branches** building overlapping features.
- **`main`** (canonical per CLAUDE.md) = this chat. Has the owner-**verified** organic visuals + pulse
  rhythm: Gaussian pulse wave + channel cooldown, dendrite surge (uv-by-distance), `branch_radii`
  trunk→branch taper, membrane-attach (`ROOT_EMBED` 0.92, core 0.9), deeper red, soma look passes.
- **`claude/session-frontier-continue-jap19w`** = the OTHER chat. **9 commits not on main**, and it
  independently built: **UI core (fuzzy omni-search + cinematic camera glide + glass theme)**,
  **cosmic LOD (dendrite mesh tiers + screen-space min-size floor)**, per-network mote colour, AND its
  own parallel **pulse-wave + dendrite-surge** rewrite + soma spacing. It LACKS main's 5 visual commits.
- **They are NOT mechanically mergeable:** both rewrote the same core with incompatible data models
  (`Pulse{arrived,fade}` vs main's cooldown model; `DendriteWave{age,active}` vs `{t,prev_intensity}`;
  `geometry::dendrite_tree` rewritten by both — their LOD tiers vs main's uv-by-distance). A blind
  auto-merge conflicts in tuning/components/geometry/scene and would not compile.
- **A trial merge was attempted on a throwaway branch and ABORTED; `main` is untouched.**

**Recommended consolidation (needs the owner + GPU verification — don't do blind):**
1. Pick ONE canonical = **`main`** (CLAUDE.md mandate; has owner-verified visuals/pulse).
2. **Stop the other chat from diverging further** (have it `git reset --hard origin/main` once
   consolidated, like the owner's machine does).
3. Base = main (keep its verified pulse/dendrite/visuals). Port the other branch's **net-new,
   separable** features on top: `search.rs` + omni-search UI, `nav` camera-glide, glass theme, LOD
   mesh tiers + `MinScreenSize`, mote colour. **Drop** the other branch's parallel pulse/dendrite
   rewrite (main's is the verified one). The LOD tiers are fused into `dendrite_tree`, so that port
   must be redone against main's uv-by-distance version — the one genuinely tricky piece.
4. Each ported feature = a verifiable slice, compile + test green, owner verifies on the GPU.
- **Dead branches** (0 unique commits, safe to delete anytime): `claude/claude-md-continuation-v10vu2`,
  `claude/neural-business-engine-arch-h7i8xj`.
</details>
</details>

## ⭐ SESSION FRONTIER (read first — latest state)

**Continuous pulse WAVE replacing the dot pulses — Slice 1 of 3 done & owner-confirmed.** The old
billboard "dot" pulse is gone; energy now travels as a glowing wave along the connection tubes.
- **Shader/material:** `PulseWaveMaterial` (`pulse_wave.wgsl` + `shaders.rs`) = the soma's glassy
  Fresnel rest state PLUS a travelling Gaussian crest `exp(-(uv.x - t_center)²/2w²)` along the tube
  length (HDR crest → bloom = liquid wave). Per-tube material instance. ⚠ **runtime WGSL — naga
  validates on the GPU, not at build;** if tubes render pink/black/invisible, get the console `naga`
  error.
- **Driving:** `Pulse` is now a *logical timeline only* (no mesh). `drive_pulse_waves` (systems.rs)
  maps each active `Pulse.t` → its connection's `wave` uniform, oriented to uv via
  `ConnectionWave.fwd_edge`, touching only active/just-idle materials (`WaveActive` set) so idle
  tubes aren't re-uploaded each frame.
- **Rhythm (owner-confirmed):** channels rest `CHANNEL_COOLDOWN`(2s) after absorbing a pulse.
  `advance_pulses` frees the channel + sets cooldown on arrival (no instant re-launch);
  `tick_channels` counts the rest down then releases the next queued (reverse) pulse;
  `fire_scheduler` only claims a channel that's free AND rested. Net: arrive → ~2s pause → reply
  back, no ping-pong.
- **Tunables (`tuning.rs`):** `PULSE_SPEED`=0.3 (owner: "perfect"), `PULSE_WIDTH`=0.07 (small tight
  crest, owner-confirmed), `PULSE_WAVE_AMP`=2.6, `CHANNEL_COOLDOWN`=2.0.
- **Cleanup:** removed `PulseAssets`, the dot billboard, and the now-dead `GraphNode.network` /
  `GraphEdge.path` fields. Connections now wear the unified glassy material (same family as dendrites).

**Slice 2 — dendrite surge — BUILT (awaiting GPU eyes).** Dendrite trees now wear a per-soma
`PulseWaveMaterial` (replaced the old `dendrite_of` SomaMaterial). When a soma fires, its light surges
root→tip through the tree: `drive_dendrite_waves` (systems.rs) detects the firing rising edge
(`Firing.intensity` jump via `DendriteWave.prev_intensity`), restarts `t=0`, advances at
`DEND_WAVE_SPEED`(1.2) out past the tips, then idles (touches only active trees). Crux fix:
`uv.x` now encodes **normalised distance-from-soma across the whole tree** (not per-branch 0→1) —
`TubeBuilder::add_with_u` + `normalize_u`, `grow_dendrite` tracks cumulative arc-length, normalised by
max reach in `dendrite_tree`. Test `dendrite_uv_runs_root_to_tip` guards it. Tunables `DEND_WAVE_AMP`(1.8).

**Soma-spacing + root-embed fix — DONE (addresses the "looks off" feedback).** Connections only do the
embed-inside-and-flare attachment when `distance > ri+rj+1.0`; close somas fell back to center-to-center
so the fat tube stabbed through both glowing balls. Fix: spread clusters out (`density_radii` 32.6/24.8 →
45/34) so more pairs take the proper path, and deepen the embed (`ROOT_EMBED` 0.82 → 0.7) so connectors
fuse with no surface gap. Knobs: raise `density_radii` (geometry.rs) for more spread; lower `ROOT_EMBED`
for deeper fuse.

**NEXT — Slice 3:** radius "pump" — vertex displacement so the tube visibly swells as the crest passes
(owner pre-approved). Highest naga-risk piece (custom vertex shader); keep it separate, build only after
Slice 2 validates on the GPU.


**Cosmic-scaling workspace (4-system arch, in progress).** Goal: abandon orbit/satellite placement;
all data lives inside one continuous fractal-dendrite anatomy, revealed by camera distance.
- **System 1 (LOD) — ANCHOR LAID (`lod.rs`).** `compute_lod` reads the camera each frame: global band
  from `OrbitCamera.radius`, focus node from nearest `eye→NodeInfo.pos`. Both run through the pure
  `detail_for_distance()` (cubic smoothstep, thresholds `LOD_MICRO_DIST`/`LOD_GALACTIC_DIST` in
  `tuning.rs`) → `LodState{zoom, focus, focus_detail}`. Live HUD readout (band + focus + detail) so
  it's verifiable while flying. 2 unit tests. NOT YET applied to geometry (needs the anatomy first).
- **System 2 (embedded anatomy) — BUILT, headless-verified.** `anatomy.rs::build_anatomy(snap)` (pure,
  tested) derives per-entity sessions/packages/research-proxies. `geometry.rs::dendrite_tree` now
  returns branch polylines; `scene.rs::embed_anatomy` weaves them onto the branches: sessions = a
  sequential bead chain along the trunks (gold = completed, red = no-show/cancel), packages + research
  proxies = terminal twigs at the tips (amber active / dim spent / indigo cross-domain). Every element
  has a `LodReveal` so `lod::apply_lod_reveal` scales it 0→full only on deep zoom (Micro). *Awaiting a
  Micro-zoom desktop screenshot to tune bead/twig sizes + the reveal window.*
- System 3 (kinetic flight + Ctrl+P fuzzy search): extend `CameraTarget` lerp → timed cubic ease;
  egui overlay fuzzy-matches `NodeRegistry` strings → sets target. NOT built.
- System 4 (glassmorphic egui): translucent dark panels, domain-accent trim (amber client / indigo
  research), left profile dock on focus. NOT built.
> Build order is verifiable slices (most of 2–4 only judgeable on the Windows GPU). LOD has nothing
> to fade until anatomy (2) exists, so 2 likely comes next. Earlier glowing-tube FilamentMaterial was
> REVERTED — keep the granular-soma + branching-dendrite baseline; don't reintroduce fat glowing tubes.
> **Render backend: Vulkan is now the default on Windows** (`main.rs`) — DX12's swapchain panics on
> window maximize/resize in this Bevy/wgpu version. `NBE_BACKEND=dx12|vulkan|gl` overrides.


**Visual: granular-soma overhaul (hybrid look) in flight** (per-item desktop checklist in
`docs/VISUAL_VERIFICATION.md`). Bevy **0.18.1**.
- **Phase 1 done — geometry, headless-verified, no shader risk.** Somas are now **Blender GLB
  variants** (`assets/models/soma_01..06.glb` via `soma_assets`, picked per node by hash) — superseded
  the earlier procedural displaced-icosphere pool; filaments **embed + flare** into the soma like roots
  (`ROOT_EMBED`/`ROOT_FLARE`
  in `tuning.rs`, `scene.rs` web loop); an additive **junction glow** dot sits at each anchor so light
  compounds where roots meet the surface (`JUNCTION_GLOW`). New tunables + 2 unit tests in `geometry.rs`.
  *Awaiting desktop screenshot to tune `SOMA_BUMP`/flare/glow.*
- **Branching dendrites done — geometry, headless-verified.** `geometry.rs::dendrite_mesh` now grows
  a fractal tree (`grow_dendrite` recurses `DEND_BRANCH_DEPTH` levels, 2–3 children, tapering thick
  trunk → hair tips), starting each trunk just inside the soma surface (`DEND_EMBED`/`DEND_ROOT_R`)
  so it fuses smoothly — the reference-neuron structure. Knobs in `tuning.rs`; unit test in `geometry.rs`.
  *Awaiting desktop screenshot.* (NOTE: an earlier glowing-tube `FilamentMaterial` overhaul was tried
  and **reverted** — owner preferred the granular-soma baseline; don't reintroduce fat glowing tubes.)
- Phase 2 (NOT built, gated on Phase 1 looking right): procedural 3D noise in `soma.wgsl` for smoky
  micro-crevice surface scatter — runtime-WGSL (naga-error risk).

Earlier shader work (still pending desktop verify):
- Phase 1 done (`a737743`): beads of light on filaments, hair-thin background fibers, cooler
  deep-blue atmosphere.
- Phase 2 done (`ddb87f7`): **first custom WGSL material** — `SomaMaterial` Fresnel "cell-wall"
  (`crates/nbe_app/src/soma.wgsl` + `shaders.rs`); per-network rim; tuning consts
  `RIM_POWER/INTENSITY/ALPHA` in `tuning.rs`. **⚠ WGSL compiles at RUNTIME, not at `cargo build`** —
  if somas render wrong/invisible/pink, it's a `naga` shader error in the console log; paste it to fix.
- Phase 3 (not built): UV-scroll "flowing light" fiber shader (`TubeBuilder` `uv.x` 0→1 +
  `globals.time`). **Gated**: verify Phase 2 on desktop before stacking a second runtime-shader.

**✅ Just done — Client auto-linking** (`ops::note_import`, `crates/nbe_cli/src/ops/research.rs`):
an import doc can now carry a `Clients:` header (front-matter or leading line) alongside `Tags:`.
Names are matched case-insensitively to CRM contacts (`repo::list_crm`); each match gets a
`note→client` **"mentions"** edge, unmatched names are reported (`"; no client matched [..]"`), and
the whole mentions batch is inserted atomically via `db.transact(...)`. Parser refactored to an
`ImportHeader` struct + shared `push_csv` helper. The GUI **Add Research** button benefits for free
(it routes through `note_import`). Covered by `note_import_links_named_clients_and_reports_unmatched`
in `tests/cli_tests.rs`; full suite + clippy green.

**Non-visual features still unfinished** (verifiable headless; most-actionable first):
1. **`transact()` adoption** — `nbe_data::Db::transact`/`integrity_check`/`checked_backup` are built +
   tested but only the new `note_import` client-mentions batch uses `transact` so far; the rest of
   `note_import` and `sprout`/`package_add`/`delete` are still non-atomic. Wrap them (needs
   `&Connection`-based helper variants of `new_entity`/`set_activation`/`ensure_topic`).
2. **Spatial-UI interaction backend is built but INERT.** `crates/nbe_app/src/interaction.rs`
   (`InteractionState`, `UiRequest{Sprout,Link,Edit,Dissolve}` + listeners → `ops::*`, `Dissolving`,
   `TargetVisualScale`) is unit-tested. The listener/`update_*` systems **are scheduled** in
   `main.rs`, but they're starved: **nothing writes any `UiRequest`** (no emitters) and `Dissolving`
   is **never inserted**, so the action plumbing never fires. `TargetVisualScale` *is* inserted on
   clients (`scene.rs:476`) and recomputed by `update_financial_scale`, but **no system lerps it onto
   the transform** — computed, not applied. Completing it = the B3 spatial UI (hover ring + action
   buttons + detail-panel actions, which write the `UiRequest`s) plus a scale-apply system — that
   part is visual.
3. **Money pacing** (monthly income goal vs forecast+actual) — roadmap item, not built; pure `ops`.
   NOTE: a per-ledger `pacing_target_cents` field already exists on `LedgerFacet` (schema-wide) as a
   foundation; decide whether the monthly goal rides that or a separate `repo::config_set/get` key
   (like the calendar URL).
4. **Research auto-tagging** (offline keyword rules → auto topics) — roadmap item, not built; pure `ops`.

**Headless verify:** `cd engine && cargo test -p nbe_cli && cargo clippy -p nbe_app -p nbe_cli`.

## What this is
A private, **local-first** desktop app for a personal-training business: a single encrypted
SQLite file is visualized as a living **"Brain"** — an ANN/neuron + mycelial network. Three
domains (CRM/clients, Research/knowledge, Financial/ledger) are regions of the brain. The user
operates it both via a **terminal hub** (works today) and a **3D renderer** (in active visual
iteration).

## Hard constraints (do not break)
- **Local-first, offline, zero cloud.** Single-file SQLite (`brain.db`), optional SQLCipher
  encryption. The only network use is an explicit, on-demand Google Calendar **pull** via a
  private iCal (.ics) URL.
- **Target hardware:** Windows 11 + RTX 5080 + Ryzen 9 7900X (12c) + 64 GB. Native Rust + Bevy
  (`wgpu` → DX12). No browser/Electron.
- **Dev reality:** this agent builds on **headless Linux (no GPU)** — so verify logic with
  `cargo test`/`cargo clippy`/`cargo check` and **the user runs the GUI on their Windows box**
  and sends screenshots. Build blind, compile-check, hand off. Commit + push every step.

## Workspace (`engine/`, all green: ~50 tests, clippy clean)
- `nbe_data` — SQLite + SQLCipher; Entity-Component facet schema (entity + crm/ledger/knowledge
  facets + edge + activation + layer + **package/session/slot/config** for PT); repo, seed,
  JSON snapshots. Schema v2 migrates v1.
- `nbe_layout` — Sugiyama layered layout (barycenter crossing reduction).
- `nbe_sim` — activation rules + action-potential propagation; **`Sim` engine** (`tick(dt)`:
  fire → propagate pulses → bump arrivals → decay, with a refractory period) — the GPU-free CPU
  model the renderer drives one tick/frame (returns what fired/arrived for spark visuals).
- `nbe_geometry` — deterministic organic curve + mycelial tendril geometry (glam Vec3).
- `nbe_calendar` — ICS parser + event→client matcher + `EventSource` (ureq HTTPS, `http` feature).
- `nbe_cli` — **the hub** (`nbe` binary): clients, PT packages/sessions/slots, invoices/expenses,
  notes, links; reports (revenue cash+earned, work-hours, renewals, activation, **agenda**,
  **forecast** = projected monthly income, **retention** = renewal/repeat rates,
  **nudges** = clients about to run out of sessions, re-sell before they lapse,
  **today** = one-glance morning briefing composing today's sessions + renewals due this week +
  low packages);
  `recompute-activation` (persist fresh activation from facets for the renderer); calendar-sync;
  **research topics/tags** (`note-tag`/`note-untag`/`topic-list`, `note-list --tag`): a topic is a
  hub neuron in the Research region, notes link to it — so tagged research clusters in the brain;
  **brush-up/review** (`review [--tag] [--limit]`, `note-review`): least-recently-reviewed notes
  first; reviewing fires the note's neuron (lights it up, cools over the recall window);
  **research import** (`note-import <file.md>`): parse a markdown doc (front-matter / `Title:`/`Tags:`
  header), create a note, link it to topic hubs — the backend for the GUI "Add Research" button;
  export/import. **Edits/state-transitions:** `client-update`, `note-update`, `unlink`, `delete`
  (cascades facets/edges/packages/sessions/slots); `session-list/update/delete`,
  `slot-update/delete`, `package-list/delete` (deleting an active package restores the
  previous one); short-id resolution like entities. Lib+bin, fully tested.
- `nbe_app` — **the renderer** (Bevy 0.18 + bevy_egui 0.39). Loads a `--db`, renders the Brain.
  Right-hand **Business panel** (tab buttons: Agenda/Sessions/Renewals/Forecast/Revenue/Retention
  + Refresh) reuses the `nbe_cli::ops` report handlers, read-only.
  **➕ Add Research button** (left sidebar): native file dialog (`rfd`) → `ops::note_import` → live
  scene rebuild (`SceneItem` marker + `build_scene` + `apply_reload`), so the new note/topic neurons
  appear without restart. First write-from-UI action + the reusable live-update loop for all future
  buttons. *Compiles clean (cargo check) but unverified visually — needs a desktop run.*
  **The "alive" layer** (domain-independent, rides an addressable `BrainGraph` of node handles +
  edge adjacency built at scene time): neurons **fire** (integrate-and-fire — a base ambient charge
  for everyone plus more scaled by real activation/need; flares emissive + swells the halo, then
  decays); firing **propagates** pulses of light along outgoing edges into neighbours, which can
  re-fire — capped/attenuated cascades that fade; **ambient shimmer + drifting dust motes** keep it
  alive at rest. Activation is **recomputed from facets on load** so liveliness tracks real urgency.
  Tunables are `const`s in `nbe_app/src/tuning.rs` (FIRE_BASE/FIRE_NEED/FIRE_DECAY/FLARE_GAIN/
  PULSE_ENERGY/QUEUE_CAP/MOTES_PER_NETWORK).
  **The renderer is now split into modules** (`nbe_app/src/{main,components,domain,tuning,nav,
  geometry,scene,ui,systems,panel}.rs`); `ops.rs` likewise split into `ops/{mod,clients,research,
  pt,reports,admin}.rs` — pure reorganisation, all paths preserved.
  **Visual style pass (from desktop screenshots + reference images):** calmer/slower firing + soft
  glow (not stars); networks spread out and **sized to node count** (`density_radii`, radius ∝ n^⅓)
  so a 35-node and 350-node cluster read identically; **per-network theming** — Business warm amber,
  Research blue-purple — applied to membranes/edges/dendrites/pulses; ledger folded into clients so
  Business ≈ one neuron per client; edges are a **nearest-neighbour mesh** (not raw data links) that
  **grows out of the somas** (soma-surface start + `axon_radii` flare); neurons are a translucent
  **membrane + inner glowing core** (light from inside); pulses are soft round glow billboards.
  **Pulse traffic rules (A1):** `EdgeTraffic` resource — one channel per connection, single
  occupancy + directional mutex + capped queue, so the flow stays clean.
  **Camera zoom fix:** scroll now snaps the pivot onto the node under the cursor and shrinks the
  orbit radius (smooth fly-to), so zoom-then-rotate orbits what you zoomed into. Smooth DoF focus
  pull; dim background micro-fibers for depth.
  **Interaction foundation (B1/B2):** click a neuron → `pick_node` cone-raycast selects it →
  floating **detail panel** shows its `ops::show` view; hovered/selected neurons highlight in 3D.
  *All compiles + clippy clean; needs a desktop run to verify/tune the look + interaction feel.*

> **Direction:** the GUI is becoming the primary interface (the owner avoids the terminal). Plan:
> every `ops` handler gets a button + live redraw; CLI stays as the tested engine/fallback. Phased UI
> build-out (UI-0 foundations/selection+detail → UI-1 research → UI-2 clients/PT → …) is in the plan.

> **Headless build note:** compiling `nbe_app` on Linux needs system libs:
> `libwayland-dev libxkbcommon-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev
> libasound2-dev libudev-dev` (winit/audio/gamepad backends). Not needed for `nbe_cli`/`nbe_data`/etc.

## PT business model (already in the CRM)
Clients buy packages **PT10/20/30** (=N sessions), **paid in full up front** (lumpy cash).
Sessions logged as they occur ("James Bywater 9/10"); renew when depleted. Frequency 0.5–3×/wk
via weekly **slots** → drives work-hours + renewal ETA. Revenue tracked **both** as cash-in-by-
month and earned-per-session. **Renewal auto-projection:** a client's `renewal_date` is re-derived
(from active-package remaining ÷ slot cadence) on every package/session/slot change, so renewals +
the **forecast** report stay live; `report_forecast` projects future up-front cash by month assuming
like-for-like renewal.

## Visual direction (LOCKED from user's reference images)
- **Galaxy/overview = one Brain silhouette** (not separate clusters). Two hemispheres
  (CRM left, Research right) + lower-central mass (Financial), nodes shell-biased to the cortex.
- **Zoom in = biological neuron detail**: a big "cell-body" client neuron wired by filaments to
  many **small** info-nodes, with **pulses flowing outward** from the client. (NOT orbiting —
  it's a still web; size + connections carry meaning.)
- **Look:** deep-navy/black bg, electric blue/cyan/green nodes, **warm amber hotspots** where
  activation is high, heavy HDR bloom, curved organic filaments, travelling amber sparks,
  gentle node "breathing", slow idle camera drift.
- **Navigation:** left **egui sidebar** — Galaxy view button; per-domain collapsible headers
  (counts + "go to cluster"); alphabetized node lists; click → **camera fly-to** (cancelled by
  manual drag/scroll).

## Current renderer state (`nbe_app`)
Brain-shaped layout with 3 regions, sidebar nav + fly-to, amber-hot activation, sparks/breathing/
drift all working at 160+ FPS on the RTX 5080. Node size = hierarchy (clients big, info small).
Edges culled to `weight >= 0.55`.

## Run it (Windows PowerShell, from `engine/`)
```powershell
# CLI hub (real data):
.\target\release\nbe.exe --db brain.db --help
# Visual (demo data) — re-seed to change density:
.\target\release\nbe.exe --db demo.db seed --entities 320 --edges 900
cargo run -p nbe_app --release -- --db demo.db
```
First-time setup script: `engine/scripts/setup-windows.ps1` (installs Rust/MSVC/Perl/NASM, builds).

## NEXT STEPS (in priority order)
0. **Verify the whole visual + interaction pass at the desktop** — the pinned checklist of exactly
   what needs the owner's eyes lives in **`docs/VISUAL_VERIFICATION.md`** (work it top-to-bottom).
   (Lots built unrun): firing/flares,
   per-network theming (amber Business / purple Research), soma-axon taper, glow-from-inside
   neurons, pulse traffic flow, the **camera zoom fix**, and **click-to-select → detail panel +
   hover/selection highlight**. Tune cone-pick threshold / click-vs-drag distance / panel placement
   if needed. Knobs live in `tuning.rs` + `domain.rs theme_rgb`.
0b. **Gemini design doc — phased plan agreed.** Done: A1 traffic rules, A2 background fibers, A3 DoF,
   camera fix, B1/B2 picking + detail panel + highlight. **Next: Phase B3** (corner action hub +
   hover ring of Sprout/Edit/Dissolve + wire create/link/delete to `ops`), then **Phase C shaders**
   (Fresnel volumetric somas + UV-scroll in-fiber pulse waves — slow to iterate headless), then
   **Phase D** life-event animations (neurogenesis/apoptosis; embers as entities, not a discard
   shader). Skipped by owner: gaze auto-framing. Palette stays warm/purple (not Gemini's cyan).
1. **Tune the brain overview** from the latest screenshot: hemisphere proportions, central
   fissure, optional brainstem stalk, hotspot amount, node density.
2. **Zoom-in level-of-detail (the big one):** flying into a client makes it the large cell-body
   neuron with its info-pieces (packages/sessions/linked notes & invoices) spread on **dendrite
   filaments** with **pulses travelling outward**. Uses `nbe_geometry` tendrils + `nbe_sim`.
   *(The CPU propagation model now exists: `nbe_sim::Sim` — drive one `tick`/frame and render the
   returned fired/arrived events as sparks. This is P4's logic; the GPU compute-shader version is
   a later optimization. `recompute-activation` already persists correct activation for display.)*
3. **Real cross-domain links** (client ↔ its invoices/notes) surfaced as the per-client web.
4. **Sidebar polish** (search box, selected highlight, domain colors) + connector highlight on select.
5. **Live calendar test** with the user's real private iCal URL (only verifiable with network).

## Key files
- Renderer: `engine/crates/nbe_app/src/main.rs`
- Data/schema: `engine/crates/nbe_data/src/{schema,model,repo,snapshot,seed}.rs`
- CLI: `engine/crates/nbe_cli/src/{ops,main}.rs`
- Geometry: `engine/crates/nbe_geometry/src/{curve,tendril}.rs`
- Design docs: `docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`, `docs/BUILD_PLAN.md`
