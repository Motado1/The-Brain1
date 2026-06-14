# STATUS / HANDOFF — The Neural Business Engine

> Living handoff so a fresh session can continue without losing context.
> **Branch:** `claude/neural-business-engine-arch-h7i8xj` · everything below is committed & pushed.

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
  **forecast** = projected monthly income, **retention** = renewal/repeat rates);
  `recompute-activation` (persist fresh activation from facets for the renderer); calendar-sync;
  **research topics/tags** (`note-tag`/`note-untag`/`topic-list`, `note-list --tag`): a topic is a
  hub neuron in the Research region, notes link to it — so tagged research clusters in the brain;
  **brush-up/review** (`review [--tag] [--limit]`, `note-review`): least-recently-reviewed notes
  first; reviewing fires the note's neuron (lights it up, cools over the recall window);
  export/import. **Edits/state-transitions:** `client-update`, `note-update`, `unlink`, `delete`
  (cascades facets/edges/packages/sessions/slots); `session-list/update/delete`,
  `slot-update/delete`, `package-list/delete` (deleting an active package restores the
  previous one); short-id resolution like entities. Lib+bin, fully tested.
- `nbe_app` — **the renderer** (Bevy 0.18 + bevy_egui 0.39). Loads a `--db`, renders the Brain.

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
