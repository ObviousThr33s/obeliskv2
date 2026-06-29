//! The baked glyph atlas — our own pixels, never a shipped font (the asset
//! doctrine, see `docs/assets.md`).
//!
//! Each glyph is an 8x8 1-bit bitmap, authored as a `.`/`#` grid in
//! `assets/glyphs/atlas.txt` and baked in with `include_str!`. The windowed build
//! (`lens`) stamps these into pixels, scaled to its cell; the terminal needs none
//! of this, since it prints real characters.

use std::collections::HashMap;

/// A glyph is `GLYPH_H` rows of `GLYPH_W` bits; within a row, `0x80 >> x` is column `x`.
pub const GLYPH_W: usize = 8;
pub const GLYPH_H: usize = 8;

/// One glyph's pixels: one byte per row, the high bit the leftmost column.
pub type Glyph = [u8; GLYPH_H];

/// A map from character to its baked bitmap. Built once (parsed from the baked
/// file), then read per frame — no per-frame allocation (ward 2).
pub struct Atlas {
	glyphs: HashMap<char, Glyph>,
}

impl Atlas {
	/// The atlas baked into the binary from `assets/glyphs/atlas.txt`.
	pub fn baked() -> Self {
		Self::parse(include_str!("../../assets/glyphs/atlas.txt"))
	}

	/// Parse the `.`/`#` grid: `glyph <char>` opens a glyph, then up to `GLYPH_H`
	/// rows where `#` is a lit pixel. Blank lines and `;` comments are skipped. A
	/// malformed line is simply ignored — never a panic (the safe subset).
	pub fn parse(src: &str) -> Self {
		fn flush(c: Option<char>, rows: &mut Vec<u8>, glyphs: &mut HashMap<char, Glyph>) {
			if let Some(ch) = c {
				let mut g: Glyph = [0; GLYPH_H];
				for (slot, bits) in g.iter_mut().zip(rows.iter()) {
					*slot = *bits;
				}
				glyphs.insert(ch, g);
			}
			rows.clear();
		}

		let mut glyphs = HashMap::new();
		let mut current: Option<char> = None;
		let mut rows: Vec<u8> = Vec::new();

		for line in src.lines() {
			let trimmed = line.trim_end();
			if trimmed.trim_start().starts_with(';') || trimmed.is_empty() {
				continue;
			}
			if let Some(rest) = trimmed.strip_prefix("glyph ") {
				flush(current, &mut rows, &mut glyphs);
				current = rest.chars().next();
			} else {
				let mut bits = 0u8;
				for (x, ch) in trimmed.chars().take(GLYPH_W).enumerate() {
					if ch == '#' {
						bits |= 0x80u8 >> x;
					}
				}
				rows.push(bits);
			}
		}
		flush(current, &mut rows, &mut glyphs);
		Self { glyphs }
	}

	/// The bitmap for `c`, or `None` if the atlas doesn't carry it.
	pub fn glyph(&self, c: char) -> Option<&Glyph> {
		self.glyphs.get(&c)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_baked_atlas_carries_every_glyph_the_field_draws() {
		// If a hero glyph is dropped from the data file, the window loses it — this
		// fails before the prototype does. (The status font is a separate, later job.)
		let atlas = Atlas::baked();
		for c in ['@', '#', 'm', '∘', '○', '·'] {
			let g = atlas.glyph(c).unwrap_or_else(|| panic!("the atlas is missing '{c}'"));
			assert!(g.iter().any(|&row| row != 0), "'{c}' has lit pixels, not a blank cell");
		}
	}

	#[test]
	fn parse_maps_the_grid_to_the_right_bits() {
		let atlas = Atlas::parse("glyph A\n#......#\n........\n");
		let g = atlas.glyph('A').expect("A was parsed");
		assert_eq!(g[0], 0b1000_0001, "first and last columns of the top row are lit");
		assert_eq!(g[1], 0, "the second row is bare");
	}
}
