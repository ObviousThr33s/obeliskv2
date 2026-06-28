//! Content — the parsers that turn baked data files into runtime form.
//!
//! The asset doctrine (see `docs/assets.md`): every asset is a human-authored data
//! file under `assets/`, baked into the binary (`include_str!`/`include_bytes!`),
//! and turned into runtime form by a pure parser here that errors, never panics.
//! Nothing is read from disk at runtime.

pub mod lore;
