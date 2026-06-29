// Hide the console for the windowed build so a double-clicked exe shows only the
// window; kept under `cargo test` so test output still has somewhere to go.
#![cfg_attr(not(test), windows_subsystem = "windows")]

//! Obelisk — the lens. The unified product: a real window showing the game panel
//! *and* the status/spoken text together, both drawn from the one shared
//! [`render::render`] the terminal build uses — the same world [`obelisk`] (the
//! terminal) shows, here in owned pixels with baked glyphs.
//!
//! It carries a **heartbeat**: a render clock beats a gentle redraw so the
//! fountain's aura breathes between keystrokes, exactly as the terminal does. Two
//! clocks meet but never mix — *world-time* moves only on a keypress; *render-time*
//! (`start.elapsed()`) only breathes the eye. The world never changes off the beat.
//!
//! [`obelisk`]: obelisk.rs

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use obelisk::world::entity::{Heading, Pos};
use obelisk::content::lore::Lore;
use obelisk::render::{self, Frame};
use obelisk::render::atlas::Atlas;
use obelisk::world::terrain;
use obelisk::world::{Intent, World};

/// How many cells the window shows, and how many pixels make a cell. The height
/// includes [`render::STATUS_ROWS`] for the status line and spoken lore at the foot,
/// so the panel and the text ride in one window. Glyphs paint inside this box later;
/// for now each cell is a solid block of its ink, but the aura already breathes.
const VIEW_W: usize = 48;
const VIEW_H: usize = 32 + render::STATUS_ROWS;
const CELL: usize = 16;

/// The heartbeat: how often the render clock asks for a fresh frame. The breath is a
/// slow sine, so ~30 beats a second reads as perfectly smooth while leaving the machine
/// asleep most of each beat. A keystroke redraws at once regardless, so input never
/// waits on this — it paces only the idle breath.
const BEAT: Duration = Duration::from_millis(33);

fn main() -> Result<(), winit::error::EventLoopError> {
	let mut world = build_world();
	world.tick(Intent::Wait); // prime sight, as the terminal build does, so the moth shows at once

	let event_loop = match EventLoop::new() {
		Ok(loop_) => loop_,
		Err(err) => {
			eprintln!("could not start the event loop: {err}");
			return Ok(());
		}
	};
	// The heartbeat sets the cadence from `about_to_wait`; start it waiting.
	event_loop.set_control_flow(ControlFlow::Wait);

	let now = Instant::now();
	let mut lens = Lens {
		world,
		frame: Box::new(Frame::blank()),
		start: now,
		next_beat: now,
		atlas: Atlas::baked(),
		window: None,
		context: None,
		surface: None,
	};
	event_loop.run_app(&mut lens)
}

/// The same small world the terminal build raises: the player among grown stone
/// masses, the moth a little way off, the fountain its sanctuary. Story baked in.
fn build_world() -> World {
	let lore = Lore::parse(include_str!("../../assets/lore/voice.txt"));
	let mut world = World::new(Pos { x: 20, y: 12 }, Pos { x: 25, y: 12 })
		.voiced(lore)
		.with_sanctuary(Pos { x: 16, y: 12 }, 2)
		.with_fairy(Pos { x: 22, y: 9 }, 0xFA12);
	terrain::grow_masses(&mut world.field, 0xB0A7, 40, 24, 0.4);
	world.hallow_sanctuary(); // clear the fountain's ground so the breath can run there
	world
}

/// The window and its held resources. The frame is boxed and reused tick to tick
/// (ward 2): [`render::render`] repaints it, never reallocating.
struct Lens {
	world:   World,
	frame:   Box<Frame<VIEW_W, VIEW_H>>,
	/// The render clock — render-time, never world-time. The breath and time-pulse
	/// read `start.elapsed()`; the world only moves on a keystroke.
	start:   Instant,
	/// When the next breath is due. The heartbeat asks for a redraw only once this
	/// deadline passes, then sleeps until the one after — so the loop idles between
	/// beats instead of spinning a core flat out.
	next_beat: Instant,
	/// The baked glyph atlas, parsed once at startup and stamped per cell.
	atlas:   Atlas,
	window:  Option<Rc<Window>>,
	context: Option<Context<Rc<Window>>>,
	surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

impl ApplicationHandler for Lens {
	/// The window is born here — winit only hands us a surface once the platform is
	/// ready, so creation belongs in `resumed`, not `main`.
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.window.is_some() {
			return; // already raised; a second resume changes nothing
		}
		let attrs = Window::default_attributes()
			.with_title(concat!("Obelisk — the lens  v", env!("CARGO_PKG_VERSION")))
			.with_inner_size(LogicalSize::new(as_u32(VIEW_W * CELL), as_u32(VIEW_H * CELL)));

