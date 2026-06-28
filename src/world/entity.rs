//! The in-world half of a thing: a glyph at a grid position, addressed by id.

pub type EntityId = u32;

/// The player's well-known id. Minted ids start at `1`, so they never collide.
pub const PLAYER: EntityId = 0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pos {
	pub x: i32,
	pub y: i32,
}

impl Pos {
	pub fn step_toward(self, target: Pos) -> Pos {
		Pos {
			x: self.x + (target.x - self.x).signum(),
			y: self.y + (target.y - self.y).signum(),
		}
	}

	/// Next to, but not on top of.
	pub fn adjacent(self, other: Pos) -> bool {
		let dx = (self.x - other.x).abs();
		let dy = (self.y - other.y).abs();
		dx.max(dy) == 1
	}
}

/// A cardinal facing. Screen coordinates: `+x` is east, `+y` is **down**
/// (north is up), matching the convention in [`crate::vision`]. A heading is the
/// way the player looks — it steers both movement and, later, sight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Heading {
	North,
	East,
	South,
	West,
}

impl Heading {
	/// The one-cell step taken moving forward along this heading.
	pub fn delta(self) -> (i32, i32) {
		match self {
			Heading::North => (0, -1),
			Heading::East  => (1, 0),
			Heading::South => (0, 1),
			Heading::West  => (-1, 0),
		}
	}

	/// Turn a quarter-turn clockwise on screen: E → S → W → N → E.
	pub fn turn_right(self) -> Heading {
		match self {
			Heading::North => Heading::East,
			Heading::East  => Heading::South,
			Heading::South => Heading::West,
			Heading::West  => Heading::North,
		}
	}

	/// Turn a quarter-turn counter-clockwise — the mirror of [`turn_right`](Self::turn_right).
	pub fn turn_left(self) -> Heading {
		self.turn_right().turn_right().turn_right()
	}

	/// This heading as an angle in radians, in [`crate::vision`]'s convention:
	/// `0` is east (`+x`), and the angle grows clockwise on screen (`+y` down).
	pub fn to_radians(self) -> f32 {
		use std::f32::consts::{FRAC_PI_2, PI};
		match self {
			Heading::East  => 0.0,
			Heading::South => FRAC_PI_2,
			Heading::West  => PI,
			Heading::North => -FRAC_PI_2,
		}
	}
}

/// Draw priority for a shared cell: the higher entity is the one painted.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Priority {
	Low  = 0,
	Med  = 1,
	High = 2,
}

/// A thing standing in the world: a glyph at a grid position, keyed by
/// [`EntityId`]. What a kind *is* (name, stats, art) lives elsewhere and is
/// looked up by id — this holds only what the world needs to place and find it.
#[derive(Clone, Debug)]
pub struct Entity {
	pub id:       EntityId,
	pub pos:      Pos,
	pub glyph:    char,
	pub priority: Priority,
}

impl Entity {
	pub fn new(id: EntityId, pos: Pos, glyph: char, priority: Priority) -> Self {
		Entity { id, pos, glyph, priority }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn four_right_turns_come_full_circle() {
		let mut h = Heading::East;
		for _ in 0..4 { h = h.turn_right(); }
		assert_eq!(h, Heading::East);
	}

	#[test]
	fn left_and_right_are_mirror_turns() {
		assert_eq!(Heading::East.turn_left(), Heading::North);
		assert_eq!(Heading::East.turn_right(), Heading::South);
	}

	#[test]
	fn stepping_forward_follows_the_heading() {
		assert_eq!(Heading::North.delta(), (0, -1), "north is up, -y on screen");
		assert_eq!(Heading::South.delta(), (0, 1), "south is down, +y on screen");
	}
}
