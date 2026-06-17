# Windows setup

You only need to do this once.

## 1. Get the code (if you haven't)
Open **PowerShell** and run:
```powershell
winget install -e --id Git.Git          # skip if you already have git
git clone <your-repo-url> The-Brain1
cd The-Brain1
git checkout claude/neural-business-engine-arch-h7i8xj
```

## 2. Run the setup script
```powershell
powershell -ExecutionPolicy Bypass -File engine\scripts\setup-windows.ps1
```
This installs Rust, the C++ build tools, Perl and NASM (via `winget`), then builds
everything. The first build takes a while. **Re-running is safe.**

> If Windows asks for admin permission during the Visual Studio Build Tools install, say yes.
> If any step turns red, copy the error text and send it back.

## 3. Use it
From the `engine` folder:
```powershell
cargo run -p nbe_app --release              # the 3D window (placeholder for now)
.\target\release\nbe.exe --db brain.db --help   # the CLI hub (clients, packages, reports, calendar)
```
