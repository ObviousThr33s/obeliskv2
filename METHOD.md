# Obelisk — Method

How an idea becomes a merge. Companion to **STYLE.md**: that one says how the
code should *look*; this one says how the work *flows*.

Constraint first: decide what *done* is before you start, and the work designs
itself in the space that's left.

## The finish line

Every branch is cut to cross one **finish line** — a single red test that says
what *done* means. That test is the merge condition: nothing more, nothing less.

*Why:* a branch with no defined done drifts. While it lives, `main` moves out
from under the commit it branched from, and the merge rots. A finish line keeps
the branch short, so the base barely moves and the merge stays clean. **No
finish line, no branch.**

## Ideas wait; they are not judged

An idea earns a branch only when *it* can name its own finish line. Until then
it waits — written down here, in a methodology doc (an *ideologue*), not held in
someone's head. An unready idea is parked, never rejected. The doc is a
librarian, not a judge.

## Base off green

Branch from a **green** `main` — a commit where `cargo test` passes. Branch off
a broken main and you inherit its red; you can't tell its bug from yours. Start
green, and any red you see is your own.

## Branch shape

| Size | Where it lands |
|------|----------------|
| Small / foundational | straight to `main` |
| Medium / large | its own branch |

Branches are a **flat run, not a deep stack.** A branch spawns at most *one*
sub-branch, and only when it's too big to finish in a single pass. The *design*
may be many levels deep; the *branches* never are. Design depth is not branch
depth.

## Milestones — checkboxes over finish lines

A milestone is a **named collection of finish lines**, shown as checkboxes. A box is
not a command; it is the *state* of a finish line — `[ ]` not-yet-crossed, `[x]`
crossed. Checking a box **reports a fact, it does not issue an order.**

This is what lets a roadmap generalize without *willing* the work into a fixed shape.
Written as a directive, "Phase A: finish the Lens" would decree a particular
implementation. Reframed as a checkbox over a finish line, it decrees nothing — it
names a *shape of done* and waits to be satisfied. The map stays a direction, never a
decree (see [ROADMAP.md](ROADMAP.md)).

### The box is polymorphic

One checkbox, many kinds of done. The criterion behind a box exposes a single
`done?` interface, so a milestone can mix kinds freely:

| Kind | `done?` is satisfied when |
|------|---------------------------|
| **Test** | a red test goes green (the default — see *The finish line*) |
| **Doc** | a methodology is written down (an ideologue committed) |
| **Artifact** | a thing ships (the exe builds; the installer wraps it) |
| **Milestone** | a sub-checklist is fully checked (a box of boxes) |

Because a box only ever asks `done?`, the *general* milestone structure carries no
will of its own — the will lives in the concrete finish line plugged into it.
**Generalize the tracker; keep each box's meaning local.** That is the polymorphism
that lets the whole thing be general without dictating any of its parts.

## The loop

Three touches from the person holding the idea:

1. **Name done** — write the finish line (the red test).
2. **Glance** — does the test read right?
3. **Merge** — green, nothing else crept in, **and the box checked** — but *only* the
   boxes truly crossed. A tracker that claims a finish line it never crossed is worse
   than no tracker; checking is reporting, not wishing (see *Milestones*).

Everything between *Name* and *Merge* is mechanism.
