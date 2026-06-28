//! Projecting the world into a grid of coloured cells — the terminal look kept,
//! the terminal's sixteen colour-names left behind.
//!
//! This is the pure, testable seam of the renderer. A [`Frame`] is still a grid
//! of glyphs, one per cell, exactly as a terminal is — but each glyph carries a
//! true [`Rgb`] ink, because the renderer that comes above this owns its pixels.
//! Turning cells into lit pixels, and mastering that image, are mechanism layered
//! on top of this and judged by eye; *this* layer is judged by test.

use crate::entity::Priority;
use crate::field::Field;

/// A colour, eight bits a channel. Plain data, no references (ward 1 in spirit):
/// a cell carries the colour itself, not a handle to one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

/// One cell of the grid — the terminal model, kept: a single `glyph` painted in
/// a single `ink`. The dark ground it sits on is the renderer's, uniform for now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
	pub glyph: char,
	pub ink:   Rgb,
}

impl Cell {
	/// Unlit ground: a blank space in the dimmest tone. A fresh [`Frame`] is all
	/// of these until the world is painted over it.
	pub const VOID: Cell = Cell { glyph: ' ', ink: palette::VOID };
}

/// The curated palette. Colour is chosen by *meaning*, never by a value picked
/// out by hand at the call site: a thing maps to one of these tones and no other.
/// A few phosphor-adjacent greens, one cool tone for beings, so the eye stays calm.
pub mod palette {
	use super::Rgb;

	/// The dark the whole world rests on.
	pub const VOID:  Rgb = Rgb { r: 6,   g: 8,   b: 6   };
	/// Standing stone — walls and other low, fixed terrain.
	pub const STONE: Rgb = Rgb { r: 44,  g: 74,  b: 48  };
	/// A being abroad in the world — cool against the green.
	pub const BEING: Rgb = Rgb { r: 120, g: 150, b: 210 };
	/// The player: the one cell that is surely, brightly known.
	pub const SELF:  Rgb = Rgb { r: 190, g: 235, b: 200 };
}

/// The tone a thing is painted in, decided by what it is. Drawn from
/// [`palette`] alone, so the world's colour stays coherent by construction.
const fn ink_for(priority: Priority) -> Rgb {
	match priority {
		Priority::High => palette::SELF,
		Priority::Med  => palette::BEING,
		Priority::Low  => palette::STONE,
	}
}

/// A fixed `W`×`H` grid of cells. Sized once at the type level and reused frame
/// to frame (ward 2): [`render`] clears and repaints it, never reallocating.
pub struct Frame<const W: usize, const H: usize> {
	cells: [[Cell; W]; H],
}

impl<const W: usize, const H: usize> Frame<W, H> {
	/// A grid of unlit ground.
	pub fn blank() -> Self {
		Self { cells: [[Cell::VOID; W]; H] }
	}

	/// Wipe the grid back to unlit ground, ready to be repainted.
	pub fn clear(&mut self) {
		self.cells = [[Cell::VOID; W]; H];
	}

	/// The cell at `(x, y)`, or `None` if that lies outside the grid. Bounds are
	/// checked, never assumed — no raw indexing into the world.
	pub fn at(&self, x: usize, y: usize) -> Option<&Cell> {
		self.cells.get(y).and_then(|row| row.get(x))
	}

	fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
		self.cells.get_mut(y).and_then(|row| row.get_mut(x))
	}
}

impl<const W: usize, const H: usize> Default for Frame<W, H> {
	fn default() -> Self {
		Self::blank()
	}
}

/// Paint the field's standing things into the frame. The viewport's origin is the
/// field's `(0, 0)`; a thing whose cell falls outside `W`×`H` — or at a negative
/// coordinate, off the top or left — is simply not painted, never an error.
///
/// Beings are painted here at full ink for now; in the next slice they move to
/// being lit from [`crate::recollection`] memory, dimmed by how clearly they are
/// still remembered. The frame is cleared first, so the same `Frame` serves every
/// tick.
pub fn render<const W: usize, const H: usize>(field: &Field, frame: &mut Frame<W, H>) {
	frame.clear();
	for entity in field.entities.values() {
		let (Ok(x), Ok(y)) = (usize::try_from(entity.pos.x), usize::try_from(entity.pos.y))
		else {
			continue;
		};
		if let Some(cell) = frame.get_mut(x, y) {
			*cell = Cell { glyph: entity.glyph, ink: ink_for(entity.priority) };
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::entity::{Entity, Pos, PLAYER};

	#[test]
	fn the_world_projects_its_standing_things_into_the_grid() {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, Pos { x: 1, y: 1 }, '@', Priority::High));
		let wall = field.mint();
		field.add(Entity::new(wall, Pos { x: 3, y: 2 }, '#', Priority::Low));

		let mut frame = Frame::<8, 4>::blank();
		render(&field, &mut frame);

		let here = frame.at(1, 1).expect("the player's cell is inside the grid");
		assert_eq!((here.glyph, here.ink), ('@', palette::SELF), "the player, brightly known");

		let there = frame.at(3, 2).expect("the wall's cell is inside the grid");
		assert_eq!((there.glyph, there.ink), ('#', palette::STONE), "standing stone");

		let empty = frame.at(0, 0).expect("a cell inside the grid");
		assert_eq!(*empty, Cell::VOID, "untouched ground stays void");
	}

	#[test]
	fn a_thing_outside_the_viewport_is_simply_not_painted() {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, Pos { x: 99, y: 99 }, '@', Priority::High));
		// A negative coordinate, off the top-left — must be skipped, not panic.
		let stray = field.mint();
		field.add(Entity::new(stray, Pos { x: -1, y: 0 }, '#', Priority::Low));

		let mut frame = Frame::<4, 4>::blank();
		render(&field, &mut frame);

		for y in 0..4 {
			for x in 0..4 {
				assert_eq!(
					frame.at(x, y).expect("a cell inside the grid"),
					&Cell::VOID,
					"nothing off the viewport reached the grid",
				);
			}
		}
	}

	#[test]
	fn clearing_returns_the_grid_to_unlit_ground() {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, Pos { x: 0, y: 0 }, '@', Priority::High));
		let mut frame = Frame::<2, 2>::blank();
		render(&field, &mut frame);
		assert_ne!(frame.at(0, 0), Some(&Cell::VOID), "the player was painted");

		frame.clear();
		assert_eq!(frame.at(0, 0), Some(&Cell::VOID), "and then wiped back to ground");
	}
}
