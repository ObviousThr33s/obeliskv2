//! Growing the ground: deterministic, seedable scattering of fixed terrain into
//! a [`Field`]. Same seed, same world — generation is reproducible, never random
//! at the player's expense. Whatever already stands (the player, any beings,
//! walls placed earlier) is never built over.

use crate::world::entity::{Entity, Pos, Priority};
use crate::world::field::Field;

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

/// How many cells wide a single noise lattice cell is. Larger = broader masses
/// and calmer clearings; smaller = busier, more fretful ground. Six reads as
/// stone you could walk around, not gravel underfoot.
const LATTICE: i32 = 6;

/// A positional hash → a unit value in `0.0..1.0`, pure and seed-stable. Unlike
/// [`Seed`] this is *not* a stream: the same `(x, y)` always answers the same,
/// so value noise can sample a lattice corner without remembering it.
#[allow(clippy::cast_precision_loss)] // we only keep the low 24 bits; the float is exact there
fn unit_at(seed: u64, x: i32, y: i32) -> f32 {
	let mut h = seed;
	h ^= (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
	h ^= (y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
	h ^= h >> 29;
	h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
	h ^= h >> 32;
	// Low 24 bits over 2^24 — a clean, exactly representable fraction in 0.0..1.0.
	((h & 0x00FF_FFFF) as f32) / 16_777_216.0
}

/// Smooth value noise at a world cell, in `0.0..1.0`. Four lattice corners,
/// smoothstepped and bilinearly blended — so the field rolls between highs and
/// lows instead of jittering cell to cell. The heart of "masses, not snow."
#[allow(clippy::cast_precision_loss)] // grid coords are small; the loss is nothing
fn noise_at(seed: u64, x: i32, y: i32) -> f32 {
	let cell = LATTICE.max(1);
	// Lattice corner this cell sits in, and the fraction across toward the next.
	let (ix, iy) = (x.div_euclid(cell), y.div_euclid(cell));
	let fx = x.rem_euclid(cell) as f32 / cell as f32;
	let fy = y.rem_euclid(cell) as f32 / cell as f32;
	// Smoothstep the fractions so masses ease in and out, never crease.
	let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
	let (sx, sy) = (smooth(fx), smooth(fy));
	let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
	let v00 = unit_at(seed, ix,     iy);
	let v10 = unit_at(seed, ix + 1, iy);
	let v01 = unit_at(seed, ix,     iy + 1);
	let v11 = unit_at(seed, ix + 1, iy + 1);
	lerp(lerp(v00, v10, sx), lerp(v01, v11, sx), sy)
}

/// Grow standing stone as smooth value-noise *masses* rather than scattered
/// grit. `density` is the rough fraction of open ground that becomes stone
/// (`0.0..1.0`); the high ground of the noise field rises into walls, the low
/// ground stays clear, so the world reads as clumps and clearings. Same seed,
/// same masses. Occupied cells are never built over. Returns walls placed.
pub fn grow_masses(
	field:   &mut Field,
	seed:    u64,
	width:   i32,
	height:  i32,
	density: f32,
) -> usize {
	// 0 is a fixed point of the mix — nudge it so a zero seed still grows a world.
	let seed = seed | 1;
	// Stone takes the noise above this line; a higher density floods more ground.
	let level = 1.0 - density.clamp(0.0, 1.0);
	// The living, snapshotted before any stone rises: solid masses must never close
	// in around them, or a being can be born walled into a one-exit pocket. The bus
	// ward "never build over what stands" extended a ring outward — never wall in
	// what *lives*. Walls are `Low`; everything above it keeps a cell of air.
	let anchors: Vec<Pos> = field
		.entities
		.values()
		.filter(|e| e.priority > Priority::Low)
		.map(|e| e.pos)
		.collect();
	let walled_in = |pos: Pos| {
		anchors.iter().any(|a| (pos.x - a.x).abs() <= 1 && (pos.y - a.y).abs() <= 1)
	};
	let mut placed = 0;
	for y in 0..height {
		for x in 0..width {
			let pos = Pos { x, y };
			if noise_at(seed, x, y) <= level || field.at(pos).is_some() || walled_in(pos) {
				continue;
			}
			let id = field.mint();
			field.add(Entity::new(id, pos, '#', Priority::Low));
			placed += 1;
		}
	}
	placed
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
	use crate::world::entity::PLAYER;
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

	#[test]
	fn masses_grow_the_same_world_from_the_same_seed() {
		let mut a = Field::new();
		let mut b = Field::new();
		grow_masses(&mut a, 0xB0A7, 40, 24, 0.4);
		grow_masses(&mut b, 0xB0A7, 40, 24, 0.4);
		assert_eq!(layout(&a), layout(&b), "value-noise masses must be reproducible too");
	}

	#[test]
	fn denser_ground_grows_more_stone_than_sparser() {
		let mut sparse = Field::new();
		let mut dense  = Field::new();
		let few  = grow_masses(&mut sparse, 7, 40, 24, 0.2);
		let many = grow_masses(&mut dense,  7, 40, 24, 0.6);
		assert!(many > few, "raising density floods more of the noise field into stone");
	}

	#[test]
	fn the_masses_clump_instead_of_scattering() {
		// Value noise should leave most stone with a stone neighbour — masses, not
		// lone grit. White noise at the same density would strand far more cells.
		let mut f = Field::new();
		grow_masses(&mut f, 0xB0A7, 60, 40, 0.4);
		let stone: BTreeSet<(i32, i32)> =
			f.entities.values().map(|e| (e.pos.x, e.pos.y)).collect();
		let lonely = stone.iter().filter(|&&(x, y)| {
			!stone.contains(&(x - 1, y)) && !stone.contains(&(x + 1, y))
				&& !stone.contains(&(x, y - 1)) && !stone.contains(&(x, y + 1))
		}).count();
		assert!(
			lonely * 4 < stone.len(),
			"fewer than a quarter of stones stand alone — the ground clumps",
		);
	}

	#[test]
	fn nothing_living_is_ever_walled_into_its_own_pocket() {
		// The player stands where dense noise would otherwise close in on all sides.
		// Every orthogonal neighbour must stay open, so a turn-and-step always goes.
		let mut f = Field::new();
		let p = Pos { x: 20, y: 12 };
		f.add(Entity::new(PLAYER, p, '@', Priority::High));
		grow_masses(&mut f, 0xB0A7, 40, 24, 0.4);
		for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
			let step = Pos { x: p.x + dx, y: p.y + dy };
			assert!(f.at(step).is_none(), "a being keeps a ring of air; ({dx},{dy}) is open");
		}
	}

	#[test]
	fn masses_never_grow_over_what_already_stands() {
		let mut f = Field::new();
		f.add(Entity::new(PLAYER, Pos { x: 5, y: 5 }, '@', Priority::High));
		grow_masses(&mut f, 0xB0A7, 20, 20, 0.9);
		let here = f.at(Pos { x: 5, y: 5 }).expect("the player is still standing");
		assert_eq!((here.id, here.glyph), (PLAYER, '@'), "the player was not paved over");
	}
}
