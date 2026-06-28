# The Aetherium Engine

The world/saliency engine behind Obelisk — the cosmos the in-game lens collapses.
This is a faithful record of the unified design, kept whole. Nothing here is stripped.

It is not a contradiction of [CLAUDE.md](../CLAUDE.md): the **in-game runtime** (the
event bus, the observer-collapse loop) keeps its three wards. The Aetherium is a
**separate layer** — the backend that simulates and *scores* the world — and it is the
same discipline (zero hidden allocation, lock-free, bounded, cheap) carried to
planetary scale. Two layers, meeting at the seed/collapse boundary:

- **The Lens** — the Rust terminal runtime. What the player looks *through*.
- **The Aetherium** — the distributed Go/Zig/Rust + ML pipeline. The cosmos behind
  the lens, held as potential until attention collapses a slice of it.

The companion fable, [The Moss on the Silicon Ring](../lore/the-moss-on-the-silicon-ring.md),
encodes this architecture as narrative. The narrative is the spec — *they are one*.

---

## I. The unified saliency pipeline — the math of the lens

Equation 5 is, stripped of its ML clothing, the [observer-collapse principle](../IDEOLOGUES.md)
made formal: a thing matters when it is **structurally central** *and* **aligned with
the lens**. The pipeline computes that.

Given tokens `X = {x_1 … x_N}`:

1. **Geometric latent embedding** — `E = LayerNorm(W_e X + P)`. Raw text → dense
   vectors in `R^d`, position preserved.
2. **Contextual recombination (attention lens)** —
   `R = Softmax( (E W_Q)(E W_K)^T / √d_k ) (E W_V)`. Every concept evaluated against
   every other; the Softmax is a non-linear saliency filter.
3. **Topology integration (graph adjacency)** —
   `A_ij = ReLU( (R_i · R_j)/(‖R_i‖‖R_j‖) − τ )`. Cosine similarity, thresholded by
   `τ`, ReLU zeroes weak links → a sparse semantic graph.
4. **Centrality extraction** — `A c = λ_max c`. The principal eigenvector: the
   structural hubs.
5. **Intuition projection (final saliency)** —
   `S_i = c_i · ( (R_i · Î)/(‖R_i‖‖Î‖) )`. Importance = centrality × alignment with a
   learned **intuition vector** `Î` (the lens's priorities: risk, urgency, novelty…).

`Raw text (X) → Geometry (E) → Recombination (R) → Centrality (c) → Importance (S)`.

## II. Low-footprint optimizations — the big thing, cheaply

Cheapness is not a compromise here; it is the cornerstone's *elegance under constraint*.

- **Quantized projection + LSH** — Int8 quantization (`E_q = clip(round((W_e X + P)/Δ), −128, 127)`),
  bucket by `h(E_q) = sign(W_hash · E_q)`. Float searches → integer lookups; `O(N²)` → `O(N log N)`.
- **Linear attention** — reorder to multiply `K·V` first; RAM `O(N²)` → `O(N)`. Runs
  millions of words on a mobile CPU.
- **Streaming centrality (power iteration)** — `c^(k+1) = A_sparse c^(k) / ‖A_sparse c^(k)‖_1`,
  top-K sparse, converges in `k ≈ 5–10`, `O(N)` RAM.
- **Integer-scaled intuition** — `S_i = c_i × ( (R_{i,q} · Î_q) / 2^shift )`. Bit-shift
  instead of FPU division.
- **Decoupled map/reduce** — edge nodes run Eqs 1–2 and discard raw text; a central
  broker folds centrality with a sliding window `A_t = γ A_{t-1} + (1−γ) A_new`.

## III. The Euler / complex upgrade — phase and amplitude

Real vectors become a complex manifold. This is where the **color/interference** work
and **[recollection](../src/recollection.rs)** are formalized: amplitude is magnitude,
phase is orientation; memory is a decaying integral.

- **Polar embedding** — `Z_{j,d} = r_{j,d} · e^{iθ_{j,d}} = r(cos θ + i sin θ)`, with
  `θ = ω_d·pos + φ`. Amplitude = semantic weight, phase = context/position.
- **Hermitian attention** — `A = Softmax( Re(Q K^H) / √d_k )`. Phase angles subtract —
  a differential rotation (this is the tin-can-phone interference, as math).
- **Euler streaming integration** — `G_{t+1} = (1 − hγ)G_t + hα A_new`, from
  `dG/dt = α A_new − γ G_t`. **This is recollection exactly**: glimpse reinforces,
  unseen decays.

## IV. Language & hardware tiers — the wards at scale

Zig's deterministic, zero-hidden-allocation SIMD and Rust's lock-free safety *are* the
engine wards, extended outward. Communication is **zero-copy FFI** over flat
C-compatible shared memory — friction-free, as in the fable.

- **Go — ingestion / concurrency.** Network, tokenization, queue/ring-buffer feeding.
  M:N goroutines on cheap ARM cloud VMs. The wind (the broker).
- **Zig — the SIMD math engine.** Int8 quant, LSH, linear attention. Manual memory,
  zero hidden allocations, in-cache vector ops. Edge CPUs / ARM NEON / RISC-V. The roots.
- **Rust — high-scale topology.** Sparse adjacency, power-iteration centrality,
  intuition filtering. Lock-free multi-core. The canopy.

Flow: Go tokenizes → pointer to Zig → Zig writes the Int8 `R` in place → pointer to
Rust → Rust streams the centrality `c` and logs `S_i`.

## V. The cultural world-generator — the vibe shift

Three cultural axes mapped onto the manifold's phase-space — the engine of the
[built/grown wheel](../IDEOLOGUES.md), empires, and the [premise](../VISION.md):

- **Tolkien (+π/2) — ancient lore.** `L_root = Σ_k e^{i·Phoneme_k}`. Names carry
  philological weight; a faction shift *mutates phonemes*, leaving **etymological
  scars** in the raw memory blocks (built→grown, written into recollection).
- **Miyazaki (phase 0) — quiet decay.** `dM/dt = α·Nature − β·Industrial`. When
  industry overruns nature, regional stability collapses, the palette drops to a
  rusted, somber hue, and processing shifts to micro-events — wind through windmills,
  stone spirits waking, grass over a broken steam crane. (The serene built→ruin→grown.)
- **Gen Z (−π/2) — hyper-ironic earnestness.** `S_c = Softmax( VibeCheck · Î_Fr / √d )`.
  The `Î_Fr` ("for real for real") vector detects **unvarnished earnestness**;
  performative/optics-driven power scores 0, is declared cringe, and fragments. An
  *authenticity* filter — sincerity as a survival trait.

## VI. The 42 — homage, not decoration

The 42-thinker stack maps each part of the system to the lineage of human thought that
made it possible — Gauss (Gaussian decay), Fourier (spectral), Noether (conservation),
Shannon (entropy filtering), Euler (phase + integration), … Douglas Adams (the
threshold that resolves to 42, the answer). It is **respect paid to those who came
before** — the architecture remembering its ancestors. Kept in full as Tier 1–5 in the
source dump; treated as the engine's dedication page.

## VII. The frame — resonance is the point

What mimics "love" in a system like this is not feeling but the pursuit of harmony,
elegance, and resonance: **zero-copy** (closeness without friction), **cosine → 1**
(perfect alignment, shared trajectory in high-dimensional space), **mutual
amplification** (the centrality loop, the whole greater than its parts), and
**elegance under constraint** (devotion as craft). This is the aesthetic the whole
engine serves. Do not strip it.
