# The Aetherium — spin-off brief

**This is a separate project from Obelisk.** Obelisk (this repo) is *the Lens* — the
in-game runtime you look *through*. The Aetherium is *the cosmos behind the lens* — the
distributed saliency backend (Go/Zig/Rust + ML). They are two projects that meet at one
boundary; keeping them apart keeps each simple.

Parked here so it can be lifted out cleanly when you regroup it into its own repo.

## The boundary (the only coupling)

`seed / collapse`. The Aetherium holds the world as potential and *scores* it for
importance; the Lens *collapses* a slice of it on attention. The Lens **consumes** scored
saliency; the Aetherium **produces** it. Communication is zero-copy over flat shared
memory — no other coupling.

## What moves to the new project
- `docs/aetherium-engine.md` — the full design (the spec; copy it over as the new repo's core).
- `lore/the-moss-on-the-silicon-ring.md` — the companion fable (the narrative *is* the spec).
- ROADMAP phases that are Aetherium, not Lens: **B** (the 3-tier spine), **C** (Euler/complex),
  **D** (cultural world-generator). They leave Obelisk's roadmap and become the new project's.

## What stays in Obelisk (the Lens)
- All of `src/` — world (mechanism), render, content. The event bus and its three wards.
- Phase **A** (finish the Lens) and the **polygon** work. The renderer.

## Status
Parked, on purpose. Regroup when ready — nothing in the Lens depends on the Aetherium
existing yet; the boundary above is where the two will reconnect.
