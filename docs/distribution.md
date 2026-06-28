# Distribution — how the work leaves the bench

How Obelisk reaches a hand that isn't ours. Companion to [METHOD.md](../METHOD.md)
(how the work *flows*) and [STYLE.md](../STYLE.md) (how the code *reads*); this says
how the work *ships*.

One principle, held against drift:

## The artifact is one self-contained file

The shippable thing is a **single executable with every asset baked in** — glyphs,
lore, palette, all of it compiled in via `include_bytes!` / `include_str!`, the way
`lore/voice.txt` already is. No loose `assets/` folder, no data file beside the exe,
nothing to misplace, swap, or lose.

*Why:*

- **It cannot come apart.** A folder of assets is a folder that arrives missing one
  file. A single object either runs or doesn't; there is no half-state.
- **It is its own integrity.** The work ships as one indivisible thing — the nearest
  honest form of "this is the piece, whole." Nothing to pull apart, nothing to
  substitute. The binary *is* the game.
- **It sidesteps the host.** Baked pixels are our pixels (see the glyph note below):
  no font file, no system lookup, no per-machine font/asset resolution. Identical
  bytes render identical light everywhere.

### Glyphs are baked, never a font file

Type is **our own pixel data** — `const` byte arrays compiled in — not a `.ttf` the
host interprets. A font file invites hinting, antialiasing, and fallback differences
across machines; baked pixels have none of that surface. The cost is labor (we draw
the glyphs), not compatibility. This is *why* the single-file artifact is also the
most portable one.

## The installer is optional gift-wrap, never the artifact

A Windows installer (`cargo-wix`/MSI, or NSIS) may later wrap that same exe for a
Start-menu entry and an uninstaller. It changes **nothing** about the binary — it
only places it. So the installer is a finishing step taken when we want the
professional feel, not a fork to decide up front. **Build for the single exe first;
the installer is always a later, additive pass.**

> Deferred, on purpose, so we don't forget: **make the installer** once the exe is
> something we'd hand out. Tracked in memory as well as here.
