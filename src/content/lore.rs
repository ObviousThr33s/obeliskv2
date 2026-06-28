//! The world's voice: lines of story keyed by moment, kept out of the code.
//!
//! Mechanism only — it parses and looks up. *What* is said lives in content
//! files (see `lore/voice.txt`), never hardcoded here, so the story can be
//! rewritten without touching the engine.

use std::collections::HashMap;

/// A table of story lines keyed by a moment's name (e.g. `"seen.moth"`).
#[derive(Default)]
pub struct Lore {
	lines: HashMap<String, String>,
}

impl Lore {
	/// Parse `key = value` lines. Blank lines and `#` comments are skipped, and
	/// whitespace around both sides is trimmed. A repeated key takes its last
	/// value, so a content file can override an earlier line further down.
	pub fn parse(source: &str) -> Self {
		let mut lines = HashMap::new();
		for raw in source.lines() {
			let line = raw.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			if let Some((key, value)) = line.split_once('=') {
				lines.insert(key.trim().to_owned(), value.trim().to_owned());
			}
		}
		Self { lines }
	}

	/// The line for a moment, if the story has one to say.
	pub fn line(&self, key: &str) -> Option<&str> {
		self.lines.get(key).map(String::as_str)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_moment_keeps_its_line() {
		let lore = Lore::parse("seen.moth = She is here.");
		assert_eq!(lore.line("seen.moth"), Some("She is here."));
	}

	#[test]
	fn comments_and_blank_lines_are_not_story() {
		let lore = Lore::parse("# a note to self\n\nseen.moth = She is here.\n");
		assert_eq!(lore.line("seen.moth"), Some("She is here."));
		assert_eq!(lore.line("# a note to self"), None);
	}

	#[test]
	fn whitespace_around_key_and_line_is_trimmed() {
		let lore = Lore::parse("   seen.moth   =   She is here.   ");
		assert_eq!(lore.line("seen.moth"), Some("She is here."));
	}

	#[test]
	fn a_moment_with_nothing_to_say_stays_silent() {
		let lore = Lore::parse("seen.moth = She is here.");
		assert_eq!(lore.line("seen.vesh"), None);
	}

	#[test]
	fn the_last_word_on_a_moment_wins() {
		let lore = Lore::parse("seen.moth = first\nseen.moth = second");
		assert_eq!(lore.line("seen.moth"), Some("second"));
	}
}
