# STATUS / HANDOFF — The Neural Business Engine

> Living handoff so a fresh session can continue without losing context. Pairs with
> **`docs/ROADMAP.md`** (master plan + progress tracker) and **`docs/VISUAL_VERIFICATION.md`**
> (owner's GPU checklist). For orientation read `CLAUDE.md` first.

**Branch:** `claude/soma-dendrite-connections-9oq6rp` — all current work lives here (NOT `main`).
Commit + push every meaningful step. Only one chat should drive this branch at a time (parallel
sessions race on push — it's happened).

## 🧭 Where we are
The galaxy → solar-system business engine. Roadmap milestones **M1–M5 are done and pushed**; **M6 is
next**. Full detail + checkboxes in `docs/ROADMAP.md`.

- **M1 — structure:** each client/topic **sun** grows ~5 small **planet** nodes at its dendrite tips —
  the curated profile/knowledge "aspects" (`anatomy.rs` `AspectKind`/`Aspect`/`build_anatomy` →
  `scene.rs::embed_planets`, `Planet` component, `PLANET_*` scale/LOD consts). Backed by a new client
  **`profile` facet** (schema v3: fitness_goals/dietary_needs/injury_history).
- **M2 — UI shell:** top **dock** (CRM/Finance/Research/Galaxy camera jumps); **detail panel** lists a
  clicked node's aspects + one-click **session-log** buttons (`UiRequestSessionLog` → `on_session_log`
  → `ops::session_log` → reload). *Deferred:* floating 3D `WorldButton`s (action works from the panel).
- **M3 — data-driven life:** depleting/renewing clients shift into a pulsing hot-orange-red **warning**
  (`RenewalWarn` from sessions-remaining + renewal proximity, read by `fire_render`); logging a session
  **fires** that client so a pulse runs down its connections (`FireRequests`/`apply_fire_requests`).
- **M4 — finance numbers:** `ops::report_tax` (estimated set-aside from package income − expenses, rate
  from `tax_rate` config, default 25%) + a **Tax** tab beside Forecast/Revenue/Retention. *Deferred:*
  the 3D radial finance network.
- **M5 — robustness & packaging (agent side):** `ops::maintain` (integrity_check + VACUUM + WAL
  checkpoint, CLI `Maintain`); release profile (lto/codegen-units/strip); cargo-bundle metadata +
  `docs/PACKAGING.md`. *Owner-run on Windows:* add `assets/icon.png`, `cargo bundle --release`, verify
  wgpu→DX12.

**Next — M6 (automation & AI):** heuristic **predictive grouping** (flag clients with similar
package-depletion rates / localized schedule gaps) — pure `ops`, headless-testable. Offline local LLM
(Llama/Mistral → read-only SQL) is the deferred, optional final phase.

**Tests green:** `nbe_data` 13 · `nbe_cli` 36 · `nbe_app` 24 · clippy clean across the workspace.

## 🎨 Visual state (owner-confirmed unless noted)
- **Somas:** six Blender GLB cell bodies (`assets/models/soma_01..06.glb`, picked per node by hash via
  `soma_assets`), re-skinned in-engine to glowing translucent glass tinted per network
  (`apply_soma_skin` → `SomaMaterial`); glowing nucleus + halo billboards read through. **Confirmed.**
- **Connections:** colorless glass tubes that take colour from the **soma** (amber bleed near the root,
  fading along the tube) + the travelling pulse — fused into the membrane (flared roots emerging from
  the cell-body glow). **Confirmed.**
- **Breathing: OFF** (`BREATH_AMP`/`BREATH_GLOW_AMP` = 0 — owner preference; everything static + welded).
  The `animate_breath`/`animate_breath_with` machinery stays, gated by those consts.
- **Pending GPU verify** (see `docs/VISUAL_VERIFICATION.md`): the planets (M1), the dock/panel/session
  log (M2), and the M3 warning pulse + session-log pulse.

## Deferred / parked slices (pick up anytime)
- **Galaxy→solar "dive":** double-click a client → glide-center it + tighten DoF to blur the rest +
  push planets out for a grander scale. Scoped, not built.
- M2 floating 3D world buttons · M4 3D radial finance viz · M5 Windows `.exe`/icon (owner-run).

## Hard constraints (do not break)
- **Local-first, offline, zero cloud.** Single SQLite file (`brain.db`), optional SQLCipher. The only
  network use is an explicit on-demand Google Calendar pull from a private iCal URL.
- **Target:** Windows 11 + RTX 5080, native Rust + Bevy (`wgpu` → DX12). No browser/Electron.
- **Dev reality:** this agent runs **headless Linux, no GPU** — verify logic with
  `cargo test`/`clippy`/`check`; the **owner runs the GUI on Windows** and sends screenshots. Build
  blind, compile-check, hand off.

## Crate map (`engine/`)
- `nbe_data` — SQLite + SQLCipher; entity + facet schema (crm/ledger/knowledge/**profile** + edge/
  activation/layer + package/session/slot/config); repo, seed, JSON snapshots. Schema **v3**.
- `nbe_cli` — the `nbe` hub + the tested business logic in `src/ops/{mod,clients,research,pt,reports,
  admin}.rs` (clients, PT packages/sessions/slots, invoices/expenses, notes/links, reports incl.
  `report_tax`, `maintain`, calendar sync). Every feature = an `ops::*` fn + a `tests/cli_tests.rs` test.
- `nbe_app` — the Bevy renderer: `scene.rs` (build_scene/embed_planets), `systems.rs` (firing/pulse/
  LOD/camera), `anatomy.rs`, `soma_assets.rs`, `shaders.rs` (+ `soma.wgsl`/`pulse_wave.wgsl`),
  `interaction.rs`, `ui.rs`/`panel.rs`, `nav.rs`, `tuning.rs`, `components.rs`, `domain.rs`, `lod.rs`.
- `nbe_sim` — activation rules + pulse propagation. `nbe_layout` — Sugiyama. `nbe_geometry` — organic
  curve/tendril geometry. `nbe_calendar` — ICS parser + event→client matcher.

## PT business model (already in the CRM)
Clients buy **PT10/20/30** packages (N sessions), paid in full up front (lumpy cash). Sessions logged
as they occur; renew when depleted. Weekly **slots** (cadence 0.5–3×/wk) drive work-hours + renewal
ETA. `renewal_date` is re-derived (remaining ÷ cadence) on every package/session/slot change, so
renewals + `report_forecast` stay live. Profile (goals/diet/injury) is owner-entered.

## Run it (Windows PowerShell)
```powershell
cd C:\Users\<you>\The-Brain1
git checkout claude/soma-dendrite-connections-9oq6rp
git pull origin claude/soma-dendrite-connections-9oq6rp
cd engine
cargo run -p nbe_app --release -- --db brain.db
# empty scene? seed one: cargo run -p nbe_cli -- --db brain.db seed
# DB hygiene (app closed): cargo run -p nbe_cli -- --db brain.db maintain
```
Packaging steps: `docs/PACKAGING.md`. Headless verify (agent): `cargo test -p nbe_data -p nbe_cli`,
`cargo clippy -p nbe_app -p nbe_cli`, `cargo build -p nbe_app` (needs the X11/wayland/alsa/udev libs).

## Pointers
- Master plan + progress: `docs/ROADMAP.md` · GPU checklist: `docs/VISUAL_VERIFICATION.md`
- Packaging: `docs/PACKAGING.md` · Architecture: `docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`,
  `docs/BUILD_PLAN.md` · Research import format: `docs/GEMINI_RESEARCH_FORMAT.md`
