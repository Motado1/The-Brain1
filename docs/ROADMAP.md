# ROADMAP — The Brain (master plan + progress tracker)

> Living roadmap for the galaxy/solar-system business engine. Check items off as they land.
> Pairs with `docs/STATUS.md` (cross-session handoff) and `docs/VISUAL_VERIFICATION.md` (owner's
> visual checklist). Legend: `[ ]` todo · `[~]` in progress · `[x]` done.

## Vision
A local-first, offline desktop "Brain": a personal-training business as a living 3D neural galaxy.
Large client/topic **sun** nodes form the galaxy; zooming in reveals their **planet** detail nodes;
the graph reacts to real SQLite state (sessions remaining, renewals). A 2D UI bridges to the 3D view
for action. Built blind on headless Linux (logic verified by tests/clippy), visually verified by the
owner on Windows/RTX 5080.

Owner decisions baked in: client planets = **profile data** (goals/diet/injury); finance = **numbers
now / radial viz later**; AI = **heuristics now / local LLM deferred**; after the structure lands,
build the **UI shell** first.

## ▶ Current position (resume here)
- **Done + pushed: M1, M2, M3, M4.** Profile-planet structure (M1), UI shell — dock + planet-aware
  detail panel + one-click session log (M2), data-driven life — renewal-warning pulse + session-log
  pulse (M3), and finance numbers — Forecast/Revenue/**Tax**/Retention panel + `ops::report_tax` (M4).
  Tree green: `nbe_data` 13 ✓, `nbe_cli` 35 ✓, `nbe_app` 24 ✓, clippy clean across the workspace.
- **Visuals owner-confirmed good:** somas, fused glowing connections (breathing turned off), planets.
- **Next: M5 — robustness & packaging** (`ops::maintain` PRAGMA/VACUUM; `cargo-bundle` standalone
  `.exe` + icon; release GPU profile). Then M6 (heuristics; local LLM deferred).
- **Deferred slices to revisit:** M2's floating 3D world buttons (session action already works from the
  panel); M4's 3D radial finance viz; the "galaxy→solar-system" double-click dive + DoF blur (scoped,
  not yet built).
- Branch: `claude/soma-dendrite-connections-9oq6rp`.

---

## M1 — Client solar-system structure + profile facet  `[x]`
Suns (clients/topics) form the galaxy; each client sun has ~5 small "planet" nodes at its dendrite
tips showing its **profile** (Fitness Goals, Dietary Needs, Injury History, Schedule, Contact). Planets
sit at dendrite tips and never enter the sun-linking loop → planets touch only their parent, suns
touch only suns. Reuses the dendrite tree (carries the firing wave) + LOD reveal (planets hidden when
zoomed out = galaxy view).

### M1a — `profile` facet (backend, headless, fully tested) `[x]`
- [x] `schema.rs`: bump `CURRENT_VERSION` 2→3; add `V3` `profile_facet` table + migration step.
- [x] `model.rs`: `ProfileFacet` struct; added `profile` to `EntityWithFacets`.
- [x] `repo.rs`: `profile_from_row`, `upsert_profile`, `get_profile`, `list_profile`; wired into
      `entity_with_facets`.
- [x] `snapshot.rs`: added `profile: Vec<ProfileFacet>` (`#[serde(default)]`) + export/import loops.
- [x] `seed.rs`: seed profile rows for demo clients (deterministic from ordinal, RNG stream untouched).
- [x] `ops/clients.rs`: `profile_set()` / `profile_view()`; exported via `ops/mod.rs`.
- [x] `main.rs`: `ProfileSet`/`ProfileView` subcommands.
- [x] `tests/cli_tests.rs`: `profile_facet_set_view_and_clear`. `nbe_data` (13) + `nbe_cli` (34)
      tests green, clippy clean, whole workspace compiles.

### M1b — aspect data (`anatomy.rs`, pure + tested) `[x]`
Replace `Anatomy { sessions, packages, research_proxies }` with a fixed-order aspect list. Concrete
design (already worked out — implement as-is):
```rust
enum AspectKind { Goals, Diet, Injury, Schedule, Contact,        // client sun, in this order
                  Body, Status, Mentions, Topics, References }    // knowledge sun, in this order
struct Aspect  { kind: AspectKind, label: String, value: f32 /*0..1*/, present: bool }
struct Anatomy { aspects: Vec<Aspect> }   // exactly 5 per sun, fixed order
```
`build_anatomy(snap) -> HashMap<String, Anatomy>` (needs `snap.profile` from M1a):
- Index: profile by id, knowledge by id, `client_ids: HashSet`, cadence sum per client from
  `snap.slots`, and edge tallies (mentions/topics by `source_id`, references by `target_id`).
