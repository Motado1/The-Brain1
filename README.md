# The Brain — Neural Business Engine

A private, **local-first** desktop app for a personal-training business. A single (optionally
encrypted) SQLite file is both the data store and a living **"Brain"** — a network of neurons that
fire and pass signals, with three regions for the business: CRM/clients, Research/knowledge, and
the Financial ledger. It's driven through a CLI hub and a 3D renderer.

## The active project lives in [`engine/`](engine/)

Everything current is the Rust workspace under `engine/` (Bevy + SQLite). Start there:

- **Orientation for contributors / AI sessions:** [`CLAUDE.md`](CLAUDE.md)
- **Living status & roadmap:** [`docs/STATUS.md`](docs/STATUS.md)
- **Architecture & plan:** [`docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`](docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md), [`docs/BUILD_PLAN.md`](docs/BUILD_PLAN.md)

```bash
cd engine
cargo test -p nbe_cli                              # tested business/data logic
cargo run -p nbe_cli -- --db brain.db today        # CLI hub
cargo run -p nbe_app --release -- --db brain.db     # 3D renderer (needs a GPU)
```

## [`legacy/`](legacy/) — archived web prototype

An earlier Next.js / Supabase / RAG web prototype lives in `legacy/`. It is **not maintained** and
is kept only for reference. New work does not touch it.
