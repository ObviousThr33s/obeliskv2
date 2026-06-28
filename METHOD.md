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

## The loop

Three touches from the person holding the idea:

1. **Name done** — write the finish line (the red test).
2. **Glance** — does the test read right?
3. **Merge** — green, and nothing else crept in.

Everything between *Name* and *Merge* is mechanism.
