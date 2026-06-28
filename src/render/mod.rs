//! Projecting the world into a grid of coloured cells — the one shared truth both
//! views draw from.
//!
//! This is the pure, testable seam of the renderer, and now it carries the *whole*
//! projection, not a bare slice: the field centred on the player (a camera), the
//! fountain's breathing aura, the moth drawn from *memory* rather than her true
//! cell, and the status rows at the foot of the grid. A [`Frame`] is a grid of
//! glyphs with true [`Rgb`] ink — exactly a terminal screen — so both the terminal
//! build and the windowed build can consume the *same* frame. Two views, one truth.
//!
//! The size is each medium's to choose: a const-generic [`Frame<W, H>`] is sized at
//! compile time, so the terminal and the window pick their own `W`×`H` and simply
//! see more or less of the same centred world. The projection *law* is what is
//! shared, not the dimensions.
//!
//! Two clocks meet here but never mix: **world-time** (the tick) has already moved
//! the world before `render` is called; **render-time** (`elapsed`) only breathes
//! the aura and beats the time-pulse. Render-time colours the eye, never the world
//! (see the *two-clocks* law).

use std::time::Duration;

use crate::world::entity::{Heading, Pos, Priority, PLAYER};
use crate::world::field::Field;
use crate::world::recollection::Sighting;
use crate::world::{World, MOTH};

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

/// The curated palette — the phosphor tones the terminal build tuned by eye, now
/// the shared truth. Colour is chosen by *meaning*, never picked by hand at the
/// call site: a thing maps to one of these tones and no other.
pub mod palette {
	use super::Rgb;

	/// The dark beyond the painted grid — letterbox and blank cells.
	pub const VOID:   Rgb = Rgb { r: 6,   g: 8,   b: 6   };
	/// The dark ground between things.
	pub const GROUND: Rgb = Rgb { r: 0,   g: 30,  b: 0   };
	/// Standing stone — walls and other low, fixed terrain.
	pub const STONE:  Rgb = Rgb { r: 0,   g: 90,  b: 0   };
	/// A being abroad in the world — cool against the green.
	pub const BEING:  Rgb = Rgb { r: 120, g: 150, b: 210 };
	/// The player: the one cell that is surely, brightly known.
	pub const SELF:   Rgb = Rgb { r: 180, g: 255, b: 180 };
	/// The moment the moth is seen — amber, the one warm tone.
	pub const AMBER:  Rgb = Rgb { r: 255, g: 176, b: 0   };
	/// The line at the foot of the grid.
	pub const STATUS: Rgb = Rgb { r: 0,   g: 150, b: 0   };
	/// The fountain's heart and the time-pulse.
	pub const WISP:   Rgb = Rgb { r: 120, g: 220, b: 230 };
}

/// How many rows at the foot of every frame are reserved for the status line and
/// the world's spoken line. The play area is the rows above these.
pub const STATUS_ROWS: usize = 2;

/// The tone a generic thing is painted in, decided by what it is. The two
/// principals (player, moth) are coloured by hand below; everything else maps
/// here, so the world's colour stays coherent by construction.
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

/// Paint the world into the frame: the field centred on the player, the fountain's
/// breathing aura, the moth from memory, and the status rows beneath. The frame is
/// cleared first, so the same `Frame` serves every tick (ward 2).
///
/// `elapsed` is render-time only — it breathes the aura and beats the time-pulse,
/// and touches nothing the world believes about itself.
pub fn render<const W: usize, const H: usize>(
	world:   &World,
	frame:   &mut Frame<W, H>,
	elapsed: Duration,
) {
	frame.clear();
	let player = match world.field.get(PLAYER) {
		Some(p) => p.pos,
		None    => return, // no eye to centre on; leave the grid unlit
	};

	let play_h = H.saturating_sub(STATUS_ROWS);
	let breath = breath_at(elapsed);
	let remembered = world.recollection.recall(MOTH).copied();
	let fountain = world.sanctuary().map(|(center, _)| center);

	// The camera: read a screen cell `(sx, sy)` back to the world cell it shows, with
	// the player held at the centre of the play area. This is the whole of the view's
	// motion — move the player and the world slides under a still eye.
	let (cx, cy) = (as_i32(W / 2), as_i32(play_h / 2));
	let world_of = |sx: usize, sy: usize| Pos {
		x: player.x + as_i32(sx) - cx,
		y: player.y + as_i32(sy) - cy,
	};

	for sy in 0..play_h {
		for sx in 0..W {
			let here = world_of(sx, sy);
			// The world's true aura, breathed for the eye only — safety never flickers.
			let aura = world.aura_at(here) * breath;
			let cell = project_cell(here, player, &remembered, &world.field, aura, fountain == Some(here));
			if let Some(slot) = frame.get_mut(sx, sy) {
				*slot = cell;
			}
		}
	}

	paint_status(world, frame, play_h, elapsed);
}

