# The Neural Business Engine — native engine (`engine/`)

Native Rust implementation of the architecture in
[`../docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md`](../docs/ARCHITECTURE_NEURAL_BUSINESS_ENGINE.md),
following the phased plan in [`../docs/BUILD_PLAN.md`](../docs/BUILD_PLAN.md).

**Stack:** Rust · Bevy (`wgpu` → DirectX 12 on Windows) · SQLite + SQLCipher (later phases).
**Target:** RTX 5080 / Ryzen 9 7700X / 64 GB / Windows.

## Workspace layout

```
engine/
  Cargo.toml            workspace (Bevy pinned in [workspace.dependencies])
  crates/
    nbe_app/            P0 — Bevy shell: window, DX12, frame-time HUD, orbit camera
    # nbe_data/         P1 — SQLite + SQLCipher data engine (added next)
    # nbe_layout/       P2 — Sugiyama layered ANN layout (added next)
```

## Run (Windows workstation)

```powershell
cd engine
cargo run -p nbe_app --release
```

A 1600×900 window opens with a placeholder grid and a frame-time HUD (top-left).
**Left-drag** orbits, **scroll** zooms. On startup `bevy_render` logs the selected GPU,
which on the target machine reads approximately:

```
AdapterInfo { name: "NVIDIA GeForce RTX 5080", backend: Dx12, device_type: DiscreteGpu, .. }
```

That log line + a vsync-locked HUD is the **P0 acceptance gate**.

## Build on Linux (CI / this dev container)

The code is cross-platform; on Linux `wgpu` selects Vulkan instead of DX12. Bevy's default
features need a few system libraries:

```bash
sudo apt-get install -y libasound2-dev libudev-dev libx11-dev libxkbcommon-dev libwayland-dev
cargo check --manifest-path engine/Cargo.toml
```

A headless container can `cargo check`/`cargo test` but cannot open a window or drive a GPU —
the visual P0 gates are verified on the Windows workstation.
