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
	/// The atlas baked into the binary: our own hero marks from
	/// `assets/glyphs/atlas.txt`, then standard ASCII filled in from the `font8x8`
	/// crate so the window's HUD text reads as letters. Hand-tuned glyphs are parsed
	/// first and always win.
	pub fn baked() -> Self {
		let mut atlas = Self::parse(include_str!("../../assets/glyphs/atlas.txt"));
		atlas.fill_ascii_from_font8x8();
		atlas
	}

	/// Fill the standard printable ASCII glyphs from `font8x8` (a public-domain 8x8
	/// bitmap font, pure Rust — no `.ttf`). A slot already held by one of our hero
	/// glyphs is never overwritten. font8x8 packs a row LSB-first (bit 0 = leftmost);
	/// our convention is MSB-first (`0x80` = leftmost), so each row's bits are reversed.
	fn fill_ascii_from_font8x8(&mut self) {
		use font8x8::UnicodeFonts;
		for code in 0x20u8..=0x7e {
			let c = char::from(code);
			if self.glyphs.contains_key(&c) {
				continue; // a hand-tuned hero glyph already holds this slot
			}
			if let Some(rows) = font8x8::BASIC_FONTS.get(c) {
				let mut g: Glyph = [0; GLYPH_H];
				for (slot, &row) in g.iter_mut().zip(rows.iter()) {
					*slot = row.reverse_bits();
				}
				self.glyphs.insert(c, g);
			}
		}
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
	fn the_baked_atlas_renders_hud_letters_not_blocks() {
		// Build-to-last: the window's status text must read as letters. If the font8x8
		// fill ever breaks, the HUD falls back to solid blocks — and this fails first.
		let atlas = Atlas::baked();
		for c in "facing north east south west move turn wait space quit".chars() {
			if c == ' ' {
				continue;
			}
			assert!(
				atlas.glyph(c).is_some_and(|g| g.iter().any(|&row| row != 0)),
				"the HUD letter '{c}' must render as a glyph, not a block",
			);
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
