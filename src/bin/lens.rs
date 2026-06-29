// Hide the console for the windowed build so a double-clicked exe shows only the
// window; kept under `cargo test` so test output still has somewhere to go.
#![cfg_attr(not(test), windows_subsystem = "windows")]

//! Obelisk — the lens. The unified product: a real window showing the game panel
//! *and* the status/spoken text together, both drawn from the one shared
//! [`render::render`] the terminal build uses. Where [`obelisk`] (the terminal) is
//! the reference view and `pane` is the smallest honest CPU-floor slice, this is
//! the destination — where glyphs and the rest of the experience land.
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

use obelisk::world::entity::Pos;
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

/// The heartbeat: how often the render clock asks for a fresh frame. Fast enough that
/// the slow breath reads as smooth, slow enough that the machine mostly sleeps — a
/// steady beat, never a busy spin.
const BEAT: Duration = Duration::from_millis(50);

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

	let mut lens = Lens {
		world,
		frame: Box::new(Frame::blank()),
		start: Instant::now(),
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
		.with_sanctuary(Pos { x: 16, y: 12 }, 2);
	terrain::grow_masses(&mut world.field, 0xB0A7, 40, 24, 0.4);
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
			.with_title("Obelisk — the lens")
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
			_ => {}
		}
	}

	/// The heartbeat. Between events winit calls this; we ask for one fresh frame and
	/// then sleep until the next beat. That steady redraw is what lets render-time
	/// breathe the aura while the world itself holds perfectly still.
	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		if let Some(window) = &self.window {
			window.request_redraw();
		}
		event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + BEAT));
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
			Key::Named(NamedKey::ArrowUp) => Intent::Forward,
			Key::Named(NamedKey::ArrowLeft) => Intent::TurnLeft,
			Key::Named(NamedKey::ArrowRight) => Intent::TurnRight,
			Key::Named(NamedKey::ArrowDown | NamedKey::Space) => Intent::Wait,
			Key::Named(NamedKey::Escape) => {
				event_loop.exit();
				return;
			}
			Key::Character(ref c) => match c.as_str() {
				"w" => Intent::Forward,
				"a" => Intent::TurnLeft,
				"d" => Intent::TurnRight,
				"s" | " " => Intent::Wait,
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

/// Turn the cell grid into lit pixels, stamping each cell's glyph from the atlas.
/// A pixel asks which cell it falls in, then which glyph-pixel it covers (the cell
/// is `CELL`/`GLYPH_W` times the glyph's size). A lit glyph-pixel takes the cell's
/// ink; everything else is [`VOID`], so the field reads as shapes on dark ground.
///
/// A blank cell is bare void; a glyph the atlas doesn't carry yet (e.g. status
/// letters) falls back to a solid block, so it still reads. No allocation, no raw
/// indexing (ward 2, the safe subset): every access is bounds-checked.
///
/// [`VOID`]: render::palette::VOID
fn rasterise<const W: usize, const H: usize>(
	frame:  &Frame<W, H>,
	atlas:  &Atlas,
	width:  usize,
	height: usize,
	pixels: &mut [u32],
) {
	let scale = (CELL / render::atlas::GLYPH_W).max(1);
	let void = pack(render::palette::VOID);
	for py in 0..height {
		let cy = py / CELL;
		let gy = (py % CELL) / scale;
		let row = py * width;
		for px in 0..width {
			let cell = frame.at(px / CELL, cy);
			let glyph = cell.map_or(' ', |c| c.glyph);
			let lit = if glyph == ' ' {
				false // bare ground: the void shows through
			} else if let Some(bm) = atlas.glyph(glyph) {
				let gx = (px % CELL) / scale;
				bm.get(gy).is_some_and(|&bits| bits & (0x80u8 >> gx) != 0)
			} else {
				true // a glyph not yet drawn: a solid block, so it still reads
			};
			let color = if lit { cell.map_or(void, |c| pack(c.ink)) } else { void };
			if let Some(slot) = pixels.get_mut(row + px) {
				*slot = color;
			}
		}
	}
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
