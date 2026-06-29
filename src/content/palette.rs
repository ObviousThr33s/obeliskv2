//! Palette parsing — named colours from a baked data file (the asset doctrine,
//! see `docs/assets.md`). Deliberately *lenient*: it scans the text for each
//! colour's name, `hex`, and `role`, so a malformed or missing field — like the
//! empty `"rgb":` some tools emit — never derails the parse. The hex is the source
//! of truth; everything else is best-effort. There will be many palettes; one
//! tolerant parser reads them all.

/// One named colour and the UI role authored for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Swatch {
	pub name: String,
	pub rgb:  (u8, u8, u8),
	pub role: String,
}

/// A named set of swatches — one palette.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Palette {
	pub name:     String,
	pub swatches: Vec<Swatch>,
}

impl Palette {
	/// The baked palettes (from `assets/palettes/`).
	pub fn sunset_arcade() -> Palette {
		Palette::parse(include_str!("../../assets/palettes/sunset-arcade.json"))
	}

	/// The warm, soft-focus palette.
	pub fn golden_hour_haven() -> Palette {
		Palette::parse(include_str!("../../assets/palettes/golden-hour-haven.json"))
	}

	/// Parse a palette from text — lenient (see the module note). Understands the
	/// JSON-ish shape: `"palette_name": "..."` and colour blocks opened by a line
	/// `"<name>": {`, each carrying a `"hex": "#RRGGBB"` and optional `"role": "..."`.
	/// Anything else is skipped; a malformed field is never fatal.
	pub fn parse(src: &str) -> Palette {
		let mut palette = Palette::default();
		let mut current: Option<String> = None;

		for line in src.lines() {
			if let Some(name) = quoted_value_after(line, "palette_name") {
				palette.name = name;
			} else if let Some(name) = block_name(line) {
				current = Some(name);
			} else if let Some(hex) = quoted_value_after(line, "hex") {
				if let Some(rgb) = hex_to_rgb(&hex) {
					palette.swatches.push(Swatch {
						name: current.clone().unwrap_or_default(),
						rgb,
						role: String::new(),
					});
				}
			} else if let Some(role) = quoted_value_after(line, "role") {
				if let Some(last) = palette.swatches.last_mut() {
					last.role = role;
				}
			}
		}
		palette
	}

	/// The colour for a swatch name, if present.
	pub fn get(&self, name: &str) -> Option<(u8, u8, u8)> {
		self.swatches.iter().find(|s| s.name == name).map(|s| s.rgb)
	}
}

/// Parse `#RRGGBB` (or bare `RRGGBB`) into bytes. `None` unless it is six hex digits.
pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
	let h = hex.trim().trim_start_matches('#');
	if h.len() != 6 {
		return None;
	}
	let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
	Some((byte(0)?, byte(2)?, byte(4)?))
}

/// The name of a JSON block opener like `"deep_walnut": {` — but not the `colors`
/// container. `None` for any other line.
fn block_name(line: &str) -> Option<String> {
	let t = line.trim();
	if !t.ends_with('{') {
		return None;
	}
	let key = t.strip_prefix('"')?.split('"').next()?;
	if key.is_empty() || key == "colors" {
		return None;
	}
	Some(key.to_string())
}

/// The first quoted value after `"key"` on a line — e.g. `"hex"` in
/// `"hex": "#FF9D23"` returns `#FF9D23`. `None` if the key or a quoted value is absent.
fn quoted_value_after(line: &str, key: &str) -> Option<String> {
	let needle = format!("\"{key}\"");
	let after = line.split_once(&needle)?.1.split_once(':')?.1;
	let start = after.find('"')? + 1;
	let rest = after.get(start..)?;
	let end = rest.find('"')?;
	rest.get(..end).map(str::to_string)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hex_parses_to_bytes() {
		assert_eq!(hex_to_rgb("#4B2E2B"), Some((75, 46, 43)));
		assert_eq!(hex_to_rgb("FF9D23"), Some((255, 157, 35)));
		assert_eq!(hex_to_rgb("#nope"), None);
	}

	#[test]
	fn parses_the_raw_palette_despite_a_malformed_rgb_field() {
		// The exact shape pasted from Gemini — the empty `"rgb":` breaks strict JSON,
		// but the hex is intact, so a tolerant parse must still succeed. This is the
		// whole point: read messy real data without choking.
		let raw = r##"{
  "palette_name": "Sunset Arcade",
  "colors": {
    "deep_walnut": {
      "hex": "#4B2E2B",
      "rgb":,
      "role": "Background, woodgrain base"
    },
    "sunburst_amber": {
      "hex": "#FF9D23",
      "rgb":,
      "role": "Primary active text"
    }
  }
}"##;
		let p = Palette::parse(raw);
		assert_eq!(p.name, "Sunset Arcade");
		assert_eq!(p.get("deep_walnut"), Some((75, 46, 43)));
		assert_eq!(p.get("sunburst_amber"), Some((255, 157, 35)));
		assert_eq!(p.swatches.first().map(|s| s.role.as_str()), Some("Background, woodgrain base"));
	}

	#[test]
	fn the_baked_palettes_load() {
		let s = Palette::sunset_arcade();
		assert_eq!(s.name, "Sunset Arcade");
		assert_eq!(s.get("sunburst_amber"), Some((255, 157, 35)));
		assert!(Palette::golden_hour_haven().get("honey_oak").is_some());
	}
}
