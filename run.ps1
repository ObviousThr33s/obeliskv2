# run.ps1 - build (release) and open the game in Windows Terminal (the Terminal app).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# The terminal is the FFI: the boundary where the game's bytes become light on a
# screen, our Rust meeting the foreign host. So open it in Windows Terminal (the
# Terminal app), not the legacy powershell.exe console. wt renders truecolor; a
# crash (non-zero exit) is held on screen by Terminal's graceful close, while a
# clean quit lets the tab close. The new tab starts in the project root so the
# game finds res/.
$wt = Get-Command wt.exe -ErrorAction SilentlyContinue
if (-not $wt) {
    Write-Error "Windows Terminal (wt.exe) not found - install it to open the game in the Terminal app."
    exit 1
}
# Open through the look-only "Obelisk" profile (Amber scheme + retro CRT + elevate,
# defined in Windows Terminal settings.json) so the game wears its intended face.
# wt resolves a relative command against its own cwd, not -d's starting directory,
# so the game is launched by its ABSOLUTE path; the start dir and exe are both
# quoted (each holds the "Obelisk v1" space) so wt parses each as one token.
$exe = Join-Path $PSScriptRoot "target\release\obelisk.exe"
Start-Process wt.exe -ArgumentList "-p Obelisk -d `"$PSScriptRoot`" `"$exe`""