- **Client** (each `snap.crm`): Goals/Diet/Injury via `text_aspect` from `ProfileFacet`
  (present = non-empty trimmed text, value 1.0/0.0); Schedule = `cadence` (value `cad/3.0` clamped,
  label `"{cad:.1}x/wk"`, present `cad>0`); Contact = `contact` text present, value by lifecycle
  (renewal 1.0 / active 0.6 / lead 0.4 / else 0.2).
- **Knowledge** (each `snap.knowledge` whose id ∉ client_ids): Body = `body_md` char count
  (value `len/400` clamped); Status = review_status (reviewed 1.0 / draft 0.4 / archived 0.15);
  Mentions/Topics/References via `count_aspect` (value `n/scale` clamped, scales 5/5/8).
- Empty client still emits all 5 (present=false) so layout is stable. Client takes precedence over
  a knowledge facet on the same id.
- [x] Tests: client 5-aspect fixed order + present flags + value bounds; knowledge edge counts;
      client-with-knowledge-facet → client set; empty client → 5 present=false. Drop the old
      `anatomy_groups_*` test.

### M1c — planets at tips (`scene.rs` `embed_anatomy` → `embed_planets`) `[x]`
- [x] `embed_planets(commands, halo_quad, planet_mats, branches, an, sun_entity, sun_r)`: tips =
      `branches.filter(|b| b.leaf).filter_map(|b| b.points.last())`; guard empty; place
      `aspects[i]` at `tips[(i*tips.len()/n) % tips.len()]`. Per aspect spawn a nucleus billboard
      (+ optional small halo) reusing the sun nucleus pattern (scene.rs ~501-540): `Mesh3d(halo_quad)`,
      additive `core_mat`, `Billboard`, `LodReveal { base_scale: PLANET_BASE*(0.6+0.4*value) [or 0.35
      if !present], start: PLANET_LOD_START, full: PLANET_LOD_FULL }`, `Planet { sun: sun_entity,
      aspect: kind, value, label }`, `SceneItem`.
