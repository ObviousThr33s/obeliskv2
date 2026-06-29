//! The terminal medium: the shared front-end the `obelisk` and `obelisk_debug`
//! leaves both grow from. One driver, one [`Debug`] switch — the only thing the
//! two binaries disagree on. Input (keys → intent) and output (a `Frame` blitted
//! to the real terminal) live here; the world and the lens stay in the trunks.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::{
	cursor,
	event::{self, Event, KeyCode, KeyEventKind},
	queue,
	style::{Color, Print, ResetColor, SetForegroundColor},
	terminal,
};

use crate::content::lore::Lore;
use crate::render::{self, Cell, Frame};
use crate::world::entity::{Heading, Pos};
use crate::world::terrain;
use crate::world::{Intent, World};

/// Debug switches for one run. A single boolean today (`OFF`/`ON`), but shaped so
/// it can grow into named tags/groups (a bitset with a `has(tag)` test) *without
/// changing the call sites*: `run(Debug)` stays the seam, and each binary keeps
/// passing one value. That is the "one DEBUG variable, expandable to tags" shape.
#[derive(Clone, Copy, Default)]
pub struct Debug {
	on: bool,
}

impl Debug {
	/// Debug off — the ordinary `obelisk` build.
	pub const OFF: Debug = Debug { on: false };
	/// Debug on — the `obelisk_debug` build.
	pub const ON: Debug = Debug { on: true };

	/// Whether any debug behaviour is wanted. When this becomes a tag-set, keep
	/// this as "any tag set" and add `has(tag)` beside it.
	pub fn enabled(&self) -> bool {
		self.on
	}
}

/// The terminal's viewport, in cells. The const-generic frame is sized at compile
/// time; this front-end picks a wide, short grid that sits in a usual terminal.
const VIEW_W: usize = 64;
const VIEW_H: usize = 24;

/// Run the terminal front-end to completion. The single driver behind both the
/// `obelisk` and `obelisk_debug` binaries; `debug` is the only thing that differs.
pub fn run(debug: Debug) -> io::Result<()> {
	let mut world = build_world();
	world.tick(Intent::Wait); // prime sight, so the moth shows on the very first frame
	let _guard = enter_terminal(debug)?; // restores the terminal on the way out, even on error
	let mut out = io::stdout();
	let mut frame: Box<Frame<VIEW_W, VIEW_H>> = Box::new(Frame::blank());
	let start = Instant::now();

	loop {
		render::render(&world, &mut frame, start.elapsed());
		draw(&mut out, &frame)?;
		// Poll, so the fountain can breathe between keystrokes rather than blocking.
		if !event::poll(Duration::from_millis(33))? {
			continue;
		}
		if let Event::Key(key) = event::read()? {
			// Windows consoles report press and release — act on press only, so one
			// keystroke is one tick.
			if key.kind != KeyEventKind::Press {
				continue;
			}
			let intent = match key.code {
				KeyCode::Char('q') | KeyCode::Esc => break,
				KeyCode::Up | KeyCode::Char('w') => Intent::Step(Heading::North),
				KeyCode::Down | KeyCode::Char('s') => Intent::Step(Heading::South),
				KeyCode::Left | KeyCode::Char('a') => Intent::Step(Heading::West),
				KeyCode::Right | KeyCode::Char('d') => Intent::Step(Heading::East),
				KeyCode::Char(' ') => Intent::Wait,
				_ => continue, // an unbound key spends no tick
			};
			world.tick(intent);
		}
	}
	Ok(())
}

/// Build a small world: the player in a scattered field of standing stones, the
/// moth a little way off. The story comes from `assets/lore/voice.txt`, baked in.
fn build_world() -> World {
	let lore = Lore::parse(include_str!("../assets/lore/voice.txt"));
	let mut world = World::new(Pos { x: 20, y: 12 }, Pos { x: 25, y: 12 })
		.voiced(lore)
		.with_sanctuary(Pos { x: 16, y: 12 }, 2)
		.with_fairy(Pos { x: 22, y: 9 }, 0xFA12);
	// Stone grown as value-noise masses, not scattered grit — clumps and clearings.
	terrain::grow_masses(&mut world.field, 0xB0A7, 40, 24, 0.4);
	world.hallow_sanctuary(); // clear the fountain's ground so the breath can run there
	world
}

/// Blit the shared frame to the terminal: one cell, one glyph, in its own ink.
fn draw<const W: usize, const H: usize>(out: &mut impl Write, frame: &Frame<W, H>) -> io::Result<()> {
	for sy in 0..H {
		let row = u16::try_from(sy).unwrap_or(u16::MAX);
		queue!(out, cursor::MoveTo(0, row))?;
		for sx in 0..W {
			let cell = frame.at(sx, sy).copied().unwrap_or(Cell::VOID);
			let ink = cell.ink;
			queue!(out, SetForegroundColor(Color::Rgb { r: ink.r, g: ink.g, b: ink.b }), Print(cell.glyph))?;
		}
	}
	queue!(out, ResetColor)?;
	out.flush()
}

/// Put the terminal into raw, full-screen mode and hand back a guard whose `Drop`
/// restores it. In debug, the window title is marked so the two builds are easy to
/// tell apart — the first, smallest thing the `Debug` switch reaches.
fn enter_terminal(debug: Debug) -> io::Result<TerminalGuard> {
	terminal::enable_raw_mode()?;
	let title = if debug.enabled() { "obelisk [DEBUG]" } else { "obelisk" };
	crossterm::execute!(
		io::stdout(),
		terminal::EnterAlternateScreen,
		terminal::SetTitle(title),
		cursor::Hide
	)?;
	Ok(TerminalGuard)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
	fn drop(&mut self) {
		let _ = terminal::disable_raw_mode();
		let _ = crossterm::execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
	}
}