		let window = match event_loop.create_window(attrs) {
			Ok(window) => Rc::new(window),
			Err(err) => {
				eprintln!("could not open a window: {err}");
				event_loop.exit();
				return;
			}
		};
		let context = match Context::new(window.clone()) {
			Ok(context) => context,
			Err(err) => {
				eprintln!("no display to draw on: {err}");
				event_loop.exit();
				return;
			}
		};
		let surface = match Surface::new(&context, window.clone()) {
			Ok(surface) => surface,
			Err(err) => {
				eprintln!("no drawing surface: {err}");
				event_loop.exit();
				return;
			}
		};

		window.request_redraw();
		self.window = Some(window);
		self.context = Some(context);
		self.surface = Some(surface);
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::KeyboardInput { event: key, .. } => self.on_key(event_loop, key),
			WindowEvent::RedrawRequested => self.redraw(),
			WindowEvent::Resized(_) => {
				// The window changed size; repaint so the grid re-fits to it.
				if let Some(window) = &self.window {
					window.request_redraw();
				}
			}
			_ => {}
		}
	}

	/// The heartbeat. winit calls this before the loop waits; we ask for a fresh frame
	/// *only* once the beat is actually due, then sleep until the next one. Requesting a
	/// redraw every pass instead would leave one forever pending and spin a core flat
	/// out — the `WaitUntil` would never get to idle. Gating on the deadline is what lets
	/// render-time breathe the aura while the machine mostly sleeps.
	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		let now = Instant::now();
		if now >= self.next_beat {
			self.next_beat = now + BEAT;
			if let Some(window) = &self.window {
				window.request_redraw();
			}
		}
		event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_beat));
	}
}

impl Lens {
	/// One keystroke, one tick — the same bindings as the terminal build. Release
	/// events and unbound keys spend nothing; only a press that means something ticks.
	fn on_key(&mut self, event_loop: &ActiveEventLoop, key: KeyEvent) {
		if key.state != ElementState::Pressed {
			return;
		}
		let intent = match key.logical_key {
			Key::Named(NamedKey::ArrowUp) => Intent::Step(Heading::North),
			Key::Named(NamedKey::ArrowDown) => Intent::Step(Heading::South),
			Key::Named(NamedKey::ArrowLeft) => Intent::Step(Heading::West),
			Key::Named(NamedKey::ArrowRight) => Intent::Step(Heading::East),
			Key::Named(NamedKey::Space) => Intent::Wait,
			Key::Named(NamedKey::Escape) => {
				event_loop.exit();
				return;
			}
			Key::Character(ref c) => match c.as_str() {
				"w" => Intent::Step(Heading::North),
				"s" => Intent::Step(Heading::South),
				"a" => Intent::Step(Heading::West),
				"d" => Intent::Step(Heading::East),
				" " => Intent::Wait,
				"q" => {
					event_loop.exit();
					return;
				}
				_ => return,
			},
			_ => return,
		};
		self.world.tick(intent);
		if let Some(window) = &self.window {
			window.request_redraw(); // the world moved; ask for fresh pixels at once
		}
	}

	/// Paint the world into the cell grid (the shared render seam), then the grid into
	/// the window's pixels. Any failure simply skips this frame — degrade, don't crash.
	fn redraw(&mut self) {
		let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
			return;
		};
		let size = window.inner_size();
		let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
			return; // a zero-sized window (minimised) has nothing to draw
		};
		if surface.resize(w, h).is_err() {
			return;
		}

		render::render(&self.world, &mut self.frame, self.start.elapsed());

		let mut buffer = match surface.buffer_mut() {
			Ok(buffer) => buffer,
			Err(_) => return,
		};
		rasterise(&self.frame, &self.atlas, size.width as usize, size.height as usize, &mut buffer);
		let _ = buffer.present();
	}
}

