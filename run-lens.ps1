# run-lens.ps1 - build (release) and open Obelisk, the lens: the windowed product.
# The lens owns its own pixels, so unlike run.ps1 (the terminal build) there is no
# Windows Terminal to host it - the exe IS the window. Always release: a debug build
# of the per-frame rasteriser is what makes it feel slow and clunky.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

cargo build --release --bin lens
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Started from the project root so the game finds its baked assets' siblings on disk.
$exe = Join-Path $PSScriptRoot "target\release\lens.exe"
Start-Process -FilePath $exe -WorkingDirectory $PSScriptRoot
