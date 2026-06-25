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

---

## M1 — Client solar-system structure + profile facet  `[~]`
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

### M1b — aspect data (`anatomy.rs`, pure + tested) `[ ]`
- [ ] Replace `Anatomy { sessions, packages, research_proxies }` with a fixed-order `Aspect` list
      (`AspectKind`, label, value 0..1, present). Client aspects from profile/slots/crm; knowledge
      aspects (Body/Status/Mentions/Topics/References) from KnowledgeFacet + edges.
- [ ] Rewrite tests (count/order/present/value-bounds/determinism, both networks).

### M1c — planets at tips (`scene.rs` `embed_anatomy` → `embed_planets`) `[ ]`
- [ ] Place each aspect at a distinct deterministic dendrite leaf tip; render as the sun nucleus+halo
      billboard pattern scaled small; `LodReveal`; `Planet { sun, aspect, value, label }` component.
- [ ] Do NOT add planets to `groups`/`index`/`BrainGraph`/`network_links` (topology preserved).
- [ ] Delete the old trunk-bead loop.

### M1d — scale/vastness (`tuning.rs`) `[ ]`
- [ ] `PLANET_BASE`, `PLANET_HALO_REL`, `PLANET_LOD_START/FULL` (~3:1 sun:planet, planets bloom after
      the dendrite thicket, hidden at galactic zoom). Optional `LOD_GALACTIC_DIST`/`density_radii`.

---

## M2 — UI shell  `[ ]`  *(build next, per owner)*
- [ ] Global dock (`ui.rs` `dock_ui`): CRM/Finance/Research buttons → `CameraGlide.to(network_view)`.
- [ ] Context side-panel: extend `detail_panel_ui`/`Picker` to show planet data + host action buttons.
- [ ] Floating 3D action buttons ("Log Session") via `Billboard`+`face_camera`+`LodReveal`+`WorldButton`;
      extend picking to hit them.
- [ ] One-click session log/decrement: `UiRequestSessionLog` → `on_session_log` → new
      `ops::session_log` → `recompute_renewal()` → `SceneControl.reload`.

## M3 — Data-driven life  `[ ]`
- [ ] Material states from session balance + renewal proximity (extend `recompute_activations`); ≤
      threshold → renewal-warning pulse/hue shift.
- [ ] Session-log → travelling pulse down the client's dendrite (trigger an immediate `DendriteWave`).

## M4 — Finance: numbers now, radial viz later  `[ ]`
- [ ] Surface existing forecasting in the Finance panel: `project_depletion`/`recompute_renewal`,
      `report_forecast`, `report_revenue`, `report_finance`.
- [ ] New `ops::report_tax` (estimated withholding per cleared package / by `tax_bucket`).
- [ ] Deferred: 3D Financial Time-Series Network (annual core → 12 monthly somas → payment nodes).

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
