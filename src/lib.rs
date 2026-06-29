//! Obelisk — the Lens. Mechanism only; which creature it is, is content.
//!
//! Three trunks the binaries (the leaves) grow from:
//! - [`world`] — the mechanism: the event bus and everything placed in the field.
//! - [`render`] — the lens: projecting the world into a grid of coloured cells.
//! - [`content`] — the parsers that turn baked data files into runtime form.
//!
//! Each binary (`obelisk`, `pane`, `lens`) is a thin leaf: it grows from these
//! trunks and branches only at *input* (keys → intent) and *output* (a `Frame` to
//! its medium). The Aetherium backend is a separate project (see
//! `docs/aetherium-spinoff.md`).

pub mod world;
pub mod render;
pub mod content;
/// The terminal front-end shared by the `obelisk` and `obelisk_debug` leaves.
pub mod terminal;
