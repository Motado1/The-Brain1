# CLAUDE.md — orientation for this repo

> Read this first, then `docs/STATUS.md` for the living handoff (what's built, what's next).

## Two projects live here — know which one you're in
- **ACTIVE: `engine/`** — the **Neural Business Engine**, a native Rust workspace (Bevy + SQLite).
  This is where essentially all current work happens. Default here unless told otherwise.
- **LEGACY: repo root** — an older Next.js / Supabase / RAG web prototype (`app/`, `components/`,
  `lib/`, `supabase/`, the many `*_COMPLETE.md` files, `package.json`). Not maintained. Do **not**
  touch it unless the user explicitly asks about the web app.

## What the engine is
A private, **local-first, offline** desktop app for a personal-training business. A single
(optionally SQLCipher-encrypted) SQLite file — `brain.db` — is both the data store and a living
**"Brain"**: nodes are neurons that fire, edges carry pulses. Three domains (CRM/clients,
Research/knowledge, Financial/ledger) are regions of that brain. Two front-ends over one tested
core: a **CLI hub** (`nbe`) and a **3D renderer** (`nbe_app`).

## Hard constraints (do not break)
- **Local-first, offline, zero cloud.** Single SQLite file. The *only* network use is an explicit,
  on-demand Google Calendar pull from a private iCal URL.
- **Target:** Windows 11 + RTX 5080, native Rust + Bevy (`wgpu` → DX12). No browser/Electron.
- **Dev reality:** this agent runs on **headless Linux with no GPU.** You cannot open the window.
  Verify logic with `cargo test`/`clippy`/`check`; the **user runs the GUI on Windows** and sends
  screenshots. Build blind, compile-check, hand off.

## Commands (run from `engine/`)
```bash
cargo test -p nbe_cli            # the tested business/data logic — fast, always run this
cargo clippy -p nbe_cli          # keep clean
cargo build -p nbe_app           # compile-check the renderer (needs the libs below on Linux)
cargo clippy -p nbe_app
# user-only (GPU): cargo run -p nbe_app --release -- --db brain.db
# CLI: cargo run -p nbe_cli -- --db brain.db <command>   (e.g. `today`, `nudges`, `agenda`)
```
Compiling `nbe_app` on Linux needs system libs (CLI/data crates do not):
`libwayland-dev libxkbcommon-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev
libasound2-dev libudev-dev`.

## Crate map
- `nbe_data` — SQLite + SQLCipher; entity + facet schema (crm/ledger/knowledge + edge/activation/
  layer + package/session/slot/config); repo, seed, JSON snapshots.
- `nbe_cli` — **the hub** (`nbe` binary) and the **tested business logic** in `src/ops.rs`. Every
  feature is an `ops::*` function with tests in `tests/cli_tests.rs`, wired as a CLI command in
  `src/main.rs`.
- `nbe_app` — **the renderer** (Bevy + bevy_egui) in `src/main.rs`. Loads a `--db`, draws the Brain.
- `nbe_sim` — activation rules + action-potential propagation (`Sim` CPU model).
- `nbe_layout` — Sugiyama layered layout. `nbe_geometry` — organic curve/tendril geometry.
- `nbe_calendar` — ICS parser + event→client matcher.

## How we work (conventions)
- **GUI-first, one engine underneath.** Each feature ships as a **tested `ops` backend** (build +
  verify headless any time) and then a **button in `nbe_app`** (compile-checked here, visually
  verified by the user). The CLI is the power-user fallback. Mirror an existing `ops::*` function's
  shape and add a `tests/cli_tests.rs` test; keep `cargo test` + `clippy` green.
- **The renderer's "alive" layer** (firing/propagation/motes) reads activation that is recomputed
  from real facet urgency on load — keep visual liveliness tied to the data model, not faked.
- **Commit + push every meaningful step** to the working branch (see `docs/STATUS.md`). Keep
  `docs/STATUS.md` current when you add a feature — it is the cross-session handoff.
- Research import house format for Gemini docs: `docs/GEMINI_RESEARCH_FORMAT.md`.

## Pointers
- Living handoff / status / next steps: `docs/STATUS.md`
- **Pending desktop visual/interaction verification (what still needs the owner's eyes):**
  `docs/VISUAL_VERIFICATION.md` — keep current; the agent builds blind, so unverified visual changes
  accumulate here until the owner runs the GUI.
- Architecture + phased plan: `docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`, `docs/BUILD_PLAN.md`
- Renderer: `engine/crates/nbe_app/src/main.rs` · CLI logic: `engine/crates/nbe_cli/src/{ops,main}.rs`
