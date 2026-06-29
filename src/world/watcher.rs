//! The data half of a watcher — its name and stats — kept apart from the in-world
//! [`Entity`](crate::world::entity::Entity) (the glyph at a position). One day these
//! load from `.being` files; for now a watcher can be *generated* deterministically
//! from a seed, so a fresh one has a random-feeling name and stats that are still
//! reproducible — same seed, same watcher (the engine's "never random at the player's
//! expense" ethos, as in [`crate::world::terrain`]).

/// A watcher's data: a name and simple stats. Generated now; file-loaded later.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Watcher {
	pub name:   String,
	pub health: i32,
	pub power:  i32,
}

impl Watcher {
	/// A deterministically-generated watcher: soft, doubled-vowel syllables in the key
	/// of the lore (Ooloonoo, Vesh), and small living stats. Same seed, same watcher.
	pub fn random(seed: u64) -> Watcher {
		// xorshift64*, the same pure generator terrain uses; 0 is a fixed point, so nudge it.
		let mut s = seed | 1;
		let mut next = move || {
			s ^= s >> 12;
			s ^= s << 25;
			s ^= s >> 27;
			s.wrapping_mul(0x2545_F491_4F6C_DD1D)
		};
		let parts = ["oo", "loo", "noo", "roo", "ve", "sh", "ny", "ae", "el", "li", "th", "wi"];
		let count = 2 + (next() % 2) as usize; // two or three syllables
		let mut name = String::new();
		for _ in 0..count {
			let i = (next() % parts.len() as u64) as usize;
			name.push_str(parts.get(i).copied().unwrap_or("oo"));
		}
		Watcher {
			name:   capitalise(&name),
			health: 5 + (next() % 11) as i32, // 5..=15
			power:  3 + (next() % 9) as i32,  // 3..=11
		}
	}
}

/// First letter upper, the rest unchanged; an empty name takes a gentle default.
fn capitalise(s: &str) -> String {
	let mut chars = s.chars();
	match chars.next() {
		Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
		None => "Wisp".to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_same_seed_grows_the_same_watcher() {
		assert_eq!(Watcher::random(0xFA12), Watcher::random(0xFA12), "generation is reproducible");
	}

	#[test]
	fn a_generated_watcher_is_named_and_alive() {
		let w = Watcher::random(7);
		assert!(!w.name.is_empty(), "she is named");
		assert!(w.name.starts_with(|c: char| c.is_uppercase()), "named with a capital");
		assert!(w.health > 0 && w.power > 0, "alive, with some strength");
	}
}
