//! Obelisk — the playable seam. A top-down phosphor view in the real terminal:
//! green for the light, amber for the moment the moth is seen (see
//! `stones/the-glass.md`). You carry the eye; the moth holds while you watch her
//! and comes to the light the moment you look away.
//!
//! Controls: move forward `↑`/`w`, turn `←`/`→` (or `a`/`d`), wait `space`,
//! quit `q`. The world is the library; this file is only input and ink.

use std::io::{self, Write};

use crossterm::{
	cursor,
	event::{self, Event, KeyCode, KeyEventKind},
	queue,
	style::{Color, Print, ResetColor, SetForegroundColor},
	terminal::{self, ClearType},
};

use obelisk::entity::{Heading, Pos, PLAYER};
use obelisk::field::Field;
use obelisk::lore::Lore;
use obelisk::recollection::Sighting;
use obelisk::terrain;
use obelisk::world::{Intent, World, MOTH};

/// Phosphor tones — the only two colours that earned their place, plus the
/// dim greens that read as depth.
const LIGHT:  Color = Color::Rgb { r: 180, g: 255, b: 180 }; // the player, the eye
const AMBER:  Color = Color::Rgb { r: 255, g: 176, b: 0 };   // the moment she is seen
const WALL:   Color = Color::Rgb { r: 0,   g: 90,  b: 0 };   // standing stone
const GROUND: Color = Color::Rgb { r: 0,   g: 30,  b: 0 };   // the dark between
const STATUS: Color = Color::Rgb { r: 0,   g: 150, b: 0 };   // the line at the foot

fn main() -> io::Result<()> {
	let mut world = build_world();
	let _guard = enter_terminal()?; // restores the terminal on the way out, even on error
	let mut out = io::stdout();

	loop {
		draw(&mut out, &world)?;
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
	let mut world = World::new(Pos { x: 20, y: 12 }, Pos { x: 28, y: 9 }).voiced(lore);
	terrain::scatter_walls(&mut world.field, 0xB0A7, 40, 24, 70);
	world
}

/// Paint one frame: the field centred on the player, the moth drawn only from
/// memory — amber where seen now, a fading green where she was last glimpsed.
fn draw(out: &mut impl Write, world: &World) -> io::Result<()> {
	let (cols, rows) = terminal::size()?;
	let view_rows = rows.saturating_sub(2).max(1); // foot of the screen is the status
	let cx = i32::from(cols) / 2;
	let cy = i32::from(view_rows) / 2;

	let player = match world.field.get(PLAYER) {
		Some(p) => p.pos,
		None => return Ok(()),
	};
	let remembered = world.recollection.recall(MOTH).copied();

	queue!(out, terminal::Clear(ClearType::All))?;
	for sy in 0..view_rows {
		for sx in 0..cols {
			let here = Pos {
				x: player.x + i32::from(sx) - cx,
				y: player.y + i32::from(sy) - cy,
			};
			let (glyph, colour) = cell(here, player, &remembered, &world.field);
			queue!(
				out,
				cursor::MoveTo(sx, sy),
				SetForegroundColor(colour),
				Print(glyph),
			)?;
		}
	}

	let facing = match world.facing {
		Heading::North => "north",
		Heading::East => "east",
		Heading::South => "south",
		Heading::West => "west",
	};
	queue!(
		out,
		cursor::MoveTo(0, view_rows),
		SetForegroundColor(STATUS),
		Print(format!(
			"facing {facing}    move ↑/w   turn ←/→   wait space   quit q",
		)),
	)?;
	if let Some(line) = &world.spoken {
		queue!(out, cursor::MoveTo(0, view_rows + 1), SetForegroundColor(AMBER), Print(line))?;
	}

	queue!(out, ResetColor)?;
	out.flush()
}

/// What stands at one world cell, and in what tone. The moth is never drawn from
/// her true position — only from what the watcher remembers — so looking away
/// truly loses her.
fn cell(here: Pos, player: Pos, remembered: &Option<Sighting>, field: &Field) -> (char, Color) {
	if here == player {
		return ('@', LIGHT);
	}
	if let Some(s) = remembered {
		if s.x == here.x && s.y == here.y {
			if s.clarity >= 0.999 {
				return ('m', AMBER); // seen this very tick
			}
			// A fading memory: the green dims with her clarity.
			let glow = 40_u8.saturating_add((s.clarity * 120.0) as u8);
			return ('m', Color::Rgb { r: 0, g: glow, b: 0 });
		}
	}
	if let Some(e) = field.at(here) {
		// The moth's true cell, while unseen, stays dark — the watcher cannot know it.
		if e.id != PLAYER && e.id != MOTH {
			return ('#', WALL);
		}
	}
	('·', GROUND)
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
