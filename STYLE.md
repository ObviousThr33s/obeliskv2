# Obelisk — Style & Engineering Standard

A small, enforced house standard in the spirit of the **JSF AV C++** rules: every
rule carries a *rationale*, rules are *tiered* by how hard they bite, and we
*program in a safe subset* — banning the footguns rather than trusting discipline.

Adapted for **hosted Rust**: we keep JSF's structure (rationale + tiers + subset)
and reject its avionics content. The heap is a structural requirement, not a
luxury. So the rule is *invert JSF's instinct* — ban the footguns Rust left in,
keep the power Rust added.

Most of this is machine-checked (`Cargo.toml` `[lints]`). The prose below is only
for what the tools can't see.

## Tiers

Rust's lint levels **are** the tiers — the compiler is the verification process
JSF had to run by hand.

| Tier | Mechanism | Meaning |
|------|-----------|---------|
| **Shall** | `deny` / `forbid` | The build fails. Deviation is a reviewed, explicit `#[allow]`. |
| **Will** | `warn` | Visible every compile. Fixed at leisure. |
| **Should** | this document | What tools can't check; held by review. |

**The ratchet:** the policy currently sits almost entirely at `warn`, so the build
stays green while the code is cleaned. A lint graduates to `deny` once the code
that violates it is fixed. The one rule already at full strength is
`unsafe_code = "forbid"` — there's nothing to clean, and there never should be.

## The safe subset

Ban the footguns; keep iterators, closures, and traits.

- **No `unwrap` / `expect` / `panic!`** — the engine should degrade, not crash.
  Return `Result`/`Option` and handle it. *Carve-out: tests may `unwrap`/`expect`
  — a failed `expect` is the assertion.*
- **No raw indexing** (`buf[i]`) — it panics on a bad index. Use `.get(i)` and
  handle `None`.
- **No lossy casts** — use checked conversions, or an `#[allow]` with a one-line
  proof at the math boundary.
- **Borrow before you clone** — pass `&T` / `&str`; clone only when ownership
  genuinely demands it.
- **No `unsafe`** — only ever for FFI or memory-mapped hardware, neither of which
  exists here.

## Memory & allocation

- `std` and the heap are expected — `Vec`/`String`/`HashMap` are how state is held.
- **Don't allocate in hot loops.** The tick runs constantly; rebuilding a buffer
  every frame is the thing to avoid.

## Errors

- Fallible functions return `Result<T, E>`.
- `E` is a **typed enum** for domain logic — never a `String` or `Box<dyn Error>`.
- Handle every `Option`/`Result` via `match` or `?`. No silent discards.

## Naming & API shape

- `impl Display`, not a hand-rolled `impl ToString`.
- Getters borrow and drop the `get_` prefix: `version(&self)`, not `get_version(self)`.
- New names follow Rust convention (`CamelCase` types, `snake_case` items).

## Tests as documentation

- Name tests as behaviour sentences — `walking_up_to_the_moth_brings_you_adjacent`.
- Keep them **truthful**: no stale or commented-out scaffolding.
- Use **doc-tests** for public API — executable documentation.

## Formatting

- `rustfmt` owns layout.

## Running it

```sh
cargo clippy        # the safe subset + quality lints
cargo test          # unit tests + doc-tests
```
