# Packaging & release (M5)

How to produce a shippable build of the renderer (`nbe_app`) for the Windows 11 / RTX 5080
workstation. Most of this is **run on the target OS** (the agent builds blind on headless Linux).

## Release build (optimized, stripped)
The workspace `[profile.release]` is tuned for the workstation: `opt-level=3`, `lto="thin"`,
`codegen-units=1`, `strip=true` (smaller, faster binary; debug symbols dropped). To build:

```powershell
cd engine
cargo build -p nbe_app --release
# binary: engine/target/release/nbe_app.exe   (run it directly; it loads ./assets next to it)
```
First release build is slow (LTO + single codegen unit); subsequent builds are incremental.

## Standalone app (name + icon, no terminal)
`cargo bundle` reads `[package.metadata.bundle]` in `crates/nbe_app/Cargo.toml`. Run it **on the OS
you're packaging for**:

```powershell
cargo install cargo-bundle           # once
cargo bundle --release -p nbe_app
```

### Add an icon
1. Drop a square icon at `engine/crates/nbe_app/assets/icon.png` (256×256 recommended; a `.ico` also
   works on Windows).
2. Uncomment the `icon = ["assets/icon.png"]` line in `crates/nbe_app/Cargo.toml`.
3. Re-run `cargo bundle --release -p nbe_app`.

> Windows note: `cargo bundle`'s Windows output is an MSI (needs WiX). If you just want a
> double-clickable `.exe` with an embedded icon (no installer), the lighter path is the `winresource`
> crate + a `build.rs` that embeds `assets/icon.ico` — say the word and we'll wire that instead.

## DB maintenance
Keep the SQLite file healthy (run while the app is closed):
```powershell
cargo run -p nbe_cli -- --db brain.db maintain
# → "maintenance: integrity ok; vacuumed + checkpointed"
```
`maintain` runs `PRAGMA integrity_check` (corruption canary) + `VACUUM` (defragment/shrink) +
`wal_checkpoint(TRUNCATE)`. Good to run periodically or before a backup.

## Verify on the GPU
After a release build, confirm wgpu picks **DX12** and the scene renders at the expected framerate
(the startup log prints the adapter + backend). Bloom/DoF cost is the main thing to eyeball on the
5080 — if needed, the bloom intensity / DoF aperture are in `scene.rs::spawn_camera`.
