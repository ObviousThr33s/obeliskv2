# Obelisk — Ideologues

Parked ideas. Per [METHOD.md](METHOD.md): an idea waits here until *it* can name
its own finish line. Nothing here is rejected; nothing here is begun. The doc is
a librarian, not a judge.

## The organizing principle — attention collapses reality

Everything below is one idea at different radii. **The world is a ghost everywhere
the lens isn't.** Unobserved, a thing is a `u64` seed + rules (≈free). Observed, it
is derived/grown and frozen into facts in the [`Field`](src/field.rs). Left behind,
it fades to a remembered summary in [`Recollection`](src/recollection.rs), then back
toward seed. The ghost moth is this loop at entity scale; a clearing, a region, a
planet, an era are the same loop at larger radius. This is *why* the cosmos is
cheap (only the observed shell is ever real) and *why* the engine is calm (the world
becomes real around you and softens behind you). The wards exist to make exactly
this honest. The first buildable step never changes: generalize
[`World::tick`](src/world.rs)'s observe→react loop from entity to place.

## Render layers above the rigid grid

The world model stays a discrete grid of facts — rigid on purpose, because that
rigidity is what the wards in [CLAUDE.md](CLAUDE.md) rest on. Fluidity, depth and
atmosphere are *presentation* laid over the truth, never mixed into it. Each is a
future finish line.

- **Spatial prose (Lore).** Extend [`Lore`](src/lore.rs) so the world describes
  itself in language keyed to place, facing and depth — not just events. A
  text-forward engine evokes a 3D world through what it *says* as much as what it
  draws. Cheapest of the four; content in files, no render dependencies.

- **Motion interpolation.** Ease discrete moves across sub-cell space at render
  time. The world still only knows cell→cell; the screen glides. The seed already
  exists: `clarity` is a continuous `f32`, not a flag. Pure Rust, headless-testable
  (the interpolation function), no window required.

- **The raycaster (first-person 3D).** Cast over the same rigid grid for real
  depth and parallax, without the world ever holding 3D data. The roadmap's
  first-person view. The heaviest structural add — best once motion and prose
  exist to fill it.

- **Atmosphere (the mastering pass).** Glow/bloom, dithered gradients, distance
  fog — the pass that dissolves hard cell edges into a fluid image. Depends on the
  framebuffer/window slices, so it comes later by dependency, not by importance.

## Built vs. grown places — *active exploration*

Two ways a place comes to be, and the difference between them is the live design
question (not yet a finish line):

- **Built** — larger rigid sections, authored, "a magic cast over them." Placed
  with intention; frozen unless an event changes them. Provenance: a hand or a
  template. Meaning: narrative, deliberate.
- **Grown** — paths, growth, things that arise from a simulated ecosystem.
  Provenance: local rules run over iterations. Meaning: emergent, systemic — it
  makes sense *because of its conditions*.

The cheap reconciliation (so it honours the wards): **simulate growth once, at
generation time, then freeze the result into facts** in the [`Field`](src/field.rs).
Live per-tick ecosystem simulation is the expensive path through the event bus —
parked until something needs a place to keep changing while watched. See
[[renderer-direction]] and the derivation-not-simulation note from the planet
discussion.

### The wheel — lifecycle, agency, attention

Built and grown are not a binary but a **wheel**. The same axis runs both ways:

- **Built → ruin** — authored structure abandoned, reclaimed by growth.
- **Grown → empire** — emergent activity tended, accreted, crystallized into order.

Ruins and empires are the wheel turning opposite ways: **time / neglect** turns it
down toward the wild, **agency** turns it up toward order. That agency has two
registers — **kings** impose (author, decree, cast the magic; few, top-down) and
**farmers** tend (work *with* the ecosystem's rules; many, bottom-up). A power law:
mostly farmers, vanishingly few kings. The ratio *is* the serenity.

Build the few (landmarks, the obelisk); grow the matrix they sit in (land, paths,
the wild). Don't decide ruin-or-empire per place — decide where on the wheel the
world *starts*, and let the surroundings imply the trajectory.

The third axis is **attention**: a walk and an adventure are the same path through
different lenses. The serene rule (target feel = serenity): **don't author
adventures. Grow walks, and let the lens confer adventure** at the point of
looking — the ghost-moth principle generalized. The player's attention sets the
ratio, not the level designer.

Cheap / ward discipline: **simulate none of this live.** Generate the *evidence* of
these trajectories — ruins and empires as frozen gen-time states, king/farmer as
roles baked into entities — and let the lens supply the life. History implied, not
simulated (same as the planet). See [[built-vs-grown-generation]].
