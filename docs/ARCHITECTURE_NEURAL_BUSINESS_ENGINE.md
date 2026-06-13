# The Neural Business Engine — Architecture Proposal

> A private, single-user, local-first desktop application whose entire UI **is** an
> Artificial Neural Network graph. CRM, a backlinked knowledge base, and a financial
> ledger are fused into one continuously-animated topology the operator drives in real time.
>
> **Author:** Principal Software Architecture review
> **Date:** 2026-06-13
> **Status:** Proposal (architecture only — no application code in this revision)
> **Target machine:** RTX 5080 · Ryzen 9 7700X (8C/16T) · 64 GB RAM · NVMe SSD · **Windows-only**

---

## 0. Executive Summary

The recommendation is a **native Rust engine**, not a web stack:

| Layer | Recommendation | Why |
|---|---|---|
| **App Shell / Runtime** | **Bevy** (native Rust binary, `winit` window) | ~15–30 MB `.exe`, instant cold start, no Chromium, no Node, **no GC pauses** |
| **Graphics Engine** | **`wgpu` → DirectX 12** (via Bevy's renderer) | GPU instancing + **compute-shader particles** + built-in **HDR/bloom**; saturates the 5080, idles nothing |
| **Data Engine** | **SQLite (single file) + SQLCipher** via `rusqlite` | One portable `.db`, **AES-256 at rest**, zero cloud, zero server |
| **UI panels** | **`bevy_egui`** | Native immediate-mode forms for CRM/ledger/knowledge editing |
| **Data model** | **Entity-Component (ECS)** mirrored into SQLite **facets** | One entity = one neuron; its *role* is emergent from composition, exactly as required |

This proposal explicitly **rejects** Electron, Tauri/WebView2, Next.js/React, Canvas 2D,
WebGL, and browser-WebGPU for this workload, and justifies each rejection with concrete
performance and constraint reasoning below.

> **Note on the existing repository.** The current `The-Brain1` codebase is a
> Next.js 14 + React-Three-Fiber + Three.js + **Supabase** web app. It is a capable
> visualization prototype, but it violates two *hard* constraints: Supabase is a **cloud
> dependency** (fails local-first/privacy) and Next.js is a **web-server shell** shipping a
> garbage-collected JS runtime (jeopardizes a locked 60+ FPS under sustained particle load).
> Per the owner's direction this proposal is a **clean-slate rewrite**; none of the existing
> rendering code is carried forward.

---

## 1. The Recommended Stack

### 1.1 App Shell / Runtime — Native Rust binary (Bevy)

The application is a **single native Windows executable**. There is no browser engine, no
embedded web server, no Node runtime, and no JavaScript anywhere in the hot path.

**Why Bevy specifically.** Bevy is a native Rust game/app engine built on `wgpu` and `winit`.
It provides — out of the box — the four things this project needs most: a parallel **ECS**
world, a `wgpu` renderer with a customizable **render graph**, a production **HDR + Bloom**
pipeline, and `winit` windowing. Critically, **Bevy's ECS is the literal shape of the data
model this app requires** (§3): one entity, many optional components, behavior emergent from
composition.

**Rejected alternatives (App Shell):**

- **Electron** — *excluded by the owner, and correctly so.* Bundles a full Chromium + Node
  per process (~150 MB+ on disk, ~150–300 MB idle RAM). V8's garbage collector introduces
  non-deterministic stop-the-world pauses that show up as dropped frames precisely during the
  sustained animation this app runs continuously. There is no path to a *locked* 60+ FPS
  through a GC'd render loop.
- **Tauri / WebView2** — lighter than Electron (uses the OS WebView2, no bundled Chromium),
  and a legitimate choice for many desktop apps. But the render path is still **DOM + JS +
  GC**, and "native" GPU work would go through *WebGPU-in-WebView*, i.e. an extra abstraction
  layer over the very same DirectX 12 we can target directly. On a single Windows target it
  adds indirection and a GC for **zero** upside.
- **Qt / C++** — maximal native control, but a heavier toolchain, manual memory management
  (reintroducing the leak class we want to design out), and no ECS. Rust + Bevy gives
  equivalent native performance with **memory safety by construction**.

### 1.2 Graphics Engine — `wgpu` targeting DirectX 12

All rendering is GPU-driven. On Windows, `wgpu` selects the **DX12** backend by default,
talking to the RTX 5080 with no browser sandbox in between. This unlocks **compute shaders**
(for the particle/action-potential system), explicit buffer/memory control, and deterministic
frame submission across the Ryzen's cores.

**Graphics options evaluated (Canvas vs WebGL vs WebGPU vs Native):**

| Option | Verdict | Reasoning |
|---|---|---|
| **Canvas 2D** | ✗ Reject | CPU-bound, no GPU instancing, no shaders. Collapses below 60 FPS at low-thousands of sprites — cannot host 1,200 edges + particle field + bloom. |
| **WebGL** | ✗ Reject | Visually capable, but single-threaded JS draw submission, **no compute shaders** (particle sim needs clumsy transform-feedback/texture-ping-pong), and GC spikes. Leaves the 5080 mostly idle. |
| **Browser WebGPU (e.g. Three.js `WebGPURenderer`)** | ✗ Reject | Real compute + modern API, and production-ready since Three r171 — but still runs **inside a WebView/JS/GC sandbox**, and the renderer is officially "experimental." Strictly dominated by native `wgpu` on a single Windows target. |
| **Native `wgpu` → DX12** | ✓ **Choose** | Compute shaders, explicit memory, deterministic frame times, full multicore submission, direct line to the 5080. |
| **Hand-written DX12 / Vulkan** | ◼ Overkill | Would buy nothing measurable at 500 nodes / 1,200 edges; costs months and reintroduces manual resource lifetimes. `wgpu` already lowers to DX12. |

### 1.3 Data Engine — SQLite (single file) + SQLCipher

- **One portable file.** The entire database is a single `brain.db`. Backup = copy the file.
  A compacted export is `VACUUM INTO 'backup.db'`. This satisfies "portable, single-file
  database or highly clean backup format."
- **Encryption at rest (optional toggle).** `rusqlite` with the **`bundled-sqlcipher`**
  feature gives transparent **AES-256** encryption. The key is derived from a user passphrase
  via **Argon2id** and applied with `PRAGMA key`. Encryption is opt-in so an unencrypted file
  stays trivially portable when the operator wants that.
- **Concurrency.** WAL mode lets the render/UI thread read while a background writer commits,
  so persistence never stalls a frame.
- **Human-readable backup.** A plaintext **JSON snapshot** export is provided alongside the
  binary `.db`, giving a clean, diff-able, version-controllable backup format.
- **Zero cloud, zero server.** Everything is in-process. The app has full read/write with no
  network of any kind — local-first by construction.
- **Optional analytical sidecar (deferred): DuckDB.** For heavy financial OLAP (pacing
  curves, tax-bucket roll-ups across history), an embedded DuckDB columnar engine is an
  option. At 500 entities, SQLite **views / materialized aggregate tables** are more than
  enough, so DuckDB is documented as a scale-out path only, not part of the MVP.

---

## 2. Performance Justification

**Frame budget.** 60 FPS = **16.67 ms/frame**. The workstation can drive 144 Hz, so the real
design target is **6.94 ms/frame**. The math below shows the GPU work sits far inside even the
tighter budget; the architecture's job is to make sure the *CPU side and memory behavior*
never spoil it.

### 2.1 Geometry & draw calls — the web bottleneck is designed out

The classic reason web/Canvas graph UIs stutter is **per-object draw calls issued from a
single GC'd JS thread**. This architecture eliminates that class of problem structurally:

| Element | Count | Technique | Draw calls | Triangles/frame |
|---|---|---|---|---|
| Nodes (neurons) | 500 | GPU **instanced** icospheres | **1** | 500 × ~1.3K ≈ **0.65 M** |
| Edges (synapses) | 1,200 | Instanced billboard quads / thin tubes (**straight**) | **1** | ~**2.4 K** |
| Action-potential particles | up to ~1 M | **Compute-shader** sim → 1 instanced draw | **1** | ≤ ~2 M |
| UI (egui) + post passes | — | batched | ~5–15 | trivial |

Total **~10–20 draw calls per frame**. The RTX 5080 processes triangles in the **billions per
second**, so ~2.6 M tris/frame is a **sub-millisecond** geometry cost. Because the user spec
calls for *straight* weighted pathways, edges need **no curve tessellation** — a major saving
versus organic-spline graph renderers.

### 2.2 Action potentials — particles live entirely on the GPU

Sparks are simulated by a **compute shader** that integrates each particle's position along
its parent edge from `source` to `target`, reading from a **pre-allocated ring buffer**. Spawn
rate and speed scale with `edge.weight`. The CPU only *enqueues fire events*; it never touches
per-particle data. A budget of ~1 M particles is orders of magnitude beyond the "a few sparks
per active edge" visual need, and the 5080 simulates it in well under a millisecond.

### 2.3 Bloom / glow — the heavy post-process, costed

Activation is rendered as light: activated neurons write **emissive values > 1.0** into an HDR
target, so *activation literally drives glow*. The bloom chain is:

```
HDR scene (Rgba16Float)
  → threshold/prefilter
  → progressive downsample  (6 mips, dual-filter / Kawase)
  → progressive upsample     (bilinear, additive)
  → composite + ACES/AgX tonemap
  → sRGB swapchain
```

At 4K (~8.3 M px) the full chain touches roughly **1.5×** the base pixel count across all
mips. Against the 5080's GDDR7 bandwidth (~960 GB/s class), that is on the order of
**0.5–1.0 ms** — comfortably inside the 6.94 ms target. Bevy's `Bloom` component implements
exactly this dual-filtering chain, so it is configuration, not new shader code.

### 2.4 CPU & threading — the Ryzen 9 is never the limiter

Bevy's **parallel ECS schedule** spreads work across the 7700X's 16 threads:

- **Layout system** — incremental Sugiyama layered relaxation (§3) for node positions.
- **Activation system** — evaluates business triggers (renewal proximity, task priority,
  recent KB recall) and updates `activation.value`.
- **Spawn system** — converts "fired" edges into GPU particle spawn commands.
- **Render submission** and **DB I/O** run as separate systems; persistence is a background
  writer over a channel, never blocking a frame.

At 500 nodes / 1,200 edges these systems are microsecond-to-low-millisecond work; the CPU has
enormous headroom.

### 2.5 Zero memory leaks over long sessions — a guarantee, not a hope

This is where native Rust earns its place:

- **RAII / ownership.** GPU buffers, pipelines, textures, and ECS entities are freed
  **deterministically** when dropped/despawned. There is **no garbage collector**, hence no
  stop-the-world pauses and no GC-driven frame spikes.
- **No web leak classes.** No detached DOM nodes, no closure-captured listeners, no
  retained-by-accident React state — those failure modes don't exist here.
- **Pre-allocation.** Particle and instance buffers are sized and allocated **once at
  startup**; steady-state per-frame heap allocation is ≈ **0**, so RSS and VRAM stay flat.
- **Validation.** A multi-hour **soak test** with a tracking allocator asserts flat RSS +
  VRAM and sustained 60+ FPS — leak prevention is a test gate, not an assumption.

---

## 3. Data & Sync Architecture

### 3.1 The core insight: one entity, many facets, emergent role

The owner's requirement — *"a single entity seamlessly behaves as a client, a financial
line-item, or a knowledge node depending on its current activation layer"* — is **exactly an
Entity-Component model**. An entity is one neuron. What it *is* at any moment is determined by
**which components (facets) it carries** and **which layer/lens is active** — never by a
hard-coded type column. This maps cleanly onto Bevy's ECS in memory and onto **facet tables**
in SQLite on disk:

```sql
-- the neuron: the single thing
entity(id, created_at, updated_at)

-- "behaves as a client"
crm_facet(entity_id PK→entity, contact, lifecycle_stage,
          session_schedule, renewal_date)

-- "behaves as a financial line-item"
ledger_facet(entity_id PK→entity, amount, invoice_status,
          is_expense, tax_bucket, pacing_target)

-- "behaves as a knowledge node"
knowledge_facet(entity_id PK→entity, body_md,
          template_type, review_status)

-- straight, directed, weighted synapse
edge(id, source_id→entity, target_id→entity, edge_type, weight, directed)

-- drives node scale + luminance + bloom
activation(entity_id PK→entity, value, threshold, last_fired_at)

-- input | hidden_N | output
layer_assignment(entity_id PK→entity, layer)
```

**Polymorphism in practice.** The *same* entity may simultaneously hold a `crm_facet`, a
`ledger_facet`, **and** a `knowledge_facet`. The active **layer/lens** foregrounds one:

- the entity in the **Output** layer surfaces finalized `ledger_facet` totals;
- the *same* entity in a **Hidden** layer surfaces its `crm_facet` tracking;
- referenced from the knowledge graph, it surfaces its `knowledge_facet` body.

> **Role = composition × current layer.** No duplicate records, no type-switch branching —
> the behavior the owner described falls out of the model for free.

### 3.2 Mapping the three domains onto the ANN topology

| ANN region | Position | Domain content |
|---|---|---|
| **Input layer** | left | Raw inputs: incoming ledger items, calendar logs, raw text snippets |
| **Hidden layers** | center | Active client tracking (`crm_facet`) + backlinked knowledge (`knowledge_facet`) — the densely interconnected core |
| **Output layer** | right | Aggregated totals, met targets, finalized financials (`ledger_facet` roll-ups) |

- **Layout = Sugiyama layered graph drawing.** `x` is assigned from the layer (Input → Hidden
  → Output, left to right). `y` comes from a median/barycenter ordering pass that minimizes
  edge crossings — the standard, well-understood algorithm for layered DAGs, and a perfect fit
  for an ANN metaphor.
- **Activation states.** Business triggers write `activation.value ∈ [0,1]`; the renderer maps
  it to **node scale + emissive intensity**, so high-priority/soon-to-renew/recently-recalled
  neurons visibly **swell and glow** (and feed the bloom pass).
- **Action potentials.** A daily operation "fires" an edge: GPU particles spawn at `source_id`
  and travel the **straight** pathway to `target_id` at a speed/intensity scaled by
  `edge.weight`; on arrival the target's `activation.value` is bumped — visualizing data flow
  updating a destination node.
- **Knowledge backlinks.** Bi-directional links are `edge` rows; a node's backlinks are simply
  `SELECT … FROM edge WHERE target_id = ?`. No separate link store.
- **Financial roll-ups.** SQL **views / materialized aggregate tables** compute pacing, tax
  buckets, invoice status, and totals that drive the **Output-layer** nodes. (DuckDB is the
  documented escalation only if this ever outgrows SQLite.)

### 3.3 "Sync" in a local-first app

There is **no network sync** — that's the point. "Sync" here means **in-process consistency**
between the SQLite source of truth and the live ECS render world: a repository layer loads
entities + facets + edges into ECS at startup, and a background writer persists changes back
over a channel (WAL mode, batched). Portability is achieved by **file movement**, not
replication: copy the encrypted `.db`, `VACUUM INTO` for a compact export, or emit the JSON
snapshot for a human-readable backup.

---

## 4. Phased Implementation Strategy (single-sprint MVP)

**Sprint goal:** a functional **core visualization** MVP — 500 nodes / 1,200 edges rendered
from a local SQLite seed, with activation glow and action-potential sparks, holding **60+ FPS**
on the target machine.

| Phase | Deliverable |
|---|---|
| **P0 — Scaffold** | Rust workspace; Bevy app; window + DX12 device up; ECS world; on-screen **frame-time HUD**. |
| **P1 — Data engine** | SQLite schema (§3.1); `rusqlite` + SQLCipher; seed/import; load entities + facets + edges → ECS. |
| **P2 — Core render** | Instanced node spheres + instanced **straight** edges; orbit/pan/zoom camera; **Sugiyama layered ANN layout** computed from the DB. |
| **P3 — Activation visuals** | Emissive scale/luminance from `activation.value`; **HDR target + Bloom**. |
| **P4 — Action potentials** | Compute-shader particle system firing sparks along edges; activation propagation on arrival. **← end of the single-sprint MVP.** |
| **P5 — Domain CRUD** *(follow-on)* | `bevy_egui` panels for CRM/ledger/knowledge facets; backlink panel; financial aggregation views feeding Output nodes. |
| **P6 — Hardening** *(follow-on)* | Encryption passphrase flow (Argon2id); portable export/import (binary + JSON); multi-hour **soak test** proving flat RSS/VRAM and sustained 60+ FPS at full scale. |

**Indicative crates** (for the implementation phase; none installed in this proposal):
`bevy`, `bevy_egui`, `rusqlite` (`bundled-sqlcipher`), `glam`, `bytemuck`,
`serde` / `serde_json`, `argon2`.

---

## 5. Decision Log

| Decision | Choice | Primary reason |
|---|---|---|
| App shell | **Bevy (native Rust)** | No GC, instant start, ECS == data model; alternative raw `wgpu`+`winit`+`egui` noted below |
| Graphics API | **`wgpu` → DX12** | Compute shaders + explicit memory + deterministic frames on the 5080 |
| Data store | **SQLite + SQLCipher** | Single portable file, AES-256 at rest, zero cloud |
| Data model | **ECS / facet tables** | Single entity behaves as client/line-item/knowledge by composition |
| Edge geometry | **Straight instanced quads** | Matches spec; avoids curve tessellation cost |
| Particles | **GPU compute ring buffer** | Million-particle headroom; CPU only enqueues fire events |
| Bloom | **Dual-filter HDR chain (Bevy `Bloom`)** | ~0.5–1 ms at 4K; activation drives glow |
| Leak strategy | **Rust RAII + pre-allocation + soak test** | Flat RSS/VRAM guaranteed, validated |

### Bevy vs. raw `wgpu` — the one decision worth restating

**Bevy is recommended.** Raw `wgpu` + `winit` + `egui` is the "maximum-control" alternative
(absolute control over every pipeline and the render graph), but at 500 nodes / 1,200 edges it
buys **no measurable headroom** over what Bevy already delivers, while costing weeks of
boilerplate (hand-built ECS, hand-built HDR/bloom chain, hand-managed render passes) that would
not fit the single-sprint MVP. Bevy gives native `wgpu`→DX12 performance, a render graph open
enough for the custom compute-particle node, a built-in bloom pipeline, and — most importantly
— an ECS whose component model *is* the schema in §3. Choose raw `wgpu` only if a future
requirement demands pipeline-level control Bevy's render graph cannot express; nothing in the
current spec does.
