# CLAUDE.md — Guidance for AI contributors

This file holds architectural law that the compiler can't enforce on its own.
[STYLE.md](STYLE.md) governs *how the code reads*; this governs *how the engine is
shaped*. When the two touch, both apply.

## Event bus — the core wards

The engine is built around an event bus. These three wards are **load-bearing**:
break one and the whole occlusion/timing model rots. Treat them as `Shall` (build
fails, in spirit) even where a lint can't yet catch the violation.

### 1. Zero-reference payloads

Events are **pure data**. A payload carries values and `u64` `EntityId`s — never a
pointer, reference, lifetime parameter, `Rc`/`Arc`, or `Arc<Mutex<T>>`.

*Rationale:* an event outlives the moment it was raised — it sits in a queue across
phases. A reference in the payload would tie that lifetime to whatever borrowed it,
forcing shared ownership and locks back into the hot path. Identity by `EntityId`
keeps events `Copy`-cheap, trivially queueable, and free of aliasing. The bus moves
*facts*, not handles to live state.

### 2. Lock-free bounded queue

The bus is backed by a **bounded, fixed-capacity ring buffer**. It is allocated
once. **Zero heap allocations are permitted in the hot loop** — no `Vec::push`
growth, no per-event boxing, no `String`.

*Rationale:* the tick runs constantly; an allocation per event is a per-frame tax
and a latency spike. A fixed ring sizes the worst case up front and makes overflow
an explicit, observable condition rather than silent unbounded growth.

### 3. Deferred dispatch — three strict phases

State mutation happens in **one direction, in three ordered phases per tick**:

1. **Read / Update** — systems read current state and *push events*. They mutate
   nothing shared.
2. **Dispatch** — the queue drains. Events are routed to receivers.
3. **Mutation** — receivers apply the queued changes to state.

No system mutates shared state during phase 1. No event is raised during phase 3.

*Rationale:* deferral is what makes occlusion and "enemies move when unwatched"
honest — every system observes the *same* snapshot of the world within a tick, and
order-of-update bugs (A sees B's half-applied change) become impossible by
construction. It also keeps the borrow checker happy without locks: reads and writes
never overlap in time.

## How to work here

- Read [STYLE.md](STYLE.md) before writing engine code — the safe subset (no
  `unwrap`/`panic!`, no raw indexing, typed-enum errors) is enforced.
- Tests are the spec. Name them as behaviour sentences; keep them truthful.
- Narrative/content lives in files; code is pure mechanism. Don't hardcode story.
- Propose before you build structural pieces. Restraint is a feature.