- [x] Materials: build a shared `PlanetMats` palette keyed by `AspectKind` (10 mats, like the old
      `AnatomyMats`) once in `build_scene`, colored from each network's `theme_rgb` + per-aspect hue
      offset; replaces `AnatomyMats` (now unused). Share across planets (don't `materials.add` per node).
- [x] Call site (~scene.rs 581): pass `node` (sun entity) + `r`. Do NOT add planets to
      `groups`/`index`/`BrainGraph`/`network_links` (topology preserved by construction).
- [x] Add `Planet { sun: Entity, aspect: AspectKind, value: f32, label: String }` to `components.rs`
      (used by M2 hover/select labels + optional M3 flare). Delete the old trunk-bead loop + the
      `session_*`/`package_*`/`research` parts of `AnatomyMats`.

### M1d — scale/vastness (`tuning.rs`) `[x]`
- [x] `PLANET_BASE=0.30`, `PLANET_HALO_REL=2.2`, `PLANET_LOD_START=0.45`, `PLANET_LOD_FULL=0.75`
      (~3:1 sun:planet, planets bloom after the dendrite thicket since START>`DEND_LOD_START`=0.30,
      hidden at galactic zoom). Optional vastness: widen `LOD_GALACTIC_DIST`/`density_radii`.
- [x] Gate: `cargo test -p nbe_app` + `clippy` + `cargo check -p nbe_app`; update
      `docs/VISUAL_VERIFICATION.md` with the owner checklist (galaxy=suns+links only; suns≫planets;
      zoom-in blooms ~5 profile planets per client at tips, parent-only; both networks).

---

## M2 — UI shell  `[~]`  *(egui slice landed; 3D world buttons remain)*
- [x] Global dock (`ui.rs` `dock_ui`): top strip with CRM/Finance/Research/Galaxy → `CameraGlide`
      to the network view (Finance also flips the right panel to the Forecast report).
- [x] Context side-panel: `detail_panel_ui` now lists the selected node's five **profile planets**
      (aspect label + present dot + value bar, from `NodeInfo.aspects`) and, for clients, hosts the
      one-click session-log action buttons.
- [ ] **Floating 3D action buttons** ("Log Session") via `Billboard`+`face_camera`+`LodReveal`+
      `WorldButton`; extend picking to hit them. *Deferred — the picking-feel needs the owner's GPU;
      the action is already reachable from the side panel. This is the remaining M2 slice.*
- [x] One-click session log/decrement: `UiRequestSessionLog{node,outcome}` → `on_session_log` →
      `ops::session_log` (already exists; calls `recompute_renewal`) → `SceneControl.reload`.
      `SessionOutcome` enum (Completed/NoShow/Cancelled) maps to the DB status; unit-tested.

## M3 — Data-driven life  `[x]`
- [x] **Renewal-warning state:** per-client urgency 0..1 from sessions-remaining on the active package
      (`session_warn`) max'd with renewal-date proximity (`renewal_warn`), computed in `build_scene` and
      attached as `RenewalWarn`. `fire_render` reads it → the nucleus slowly pulses (`WARN_PULSE_SPEED`)
      and shifts hue toward hot orange-red (`WARN_RGB`/`WARN_TINT`) as a client depletes. Tuning:
      `WARN_SESSIONS_AT`/`WARN_RENEWAL_DAYS`/`WARN_GLOW`/`WARN_TINT`. 2 headless ramp tests.
- [x] **Session-log → pulse:** `on_session_log` now queues the client in a new `FireRequests` resource;
      `apply_fire_requests` fires that client once the scene rebuilds (sets `Firing.accumulator` high →
      the normal `fire_scheduler` path), so a visible pulse runs down its connections + a dendrite surge
      sweeps its tree. Frame-TTL'd so an unresolved id is dropped, not retried forever.
- **Implemented as:** a dedicated `RenewalWarn` signal (kept separate from the generic activation so the
  warning reads as *depletion*, not just "busy"); reuses the existing firing/pulse machinery for the
  session-log feedback rather than a bespoke wave. `nbe_app` 24 tests green, clippy clean.

## M4 — Finance: numbers now, radial viz later  `[x]`
- [x] Finance surfaced in the business panel: `Forecast` (`report_forecast`), `Revenue`
      (`report_revenue`), `Retention`, and now `Tax` tabs; the dock's **Finance** button opens it.
      (`report_finance`/`project_depletion`/`recompute_renewal` already existed and are reused.)
- [x] New `ops::report_tax` — estimated tax set-aside from package income minus deductible expenses,
      annual summary + per-month withholding; rate from the `tax_rate` config key (default 25%). CLI
      `ReportTax` + `BizTab::Tax`; `tax_report_estimates_set_aside` test (default + config-override).
- [ ] Deferred (owner decision): 3D Financial Time-Series Network (annual core → 12 monthly somas →
      payment nodes). Revisit after the numbers are trusted on the GPU.

## M5 — Robustness & packaging  `[ ]`
- [ ] `ops::maintain` (PRAGMA integrity_check + VACUUM), CLI + optional on-close hook.
- [ ] `cargo-bundle` standalone `.exe` + custom icon.
- [ ] `[profile.release]` lto/codegen-units/strip; verify wgpu→DX12 + bloom/DoF on the 5080.

## M6 — Automation & local AI (last)  `[ ]`
- [ ] Predictive grouping via plain-Rust heuristics (similar depletion rates, schedule gaps).
- [ ] Deferred/optional: offline local quantized LLM (Llama/Mistral) → read-only SQLite NL queries
      (spike model size + latency first; sandbox generated SQL to read-only).

---

## Constraints (do not break)
Offline, local-first, single SQLite file, zero cloud. Build-blind: all business logic pure +
unit-tested; visual/scale/material work screenshot-verified by the owner. Schema migrations additive +
versioned so existing `brain.db` upgrades cleanly; snapshot round-trip stays deterministic.

## Verification gate (every milestone, before handoff)
`cargo test -p nbe_data` · `cargo test -p nbe_cli` · `cargo clippy -p nbe_cli` ·
`cargo test -p nbe_app` · `cargo clippy -p nbe_app` · `cargo build -p nbe_app`. Then update
`docs/VISUAL_VERIFICATION.md` + `docs/STATUS.md`.

## Branch
`claude/soma-dendrite-connections-9oq6rp` — commit per logical step; push
`git push -u origin claude/soma-dendrite-connections-9oq6rp`. No PR unless asked.
