# The Neural Business Engine — Build Plan

> Implementation breakdown for the architecture in
> [`ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`](./ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md).
>
> **Approach:** a *walking skeleton* — every phase produces a runnable binary with an explicit,
> testable acceptance gate, so a regression in any layer is caught before the next builds on it.
> **Phases P0–P4 are the single-sprint MVP.** P5–P6 are follow-on.
>
> **Stack:** native Rust · Bevy (`wgpu` → DirectX 12) · `bevy_egui` · SQLite + SQLCipher (`rusqlite`).
> **Target:** RTX 5080 · Ryzen 9 7700X · 64 GB · Windows. Budget: 60 FPS = 16.67 ms; stretch 144 Hz = 6.94 ms.

---

## Cross-cutting test infrastructure (build first, reuse everywhere)

| Tool | Purpose | Used by |
|---|---|---|
| **Seed generator** (`--seed N`) | Deterministic 500-entity / 1,200-edge DB with mixed facets + layer assignments | every phase |
| **Frame-time HUD** | On-screen ms + FPS + draw-call + particle counters | every perf gate |
| **GPU timestamp queries** | Per-pass cost (geometry / bloom / compute) in ms | P2–P4 |
| **Tracking global allocator** | Asserts flat RSS over time | soak test |
| `cargo test` + `clippy` / `fmt` | Unit/integration logic gates | P1–P4 |

---

## P0 — Scaffold (engine boots)

| Step | Build | Acceptance test |
|---|---|---|
| 0.1 | Cargo workspace + Bevy app, blank window | `cargo run` opens a window; `cargo build` + `clippy` clean |
| 0.2 | Force/confirm **DX12** backend, log adapter | Log prints `Dx12` + `RTX 5080` adapter name |
| 0.3 | Frame-time HUD overlay | HUD shows ms/FPS, reads at vsync (~6.9 ms @144 Hz) |
| 0.4 | Orbit/pan/zoom camera + placeholder cubes | Manual nav works smoothly; camera reused in P2 |

**Gate:** window runs at vsync with HUD, on DX12.

---

## P1 — Data engine (the source of truth)

| Step | Build | Acceptance test |
|---|---|---|
| 1.1 | SQLite migrations: `entity`, `crm/ledger/knowledge_facet`, `edge`, `activation`, `layer_assignment` | Integration test applies schema to a fresh file; asserts all tables/indexes exist |
| 1.2 | SQLCipher toggle + Argon2id key derivation | Open **without** key fails; open **with** key succeeds; encrypt→reopen round-trip |
| 1.3 | Repository layer (load/save entities + facets + edges) | Round-trip unit test per facet; **backlink query** (`edge WHERE target_id=?`) returns expected set |
| 1.4 | Seed generator (500 / 1,200, mixed facets, layers) | Generated DB has **exact** counts; deterministic for a fixed seed |
| 1.5 | JSON snapshot export/import + `VACUUM INTO` | export→import→export is idempotent; counts + checksums preserved |

**Gate:** `cargo test` green; a seeded encrypted `brain.db` exists and round-trips.

---

## P2 — Core render (the graph appears)

| Step | Build | Acceptance test |
|---|---|---|
| 2.1 | Load DB → ECS world (entity + components) | ECS entity count == DB; component presence matches facets (assert) |
| 2.2 | **Sugiyama layered layout** (x by layer; y by barycenter ordering) | Unit test on a small known graph: `x` monotonic by layer; ordering pass **lowers** a crossing-count metric |
| 2.3 | Instanced node spheres positioned by layout (**1 draw call**) | 500 spheres visible; draw-call counter == expected; FPS ≥ 60 |
| 2.4 | Instanced **straight** edges (**1 draw call**) | 1,200 edges connect correct endpoints (spot-check IDs); FPS holds |
| 2.5 | Full-scale static perf gate | Sustained **≥60 FPS** (target 144) at 500 / 1,200; record frame ms |

**Gate:** the static ANN topology renders left→center→right at locked FPS.

---

## P3 — Activation visuals (neurons come alive)

| Step | Build | Acceptance test |
|---|---|---|
| 3.1 | HDR target (`Rgba16Float`) + ACES/AgX tonemap | Emissive > 1.0 tonemaps correctly (no premature white clip) |
| 3.2 | `activation.value` → node **scale + emissive** mapping | Set known values; assert per-instance uniform reflects them; high-activation node visibly larger/brighter |
| 3.3 | **Bloom** (dual-filter chain) | Glow halos around hot nodes; bloom-pass GPU timestamp **< ~1 ms**; FPS holds |
| 3.4 | Activation system from business triggers (renewal proximity, priority, recall) | Unit test: trigger fn with fixed inputs + injected clock → expected activation |

**Gate:** activation data visibly drives size + glow; bloom within budget.

---

## P4 — Action potentials (data flows) — MVP complete

| Step | Build | Acceptance test |
|---|---|---|
| 4.1 | Particle ring buffer + **compute shader** integrating position source→target | Capture positions over frames: particle advances along edge; arrival detected |
| 4.2 | Fire-event enqueue + spawn system | Firing an edge spawns N particles; `edge.weight` scales speed/count (assert) |
| 4.3 | Arrival **bumps target activation** (propagation) | After arrival, target `activation.value` increases; cascade visible |
| 4.4 | Stress test: all 1,200 edges firing, ~100k–1M particles | FPS **≥60**; particle counter on HUD; compute-pass ms recorded |
| 4.5 | **Soak test** (1–4 h) | Tracking allocator + GPU-mem query show **flat RSS/VRAM**; no FPS decay; no crash |

**Gate:** sparks fire along synapses, update destinations, hold 60+ FPS, zero leak growth.

---

## Follow-on (post-MVP)

- **P5 — Domain CRUD:** `bevy_egui` panels for CRM / ledger / knowledge facets; backlink panel;
  financial roll-up views feeding Output nodes. *Test:* edit in panel → DB write → ECS/render reflect it.
- **P6 — Hardening:** passphrase UX, export/import UX, packaging the `.exe`, extended soak.
  *Test:* fresh-machine cold start; encrypted-file portability.

---

## Definition of Done (MVP)

A single Windows `.exe` that, from a local encrypted `brain.db` seeded with 500 entities and
1,200 edges, renders the layered ANN topology with activation-driven scale/glow and
compute-shader action-potential sparks, sustaining **60+ FPS** with **flat RSS/VRAM** over a
multi-hour soak — and a green `cargo test` suite covering the data, layout, and activation logic.
