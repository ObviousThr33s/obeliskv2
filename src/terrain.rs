//! Growing the ground: deterministic, seedable scattering of fixed terrain into
//! a [`Field`]. Same seed, same world — generation is reproducible, never random
//! at the player's expense. Whatever already stands (the player, any beings,
//! walls placed earlier) is never built over.

use crate::entity::{Entity, Pos, Priority};
use crate::field::Field;

/// A small deterministic pseudo-random source — xorshift64\*. Pure and
/// allocation-free, so generation honours the engine's no-heap ethos and stays
/// perfectly reproducible from its seed.
struct Seed(u64);

impl Seed {
	fn next(&mut self) -> u64 {
		let mut x = self.0;
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		self.0 = x;
		x.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	/// A value in `0..bound`. `bound` is a positive grid extent.
	fn below(&mut self, bound: i32) -> i32 {
		(self.next() % bound.max(1) as u64) as i32
	}
}

/// Scatter up to `count` walls across the `width`×`height` field, growing the
/// world from `seed`. Occupied cells are left alone, so generation never builds
/// over what already stands. Returns how many walls were actually placed (fewer
/// than `count` only if the field is too crowded to fit them).
pub fn scatter_walls(
	field:  &mut Field,
	seed:   u64,
	width:  i32,
	height: i32,
	count:  usize,
) -> usize {
	// 0 is a fixed point of xorshift — nudge it so a zero seed still grows a world.
	let mut rng = Seed(seed | 1);
	let mut placed = 0;
	// Bounded attempts: a crowded field must not loop forever hunting free cells.
	for _ in 0..count.saturating_mul(8) {
		if placed >= count {
			break;
		}
		let pos = Pos { x: rng.below(width), y: rng.below(height) };
		if field.at(pos).is_some() {
			continue;
		}
		let id = field.mint();
		field.add(Entity::new(id, pos, '#', Priority::Low));
		placed += 1;
	}
	placed
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::entity::PLAYER;
	use std::collections::BTreeSet;

	fn layout(field: &Field) -> BTreeSet<(i32, i32, char)> {
		field.entities.values().map(|e| (e.pos.x, e.pos.y, e.glyph)).collect()
	}

	#[test]
	fn the_same_seed_grows_the_same_world() {
		let mut a = Field::new();
		let mut b = Field::new();
		scatter_walls(&mut a, 42, 20, 10, 15);
		scatter_walls(&mut b, 42, 20, 10, 15);
		assert_eq!(layout(&a), layout(&b), "generation must be reproducible from its seed");
	}

	#[test]
	fn different_seeds_grow_different_worlds() {
		let mut a = Field::new();
		let mut b = Field::new();
		scatter_walls(&mut a, 1, 20, 10, 15);
		scatter_walls(&mut b, 2, 20, 10, 15);
		assert_ne!(layout(&a), layout(&b), "a fresh seed should turn fresh ground");
	}

	#[test]
	fn generation_never_builds_over_what_already_stands() {
		let mut f = Field::new();
		f.add(Entity::new(PLAYER, Pos { x: 5, y: 5 }, '@', Priority::High));
		scatter_walls(&mut f, 7, 10, 10, 64);
		let here = f.at(Pos { x: 5, y: 5 }).expect("the player is still standing");
		assert_eq!((here.id, here.glyph), (PLAYER, '@'), "the player was not paved over");
	}

	#[test]
	fn it_places_no_more_than_it_was_asked_for() {
		let mut f = Field::new();
		let placed = scatter_walls(&mut f, 99, 20, 20, 12);
		assert!(placed <= 12, "never more walls than requested");
		assert_eq!(f.entities.len(), placed, "every placed wall is a real entity in the field");
	}
}
