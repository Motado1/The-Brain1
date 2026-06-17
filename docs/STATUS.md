# STATUS / HANDOFF — The Neural Business Engine

> Living handoff so a fresh session can continue without losing context.
> **Branch:** `main` — all work lives here now (the old feature branches were consolidated in and
> removed). Commit & push straight to `main`.

## ⭐ SESSION FRONTIER (read first — latest state)

**Visual: granular-soma overhaul (hybrid look) in flight** (per-item desktop checklist in
`docs/VISUAL_VERIFICATION.md`). Bevy **0.18.1**.
- **Phase 1 done — geometry, headless-verified, no shader risk.** Somas are now lumpy **displaced
  icospheres** (`geometry.rs::displaced_sphere`, 6-mesh pool picked per node) for a granular,
  light-catching mass; filaments **embed + flare** into the soma like roots (`ROOT_EMBED`/`ROOT_FLARE`
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
