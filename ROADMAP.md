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

- Event bus (`haps`), field, vision, recollection, render projection. — *Lens core, green.*
- Fountain safe spot: graded **aura field** (one number, seen and felt), **breathing**,
  the moth greeting you at boot. — *Lens, green.*
- The vision crystallized: [VISION.md](VISION.md), [IDEOLOGUES.md](IDEOLOGUES.md).
- The engine design captured whole: [docs/aetherium-engine.md](docs/aetherium-engine.md),
  the fable [lore/the-moss-on-the-silicon-ring.md](lore/the-moss-on-the-silicon-ring.md).

## Phase A — finish the Lens (the v3.1 line)

- **A1 — the second view.** First-person raycaster over the *same* `Field` / `aura_at`.
  Two views, one truth. (Finish line: the raycaster renders the same world the
  top-down does.)
- **A2 — the avatar with its own mind.** `intent.bend()` — the player *influences* a
  disposition, not a puppet ([memory: avatar-intent-seed]). Headless-testable first.
- **A3 — color/phase as a channel.** "Color is faster than refresh" — encode state in
  hue/phase (the interference work), the cheap fast channel. Fold into both views.

## Phase B — stand up the Aetherium spine

- **B1 — the 3-tier skeleton.** Go ingest → Zig SIMD math → Rust graph, joined by
  **zero-copy FFI** over flat shared memory. No serialization.
- **B2 — the saliency pipeline (Eqs 1–5).** Embedding → attention → sparse graph →
  power-iteration centrality → intuition projection `S_i`. The formal lens.
- **B3 — low-footprint.** Int8 + LSH, linear attention, streaming power iteration,
  sliding-window decay. The big thing, cheaply.

## Phase C — the Euler / complex upgrade

- **C1 — phase + amplitude.** Polar embeddings, Hermitian attention. The math of the
  interference/color work.
- **C2 — Euler streaming integration.** `G_{t+1} = (1−hγ)G_t + hα A_new` — recollection
  as a continuous integral. The Lens's memory and the Aetherium's memory become one law.

## Phase D — the cultural world-generator

- **D1 — Tolkien layer.** Phoneme-root names; faction shifts mutate them, leaving
  etymological scars in memory.
- **D2 — Miyazaki layer.** `dM/dt = α·Nature − β·Industrial`; palette decay, micro-events,
  nature reclaiming the machine. The built→ruin→grown wheel, serene.
- **D3 — Gen Z layer.** `Î_Fr` earnestness filter; performative power fragments. Authenticity
  as survival. Faction clout over time-forward epochs.

## Phase E — the dedication

- **E1 — the 42.** Weave the homage through the system as its dedication page — each
  part naming the ancestor whose thought made it possible.
- **E2 — the frame.** Hold resonance/love (zero friction, cosine → 1, mutual
  amplification, elegance under constraint) as the standard every piece is judged by.

## 4.0 — one experience

The Lens collapses the Aetherium's living cosmos: you look, and a culturally coherent,
time-advancing world resolves around you; you turn away, and it returns to potential
and memory. One window, one verb — *influence* — across both layers.

**Open question for you (the finish line is yours to name):** what is the single,
testable condition that means *4.0 is done*? Until it's named, 4.0 stays a direction,
not a branch.