/// What stands at one world cell, and in what tone. The moth is never drawn from
/// her true position — only from what the watcher remembers — so looking away
/// truly loses her.
fn project_cell(
	here:       Pos,
	player:     Pos,
	remembered: &Option<Sighting>,
	field:      &Field,
	aura:       f32,
	fountain:   bool,
) -> Cell {
	if here == player {
		return Cell { glyph: '@', ink: palette::SELF };
	}
	if fountain {
		return Cell { glyph: '○', ink: palette::WISP }; // the fountain's heart
	}
	if let Some(s) = remembered {
		if s.x == here.x && s.y == here.y {
			if s.clarity >= 0.999 {
				return Cell { glyph: 'm', ink: palette::AMBER }; // seen this very tick
			}
			// A fading memory: the green dims with her clarity.
			let glow = 40_u8.saturating_add((s.clarity * 120.0) as u8);
			return Cell { glyph: 'm', ink: Rgb { r: 0, g: glow, b: 0 } };
		}
	}
	if let Some(e) = field.at(here) {
		// The moth's true cell, while unseen, stays dark — the watcher cannot know it.
		if e.id != PLAYER && e.id != MOTH {
			return Cell { glyph: e.glyph, ink: ink_for(e.priority) };
		}
	}
	if aura > 0.0 {
		return Cell { glyph: '∘', ink: aura_tone(aura) }; // the fountain's pall
	}
	Cell { glyph: '·', ink: palette::GROUND }
}

/// Paint the foot of the grid: a time-pulse and the facing/controls line, then the
/// world's spoken line beneath it. Text is written straight into cells — no
/// allocation (ward 2) — so the status is just more of the same one grid.
fn paint_status<const W: usize, const H: usize>(
	world:   &World,
	frame:   &mut Frame<W, H>,
	play_h:  usize,
	elapsed: Duration,
) {
	let status_row = play_h;
	let spoken_row = play_h + 1;

	let facing = match world.facing {
		Heading::North => "north",
		Heading::East  => "east",
		Heading::South => "south",
		Heading::West  => "west",
	};
	// Laid left to right, each piece starting where the last left off — no hand-placed
	// columns to drift out of step. The pulse, a two-cell gap, then the heading line.
	if let Some(slot) = frame.get_mut(0, status_row) {
		*slot = Cell { glyph: time_pulse(elapsed), ink: palette::WISP };
	}
	let mut col = put_str(frame, 3, status_row, "facing ", palette::STATUS);
	col = put_str(frame, col, status_row, facing, palette::STATUS);
	put_str(frame, col, status_row,
		"    move ↑/w   turn ←/→   wait space   quit q", palette::STATUS);

	if let Some(line) = &world.spoken {
		put_str(frame, 0, spoken_row, line, palette::AMBER);
	}
}

/// Write a string's glyphs into one row from `col`, in `ink`, and return the column
/// just past the last glyph — so callers can lay text left to right without naming a
/// single hand-counted position. Cells past the grid's edge are simply dropped —
/// bounds-checked, never an error (the safe subset).
fn put_str<const W: usize, const H: usize>(
	frame: &mut Frame<W, H>,
	col:   usize,
	row:   usize,
	s:     &str,
	ink:   Rgb,
) -> usize {
	let mut col = col;
	for ch in s.chars() {
		if let Some(slot) = frame.get_mut(col, row) {
			*slot = Cell { glyph: ch, ink };
		}
		col += 1;
	}
	col
}

/// The aura's tone at a given strength: up from the dark ground, through
/// wisp-green, to the cyan heart. The same number the world reads for safety;
/// here it becomes light.
fn aura_tone(strength: f32) -> Rgb {
	let s = strength.clamp(0.0, 1.0);
	let lerp = |a: u8, b: u8, t: f32| (f32::from(a) + (f32::from(b) - f32::from(a)) * t) as u8;
	if s >= 0.5 {
		let t = (s - 0.5) * 2.0;
		Rgb { r: lerp(143, 120, t), g: lerp(208, 220, t), b: lerp(160, 230, t) }
	} else {
		let t = s * 2.0;
		Rgb { r: lerp(0, 143, t), g: lerp(30, 208, t), b: lerp(0, 160, t) }
	}
}

/// The fountain's slow breath — a gentle rise and fall to multiply the aura's glow
/// by. Render-only: the world's true aura, and the safety it grants, never flicker.
fn breath_at(elapsed: Duration) -> f32 {
	let t = elapsed.as_secs_f32();
	0.78 + 0.22 * (t * 0.9).sin()
}

/// A numberless sign that time is passing: a small glyph that waxes and wanes on a
/// steady real-time beat. Cycling = time flows; frozen = time is doing something else.
fn time_pulse(elapsed: Duration) -> char {
	let frames = ['·', '∘', '○', '∘'];
	let step = elapsed.as_millis() / 350 % frames.len() as u128;
	let i = usize::try_from(step).unwrap_or(0);
	frames.get(i).copied().unwrap_or('·')
}