/// Turn the cell grid into lit pixels, **scaled to fit the window** — so the build is
/// resizable: the fixed `W`×`H` frame fills whatever size the window is, square cells,
/// centred and letterboxed (never stretched). The whole field starts as [`VOID`]
/// (letterbox, gaps, and unlit glyph pixels are all void); then each cell paints only
/// its *lit* glyph pixels in the cell's ink.
///
/// Cell-major, not pixel-major: the atlas is consulted **once per cell**, not once per
/// pixel, so a frame costs `W*H` glyph lookups instead of `width*height` of them — the
/// difference between hundreds and hundreds of thousands every breath.
///
/// A blank cell is bare void; a glyph the atlas doesn't carry yet falls back to a solid
/// block. No allocation, no raw indexing (ward 2): every access is bounds-checked, and
/// the glyph fills only `span` of each cell so `gx / scale` can never overshoot the
/// 8-wide bitmap.
///
/// [`VOID`]: render::palette::VOID
fn rasterise<const W: usize, const H: usize>(
	frame:  &Frame<W, H>,
	atlas:  &Atlas,
	width:  usize,
	height: usize,
	pixels: &mut [u32],
) {
	let (cell, off_x, off_y) = fit(width, height, W, H);
	let scale = (cell / render::atlas::GLYPH_W).max(1);
	let span = render::atlas::GLYPH_W * scale; // glyph fills this much of the cell; the rest is gap
	let void = pack(render::palette::VOID);

	// The ground for everything: letterbox, inter-glyph gaps, and unlit pixels all read
	// as void. Cells then stamp their lit pixels over the top.
	for slot in pixels.iter_mut() {
		*slot = void;
	}

	for cy in 0..H {
		let base_y = off_y + cy * cell;
		for cx in 0..W {
			let Some(here) = frame.at(cx, cy) else { continue };
			if here.glyph == ' ' {
				continue; // a blank cell is bare void; nothing to stamp
			}
			let ink = pack(here.ink);
			// One atlas lookup for the whole cell. `None` ⇒ a glyph not yet drawn, shown
			// as a solid block so it still reads.
			let bm = atlas.glyph(here.glyph);
			let base_x = off_x + cx * cell;
			for gy in 0..span {
				let py = base_y + gy;
				let row = py * width;
				let bits = bm.and_then(|g| g.get(gy / scale).copied());
				for gx in 0..span {
					let lit = match bits {
						Some(bits) => bits & (0x80u8 >> (gx / scale)) != 0,
						None       => true, // solid-block fallback
					};
					if lit {
						if let Some(slot) = pixels.get_mut(row + base_x + gx) {
							*slot = ink;
						}
					}
				}
			}
		}
	}
}

/// The square cell size and centring offsets to fit a `cols`×`rows` grid into a
/// `width`×`height` window — letterboxed, never stretched. The cell is bound by
/// whichever axis is tighter, and is always at least one pixel.
fn fit(width: usize, height: usize, cols: usize, rows: usize) -> (usize, usize, usize) {
	let cell = (width / cols.max(1)).min(height / rows.max(1)).max(1);
	let off_x = width.saturating_sub(cols * cell) / 2;
	let off_y = height.saturating_sub(rows * cell) / 2;
	(cell, off_x, off_y)
}

/// Pack an [`render::Rgb`] into the `0x00RRGGBB` word softbuffer presents.
fn pack(ink: render::Rgb) -> u32 {
	(u32::from(ink.r) << 16) | (u32::from(ink.g) << 8) | u32::from(ink.b)
}

/// A small size, widened for winit, without a lossy cast or a panic. The grid is
/// tiny; the saturating fallback can never actually be reached.
fn as_u32(n: usize) -> u32 {
	u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fit_centres_a_square_grid_and_letterboxes() {
		// A 4x2 grid into 100x100: the width axis is tighter (100/4=25 < 100/2=50), so the
		// cell is 25, the grid is 100x50, centred -> letterboxed top and bottom.
		let (cell, off_x, off_y) = fit(100, 100, 4, 2);
		assert_eq!(cell, 25, "cell is bound by whichever axis is tighter");
		assert_eq!((off_x, off_y), (0, 25), "centred, letterboxed on the looser axis");
	}

	#[test]
	fn fit_never_returns_a_zero_cell() {
		assert_eq!(fit(1, 1, 48, 34).0, 1, "even a window smaller than the grid keeps a 1px cell");
	}
}
