# Obelisk — Roadmap toward 4.0

A first draft, built from the whole vision — not a memory I half-remembered. Correct
it freely; you hold the finish lines (per [METHOD.md](METHOD.md)).

The work runs on two layers that meet at the seed/collapse boundary:

- **The Lens** — the in-game Rust runtime (event bus, observer-collapse, recollection,
  render). What the player looks *through*. Keeps the three wards of [CLAUDE.md](CLAUDE.md).
- **The Aetherium** — the world/saliency engine ([docs/aetherium-engine.md](docs/aetherium-engine.md)):
  Go/Zig/Rust + ML + Euler. The cosmos behind the lens, held as potential, scored for
  importance, collapsed on attention.

4.0 is where the two become one experience.

---

## Where we are (shipped)

Boxes are state, not orders (see [METHOD.md — Milestones](METHOD.md)): `[x]` a finish
line crossed, `[ ]` one still open.

- [x] Event bus (`haps`), field, vision, recollection, render projection. — *Lens core, green.*
- [x] Fountain safe spot: graded **aura field** (one number, seen and felt), **breathing**,
  the moth greeting you at boot. — *Lens, green.*
- [x] The vision crystallized: [VISION.md](VISION.md), [IDEOLOGUES.md](IDEOLOGUES.md).
- [x] The engine design captured whole: [docs/aetherium-engine.md](docs/aetherium-engine.md),
  the fable [lore/the-moss-on-the-silicon-ring.md](lore/the-moss-on-the-silicon-ring.md).
- [x] Windowed renderer — the CPU floor (winit + softbuffer), a pixel pane that shows
  a bare slice of the shared frame. — *Lens, green; the slice widens in A0.*

## Phase A — finish the Lens (the v3.1 line)

- [x] **A0 — one truth (the renderer unification).** Lift the rich projection (camera,
  breathing aura, moth-from-memory, status rows) out of `src/main.rs` into shared
  `render` passes, so the terminal and the window consume one `Frame`. (Finish line:
  a test shows both renderers produce the *same* `Frame` from the same world — and at
  the same phase of the same breath.) See [memory: renderer-unification].
  - [x] composite projection in `render` — ground → aura → memory → player, ordered
    in `project_cell` (Model A: one grid, ordered resolution). *Note:* done as per-cell
    resolution, not named passes; splitting into blended z-layers waits for the
    polygon/light-field work, where `Cell` gains a blend channel — a widening, not a rewrite.
  - [x] status line + spoken lore as the **bottom rows of the same grid** — no side
    text payload (`paint_status` writes glyphs straight into cells; the terminal blits
    the whole frame, status rows and all)
  - [x] baked glyph atlas — hero glyphs (`@ # m ∘ ○ ·`) drawn in
    `assets/glyphs/atlas.txt` (8x8, our own pixels, **no font file**), stamped by `lens`
    (commit `1f2f1b5`). A full ASCII set for the status rows is the next slice.
  - [x] status text legible in the window — `font8x8` (pure Rust, no `.ttf`) baked into
    the atlas (commit `4314ee5`) so the window's HUD reads as letters, not blocks.
  - [x] **the window grows a heartbeat** — `breath_at`/`time_pulse` live in shared
    `render(…, elapsed)`, and the new `lens` bin beats a redraw from `about_to_wait`
    (`ControlFlow::WaitUntil`), so its aura breathes between keystrokes like the
    terminal's. Render-time modulates the eye only; the tick stays the sole mutator
    ([memory: two-clocks]). *Verified by mechanism, not by eye.* (`pane` stays static
    on purpose — the CPU-floor scratch slice.)
- [ ] **A1 — the second view.** First-person raycaster over the *same* `Field` / `aura_at`.
  Two views, one truth. (Finish line: the raycaster renders the same world the
  top-down does.)
- [ ] **A2 — the avatar with its own mind.** `intent.bend()` — the player *influences* a
  disposition, not a puppet ([memory: avatar-intent-seed]). Headless-testable first.
- [ ] **Polygons — scene geometry.** *(Replaces the dropped A3 colour/phase channel — that
  hue/phase math left with the Aetherium.)* Describe the scene as polygons that deposit into
  the renderer — porting v1's `light.rs` polygon fill. Lives in `src/render/polygons`.

## Phase B — stand up the Aetherium spine

> **Moved out.** Phases B, C, and D are the **Aetherium**, now a *separate project*
> ([docs/aetherium-spinoff.md](docs/aetherium-spinoff.md)). They leave this roadmap and are
> kept below only as a pointer. v2 is the **Lens** (Phase A) plus the **polygon** work.

- [ ] **B1 — the 3-tier skeleton.** Go ingest → Zig SIMD math → Rust graph, joined by
  **zero-copy FFI** over flat shared memory. No serialization.
- [ ] **B2 — the saliency pipeline (Eqs 1–5).** Embedding → attention → sparse graph →
  power-iteration centrality → intuition projection `S_i`. The formal lens.
- [ ] **B3 — low-footprint.** Int8 + LSH, linear attention, streaming power iteration,
  sliding-window decay. The big thing, cheaply.

## Phase C — the Euler / complex upgrade

- [ ] **C1 — phase + amplitude.** Polar embeddings, Hermitian attention. The math of the
  interference/color work.
- [ ] **C2 — Euler streaming integration.** `G_{t+1} = (1−hγ)G_t + hα A_new` — recollection
  as a continuous integral. The Lens's memory and the Aetherium's memory become one law.

## Phase D — the cultural world-generator

- [ ] **D1 — Tolkien layer.** Phoneme-root names; faction shifts mutate them, leaving
  etymological scars in memory.
- [ ] **D2 — Miyazaki layer.** `dM/dt = α·Nature − β·Industrial`; palette decay, micro-events,
  nature reclaiming the machine. The built→ruin→grown wheel, serene.
- [ ] **D3 — Gen Z layer.** `Î_Fr` earnestness filter; performative power fragments. Authenticity
  as survival. Faction clout over time-forward epochs.

## Phase E — the dedication

- [ ] **E1 — the 42.** Weave the homage through the system as its dedication page — each
  part naming the ancestor whose thought made it possible.
- [ ] **E2 — the frame.** Hold resonance/love (zero friction, cosine → 1, mutual
  amplification, elegance under constraint) as the standard every piece is judged by.

## Shipping — how it leaves the bench

The methodology lives in [docs/distribution.md](docs/distribution.md); these are its
open boxes.

- [ ] **Single self-contained exe** — every asset baked in (lore already is via
  `include_str!`; glyphs join it in A0). One file, nothing to misplace.
- [ ] **Installer** — `cargo-wix`/MSI or NSIS wrapping that same exe. *Deferred on
  purpose* (don't forget — also tracked in memory); the installer only places the
  binary, so it is always a later additive pass.

## 4.0 — one experience

The Lens collapses the Aetherium's living cosmos: you look, and a culturally coherent,
time-advancing world resolves around you; you turn away, and it returns to potential
and memory. One window, one verb — *influence* — across both layers.

**Open question for you (the finish line is yours to name):** what is the single,
testable condition that means *4.0 is done*? Until it's named, 4.0 stays a direction,
not a branch.