/// A grid index widened to a world coordinate without a lossy cast or a panic.
/// Grids are tiny; the saturating fallback can never actually be reached.
fn as_i32(n: usize) -> i32 {
	i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::world::entity::Pos;
	use crate::world::Intent;

	/// Read a frame row back into a string — test-only, so allocation is free here.
	fn row_text<const W: usize, const H: usize>(frame: &Frame<W, H>, row: usize) -> String {
		(0..W).filter_map(|x| frame.at(x, row)).map(|c| c.glyph).collect()
	}

	#[test]
	fn the_player_stands_at_the_centre_of_the_view() {
		let w = World::new(Pos { x: 20, y: 12 }, Pos { x: 40, y: 40 });
		let mut frame = Frame::<13, 9>::blank(); // play area 13x7, centre (6, 3)
		render(&w, &mut frame, Duration::ZERO);
		let here = frame.at(6, 3).expect("the centre cell is inside the grid");
		assert_eq!((here.glyph, here.ink), ('@', palette::SELF), "the eye sits at the centre");
	}

	#[test]
	fn the_moth_is_drawn_from_memory_not_from_her_true_cell() {
		// Watch her once (she is glimpsed at her true place), then turn away: she
		// creeps a step, but memory stays frozen where she was last seen, dimming.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 3, y: 0 });
		w.tick(Intent::Wait);      // facing East, she is in view → remembered at (3, 0)
		w.tick(Intent::TurnRight); // look away → she creeps to (2, 0); memory freezes at (3, 0)

		let mut frame = Frame::<13, 9>::blank(); // play 13x7, centre (6, 3); player at (0,0)
		render(&w, &mut frame, Duration::ZERO);

		// Remembered cell (3, 0) → (6+3, 3) = (9, 3): the moth, dim green, not amber.
		let remembered = frame.at(9, 3).expect("the remembered cell is in the grid");
		assert_eq!(remembered.glyph, 'm', "she is painted where memory holds her");
		assert!(remembered.ink.r == 0 && remembered.ink.b == 0 && remembered.ink.g > 0,
			"a fading memory dims to green, never amber");

		// Her true cell (2, 0) → (8, 3) must NOT show her — the watcher cannot know it.
		let truth = frame.at(8, 3).expect("the true cell is in the grid");
		assert_ne!(truth.glyph, 'm', "her true, unseen position is never drawn");
	}

	#[test]
	fn the_render_lights_exactly_the_ground_the_world_feels_as_safe() {
		// The load-bearing invariant `aura_at` only claims in prose: "what is seen is
		// exactly what is felt." One field, shared — so every ground cell is lit with
		// aura *if and only if* the world calls that cell safe. A render that drifted to
		// its own radius, dropped the breath's sign, or rounded differently would light
		// a cell the world does not protect (or fail to) — and this test would catch the
		// lie. (The two principals are coloured by hand, so we skip the player's cell.)
		let player = Pos { x: 0, y: 0 };
		let w = World::new(player, Pos { x: 40, y: 40 })
			.with_sanctuary(Pos { x: 1, y: 0 }, 2); // heart off the player, so '○' appears too
		let mut frame = Frame::<13, 9>::blank();
		render(&w, &mut frame, Duration::ZERO);

		let play_h = 9 - STATUS_ROWS;
		let (cx, cy) = (as_i32(13 / 2), as_i32(play_h / 2));
		for sy in 0..play_h {
			for sx in 0..13 {
				let here = Pos { x: player.x + as_i32(sx) - cx, y: player.y + as_i32(sy) - cy };
				if here == player {
					continue; // the eye itself is '@', never aura
				}
				let cell = frame.at(sx, sy).expect("a cell inside the grid");
				let lit = matches!(cell.glyph, '∘' | '○'); // pall or heart — both are the aura
				assert_eq!(
					lit, w.is_safe(here),
					"cell {here:?}: lit by aura iff the world feels it safe — seen is felt",
				);
			}
		}
	}

	#[test]
	fn the_status_row_names_the_way_you_face() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 40, y: 40 });
		w.tick(Intent::TurnRight); // East → South
		let mut frame = Frame::<60, 6>::blank(); // play 60x4, status row at y=4
		render(&w, &mut frame, Duration::ZERO);
		assert!(row_text(&frame, 4).contains("facing south"),
			"the foot of the grid names the heading");
	}

	#[test]
	fn clearing_returns_the_grid_to_unlit_ground() {
		let w = World::new(Pos { x: 0, y: 0 }, Pos { x: 40, y: 40 });
		let mut frame = Frame::<5, 5>::blank();
		render(&w, &mut frame, Duration::ZERO);
		assert_ne!(frame.at(2, 1), Some(&Cell::VOID), "the world was painted");
		frame.clear();
		assert_eq!(frame.at(2, 1), Some(&Cell::VOID), "and then wiped back to ground");
	}
}
