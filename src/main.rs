//! Obelisk — the playable seam, in the real terminal. A top-down phosphor view:
//! green for the light, amber for the moment the moth is seen (see
//! `stones/the-glass.md`). You carry the eye; the moth holds while you watch her
//! and comes to the light the moment you look away.
//!
//! Controls: move forward `↑`/`w`, turn `←`/`→` (or `a`/`d`), wait `space`,
//! quit `q`. This file is only input and ink now: the world is the library, and
//! the *projection* — what each cell holds and in what tone — lives in
//! [`obelisk::render`], the one truth the windowed build draws from too.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::{
	cursor,
	event::{self, Event, KeyCode, KeyEventKind},
	queue,
	style::{Color, Print, ResetColor, SetForegroundColor},
	terminal,
};

use obelisk::entity::Pos;
use obelisk::lore::Lore;
use obelisk::render::{self, Cell, Frame};
use obelisk::terrain;
use obelisk::world::{Intent, World};

/// The terminal's viewport, in cells. A const-generic frame is sized at compile
/// time, so this build picks a wide, short grid that sits in a usual terminal; the
/// window picks its own. The projection law they share is [`render::render`].
const VIEW_W: usize = 64;
const VIEW_H: usize = 24;

fn main() -> io::Result<()> {
	let mut world = build_world();
	world.tick(Intent::Wait); // prime sight, so the moth shows on the very first frame
	let _guard = enter_terminal()?; // restores the terminal on the way out, even on error
	let mut out = io::stdout();
	let mut frame: Box<Frame<VIEW_W, VIEW_H>> = Box::new(Frame::blank());
	let start = Instant::now();

	loop {
		render::render(&world, &mut frame, start.elapsed());
		draw(&mut out, &frame)?;
		// Poll, so the fountain can breathe between keystrokes rather than blocking on input.
		if !event::poll(Duration::from_millis(90))? {
			continue;
		}
		if let Event::Key(key) = event::read()? {
			// Windows consoles report both press and release — act on press only,
			// so one keystroke is one tick.
			if key.kind != KeyEventKind::Press {
				continue;
			}
			let intent = match key.code {
				KeyCode::Char('q') | KeyCode::Esc => break,
				KeyCode::Up | KeyCode::Char('w') => Intent::Forward,
				KeyCode::Left | KeyCode::Char('a') => Intent::TurnLeft,
				KeyCode::Right | KeyCode::Char('d') => Intent::TurnRight,
				KeyCode::Down | KeyCode::Char('s') | KeyCode::Char(' ') => Intent::Wait,
				_ => continue, // an unbound key spends no tick
			};
			world.tick(intent);
		}
	}
	Ok(())
}

/// Build a small world: the player in a scattered field of standing stones, the
/// moth a little way off. The story comes from `lore/voice.txt`, baked in at build.
fn build_world() -> World {
	let lore = Lore::parse(include_str!("../lore/voice.txt"));
	let mut world = World::new(Pos { x: 20, y: 12 }, Pos { x: 25, y: 12 })
		.voiced(lore)
		.with_sanctuary(Pos { x: 16, y: 12 }, 2);
	// Stone grown as value-noise masses, not scattered grit — clumps and clearings.
	terrain::grow_masses(&mut world.field, 0xB0A7, 40, 24, 0.4);
	world
}

/// Blit the shared frame to the terminal: one cell, one glyph, in its own ink.
/// No clear sweeps the screen — every cell is overwritten each frame in place — so
/// nothing flickers and the status rows ride along as the foot of the same grid.
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
/// restores it — so a panic or an early return never leaves a wrecked terminal.
fn enter_terminal() -> io::Result<TerminalGuard> {
	terminal::enable_raw_mode()?;
	crossterm::execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
	Ok(TerminalGuard)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
	fn drop(&mut self) {
		let _ = terminal::disable_raw_mode();
		let _ = crossterm::execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
	}
}
