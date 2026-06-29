//! The world and its tick — three phases per press (see `CLAUDE.md`). This is the
//! mechanism trunk (`world/mod.rs`); its submodules are the event bus and everything
//! placed in the field.

pub mod entity;
pub mod event;
pub mod field;
pub mod haps;
pub mod recollection;
pub mod vision;
pub mod terrain;
pub mod watcher;

use std::collections::{HashMap, HashSet};
use crate::world::entity::{Entity, EntityId, Heading, Priority, Pos, PLAYER};
use crate::world::event::Event;
use crate::world::field::Field;
use crate::world::haps::Haps;
use crate::content::lore::Lore;
use crate::world::recollection::Recollection;
use crate::world::vision::can_see;

pub const MOTH: EntityId = 1;

/// The fairy — a visible watcher with a generated name and stats.
pub const FAIRY: EntityId = 2;

/// Clarity the watcher's memory of an unseen being loses each tick.
const FADE: f32 = 0.25;

/// The longest a breath takes — at the faint edge of a pall. Stronger magic (a deeper
/// aura) shortens it, so the breath's tempo is governed by the area, not a flat clock.
const BREATH_BASE: f32 = 6.0;

/// How many ticks a being takes to fade, given the aura `strength` where she rests
/// (`0.0` at the pall's edge, `1.0` at the heart). Deeper in the pall — greater magical
/// astuteness — the faster she fades: the time-stick is shared by the pall's aura.
fn breath_span(strength: f32) -> u32 {
	let span = BREATH_BASE * (1.0 - strength.clamp(0.0, 1.0));
	(span.round() as u32).max(1)
}

/// The player's field of view — a 90° cone. Wide enough to feel watchful,
/// narrow enough that turning away truly looks away.
const SIGHT_FOV: f32 = std::f32::consts::FRAC_PI_2;

/// How far the player can see, in cells.
const SIGHT_RANGE: f32 = 12.0;

/// What the player asks of a single tick. A renderer maps keys to one of these
/// and hands it to [`World::tick`]; the world holds nothing about input itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intent {
	/// Stand still this tick — watch, and let the world move around you.
	Wait,
	/// Step one cell along the way you face.
	Forward,
	/// Turn a quarter-turn left, without moving.
	TurnLeft,
	/// Turn a quarter-turn right, without moving.
	TurnRight,
}

/// A safe spot — the fountain of wisps. Within `radius` cells of `center`
/// (Chebyshev distance, the grid's own reach) the observer-tension rests: the
/// moth does not creep when unwatched, and turning away speaks no parting. A
/// place to set the eye down and breathe; nothing here is lost.
#[derive(Clone, Copy)]
struct Sanctuary {
	center: Pos,
	radius: i32,
}

pub struct World {
	pub field:        Field,
	/// The way the player looks — steers forward steps, and later sight.
	pub facing:       Heading,
	pub seen:         bool,
	/// Whether the moth stood in view as of last tick. Lets the world tell the
	/// moment she slips out of a gaze from a tick where she was never in it.
	watching:         bool,
	pub recollection: Recollection,
	/// The world's story, keyed by moment. Empty until [`voiced`](World::voiced).
	lore:             Lore,
	/// The line the world surfaced this tick, for a renderer to drain. `None`
	/// when nothing was said.
	pub spoken:       Option<String>,
	/// The safe spot, if this world has one — see [`Sanctuary`].
	sanctuary:        Option<Sanctuary>,
	haps:             Haps,
	/// The data half (name, stats) of the watchers placed in the world, keyed by id.
	watchers:         HashMap<EntityId, watcher::Watcher>,
	/// How far the moth has faded into the fountain this breath (0 = whole, fully present).
	breath:           u32,
	/// Where the moth re-emerges when reborn — her birthplace.
	moth_seed:        Pos,
}

impl World {
	pub fn new(player_pos: Pos, moth_pos: Pos) -> Self {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, player_pos, '@', Priority::High));
		field.add(Entity::new(MOTH,   moth_pos,   'm', Priority::Med));
		field.reserve_past(MOTH); // generation mints past the principals, never over them
		World {
			field,
			facing: Heading::East,
			seen: false,
			watching: false,
			recollection: Recollection::new(FADE),
			lore: Lore::default(),
			spoken: None,
			sanctuary: None,
			haps: Haps::new(),
			watchers: HashMap::new(),
			breath: 0,
			moth_seed: moth_pos,
		}
	}

	/// Give the world its story. Without it the engine runs in silence.
	pub fn voiced(mut self, lore: Lore) -> Self {
		self.lore = lore;
		self
	}

	/// Place a safe spot — a fountain of wisps — at `center`, reaching `radius`
	/// cells. Within it the gaze may rest: the moth holds even unwatched, and
	/// looking away grieves nothing. Builder-style, like [`voiced`](World::voiced).
	pub fn with_sanctuary(mut self, center: Pos, radius: i32) -> Self {
		self.sanctuary = Some(Sanctuary { center, radius });
		self
	}

	/// Place a fairy — a visible watcher with a generated name and stats — at `pos`,
	/// grown from `seed`. Builder-style, like [`voiced`](World::voiced). Added before
	/// terrain grows, so she is an anchor it never walls in.
	pub fn with_fairy(mut self, pos: Pos, seed: u64) -> Self {
		self.field.add(Entity::new(FAIRY, pos, 'F', Priority::Med));
		self.field.reserve_past(FAIRY);
		self.watchers.insert(FAIRY, watcher::Watcher::random(seed));
		self
	}

	/// The data (name, stats) of a watcher placed in the world, if any.
	pub fn watcher(&self, id: EntityId) -> Option<&watcher::Watcher> {
		self.watchers.get(&id)
	}

	/// Whether `pos` lies within the safe spot — anywhere the aura still has
	/// strength. `false` when this world has no fountain.
	pub fn is_safe(&self, pos: Pos) -> bool {
		self.aura_at(pos) > 0.0
	}

	/// The fountain's aura strength at `pos`: `1.0` at its heart, fading to `0.0`
	/// at the edge of its pall (Euclidean — a round glow), and `0.0` beyond it or
	/// when there is no fountain. One field, shared: the render paints this number
	/// and the world reads it, so what is seen is exactly what is felt.
	#[allow(clippy::cast_precision_loss)] // grid coords are small; the loss is nothing
	pub fn aura_at(&self, pos: Pos) -> f32 {
		match self.sanctuary {
			Some(s) => {
				let dx = (pos.x - s.center.x) as f32;
				let dy = (pos.y - s.center.y) as f32;
				let reach = s.radius as f32 + 1.0;
				(1.0 - (dx * dx + dy * dy).sqrt() / reach).max(0.0)
			}
			None => 0.0,
		}
	}

	/// The safe spot's heart and reach, for a renderer to paint. `None` when unset.
	pub fn sanctuary(&self) -> Option<(Pos, i32)> {
		self.sanctuary.map(|s| (s.center, s.radius))
	}

	/// One press, one tick — three phases. The player's [`Intent`] drives phase 1;
	/// the world reacts in phases 2 and 3.
	pub fn tick(&mut self, intent: Intent) {
		let mp = match self.field.get(MOTH) {
			Some(m) => m.pos,
			None    => return,
		};

		// Phase 1: Read — apply the player's intent, push events; nothing else mutates.
		match intent {
			Intent::TurnLeft  => self.facing = self.facing.turn_left(),
			Intent::TurnRight => self.facing = self.facing.turn_right(),
			Intent::Forward   => {
				let (dx, dy) = self.facing.delta();
				self.field.move_entity(PLAYER, dx, dy);
			}
			Intent::Wait => {}
		}
		// Sight is the law of this world: the moth holds while watched, and moves
		// only when she isn't. We read it from the player's true facing now.
		let pp = match self.field.get(PLAYER) {
			Some(p) => p.pos,
			None    => return,
		};
		let sees_moth = self.sees(pp, mp);
		if sees_moth && !self.seen {
			let _ = self.haps.push(Event::Seen { id: MOTH });
		}
		// The tick she passes out of a gaze that was holding her: the watcher loses her —
		// unless this is the safe spot, where turning away costs nothing.
		if !sees_moth && self.watching && !self.is_safe(pp) {
			let _ = self.haps.push(Event::Lost { id: MOTH });
		}
		// Unwatched, the moth is drawn to the light — toward the fountain's wisp if there
		// is one (there she fades and is reborn: the breath), else toward the player.
		// Decided from this tick's snapshot; applied below in the mutation phase.
		if !sees_moth {
			if self.is_safe(mp) {
				// Home in the pall: one tick of the fade, or rebirth once fully faded.
				let beat = if self.breath + 1 >= breath_span(self.aura_at(mp)) {
					Event::Reborn { id: MOTH }
				} else {
					Event::Fade { id: MOTH }
				};
				let _ = self.haps.push(beat);
			} else {
				let target = self.sanctuary.map_or(pp, |s| s.center);
				let step = mp.step_toward(target);
				let (dx, dy) = (step.x - mp.x, step.y - mp.y);
				if (dx, dy) != (0, 0) {
					let _ = self.haps.push(Event::Crept { id: MOTH, dx, dy });
				}
			}
		}

		// Phases 2 + 3: Dispatch then Mutate — drain and apply; no new pushes here.
		while let Some(event) = self.haps.pop() {
			match event {
				Event::Seen { id } if id == MOTH => {
					self.seen = true;
					// The moment earns its line, if the story has one to say.
					self.spoken = self.lore.line("seen.moth").map(str::to_owned);
				}
				Event::Lost { id } if id == MOTH => {
					// Losing her earns its own line, if the story has one to say.
					self.spoken = self.lore.line("lost.moth").map(str::to_owned);
				}
				Event::Crept { id, dx, dy } => {
					self.field.move_entity(id, dx, dy);
				}
				Event::Fade { id } if id == MOTH => {
					self.breath = self.breath.saturating_add(1);
				}
				Event::Reborn { id } if id == MOTH => {
					// Re-emerge at her seed; the breath begins again.
					self.breath = 0;
					if let Some(cur) = self.field.get(MOTH).map(|m| m.pos) {
						self.field.move_entity(MOTH, self.moth_seed.x - cur.x, self.moth_seed.y - cur.y);
					}
				}
				_ => {}
			}
		}

		// Mutation: the watcher's memory follows what it can see this tick — glimpse
		// the visible, then let the rest fade. Shared state, so it moves only here.
		let mut seen_now = HashSet::new();
		if sees_moth {
			if let Some(m) = self.field.get(MOTH) {
				self.recollection.glimpse(MOTH, m.glyph, m.pos.x, m.pos.y);
				seen_now.insert(MOTH);
			}
		}
		self.recollection.fade_unseen(&seen_now);
		// Remember whether she stood in view this tick, to catch the loss next tick.
		self.watching = sees_moth;
	}

	/// Whether the player, from `from` and facing [`facing`](World::facing), can
	/// currently see the cell `target`. A cell is opaque if a being other than the
	/// player or the moth stands in it — walls block the line; the two principals
	/// never occlude themselves.
	fn sees(&self, from: Pos, target: Pos) -> bool {
		can_see(
			(from.x, from.y),
			self.facing.to_radians(),
			SIGHT_FOV,
			SIGHT_RANGE,
			(target.x, target.y),
			|x, y| {
				self.field
					.at(Pos { x, y })
					.is_some_and(|e| e.id != PLAYER && e.id != MOTH)
			},
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_fairy_stands_in_the_world_named_and_visible() {
		let w = World::new(Pos { x: 0, y: 0 }, Pos { x: 9, y: 9 })
			.with_fairy(Pos { x: 3, y: 3 }, 0xFA12);
		let f = w.field.at(Pos { x: 3, y: 3 }).expect("the fairy stands where placed");
		assert_eq!((f.id, f.glyph), (FAIRY, 'F'), "she appears as F");
		let data = w.watcher(FAIRY).expect("the fairy carries data");
		assert!(!data.name.is_empty() && data.health > 0, "named, with living stats");
	}

	#[test]
	fn turning_then_stepping_forward_moves_the_way_you_face() {
		// Player starts facing East; the moth sits far off, out of the way.
		let mut w = World::new(Pos { x: 5, y: 5 }, Pos { x: 40, y: 40 });
		w.tick(Intent::Forward);
		assert_eq!(w.field.get(PLAYER).unwrap().pos, Pos { x: 6, y: 5 }, "east is +x");
		w.tick(Intent::TurnRight); // East → South, no step
		assert_eq!(w.field.get(PLAYER).unwrap().pos, Pos { x: 6, y: 5 }, "a turn does not move you");
		w.tick(Intent::Forward);
		assert_eq!(w.field.get(PLAYER).unwrap().pos, Pos { x: 6, y: 6 }, "south is +y, down on screen");
	}

	#[test]
	fn the_moth_holds_while_watched_and_creeps_when_unwatched() {
		// Player at the origin facing East; the moth sits five cells dead ahead,
		// squarely in the field of view.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 });
		w.tick(Intent::Wait);
		assert_eq!(
			w.field.get(MOTH).unwrap().pos, Pos { x: 5, y: 0 },
			"under your gaze, she holds perfectly still",
		);
		// Turn to face South — the moth, due east, leaves the cone. Now unwatched,
		// she takes one step toward you: she comes to the light.
		w.tick(Intent::TurnRight);
		assert_eq!(
			w.field.get(MOTH).unwrap().pos, Pos { x: 4, y: 0 },
			"unwatched, she creeps one cell toward the light",
		);
	}

	#[test]
	fn meeting_her_eyes_again_stops_her() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 });
		w.tick(Intent::TurnRight); // look away — she creeps to (4, 0)
		w.tick(Intent::TurnLeft);  // face her again — she is back in view, and holds
		let watched = w.field.get(MOTH).unwrap().pos;
		w.tick(Intent::Wait);
		assert_eq!(
			w.field.get(MOTH).unwrap().pos, watched,
			"back under your gaze, she is still again",
		);
	}

	#[test]
	fn walking_up_to_the_moth_brings_you_adjacent() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 4, y: 0 });
		for _ in 0..3 { w.tick(Intent::Forward); }
		let pp = w.field.get(PLAYER).unwrap().pos;
		let mp = w.field.get(MOTH).unwrap().pos;
		assert!(pp.adjacent(mp));
		assert!(w.seen);
	}

	#[test]
	fn first_sight_of_the_moth_gives_the_world_its_voice() {
		let lore = Lore::parse("seen.moth = She is here, and she knows your eye.");
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 3, y: 0 }).voiced(lore);
		for _ in 0..2 { w.tick(Intent::Forward); }
		assert_eq!(w.spoken.as_deref(), Some("She is here, and she knows your eye."));
	}

	#[test]
	fn losing_her_from_view_gives_the_world_its_parting_line() {
		let lore = Lore::parse("lost.moth = She is gone the moment you turn.");
		// Player at origin facing East; the moth five cells dead ahead, in view.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 }).voiced(lore);
		w.tick(Intent::Wait); // she is watched and held — nothing is lost yet
		assert_eq!(w.spoken, None, "while she is held, nothing has parted");
		// Turn away — she leaves the cone, and the world speaks her going.
		w.tick(Intent::TurnRight);
		assert_eq!(
			w.spoken.as_deref(),
			Some("She is gone the moment you turn."),
			"the tick she slips from view, the world voices the loss",
		);
	}

	#[test]
	fn a_moth_never_watched_is_never_grieved() {
		// She sits behind the player, never in the cone; turning away can lose
		// nothing, because nothing was ever held.
		let lore = Lore::parse("lost.moth = She is gone the moment you turn.");
		let mut w = World::new(Pos { x: 10, y: 0 }, Pos { x: 0, y: 0 }).voiced(lore);
		w.tick(Intent::Wait);
		w.tick(Intent::TurnRight);
		assert_eq!(w.spoken, None, "you cannot lose what your gaze never held");
	}

	#[test]
	fn a_voiceless_world_still_sees_in_silence() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 3, y: 0 });
		for _ in 0..2 { w.tick(Intent::Forward); }
		assert!(w.seen, "sight still happens");
		assert_eq!(w.spoken, None, "but a world with no lore says nothing");
	}

	#[test]
	fn the_moth_is_remembered_where_she_was_last_seen() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 4, y: 0 });
		for _ in 0..3 { w.tick(Intent::Forward); }
		let s = w.recollection.recall(MOTH).expect("the moth has been seen by now");
		assert_eq!((s.x, s.y), (4, 0), "remembered at her true place");
		assert_eq!(s.clarity, 1.0, "still in view, so perfectly clear");
	}

	#[test]
	fn seen_fires_exactly_once_no_matter_how_many_ticks_follow() {
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 2, y: 0 });
		w.tick(Intent::Forward); // reach (1,0), adjacent to the moth
		for _ in 0..20 { w.tick(Intent::Wait); }
		assert!(w.seen);
		assert!(w.haps.is_empty());
	}

	#[test]
	fn unwatched_the_moth_is_drawn_toward_the_fountain() {
		// A fountain at the origin, the player on its heart, the moth east. Look away and
		// she is drawn toward the wisp — a step nearer the heart (the breath, evolved from
		// the old "she comes to the player").
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 6, y: 0 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 2);
		w.tick(Intent::Wait);      // watched, she holds
		w.tick(Intent::TurnRight); // look away — drawn toward the fountain's wisp
		assert_eq!(
			w.field.get(MOTH).unwrap().pos, Pos { x: 5, y: 0 },
			"unwatched, she steps toward the fountain",
		);
	}

	#[test]
	fn beyond_the_pall_she_is_drawn_to_the_fountain_not_the_player() {
		// A fountain at the origin; the player and moth stand far off, beyond its reach.
		let mut w = World::new(Pos { x: 20, y: 20 }, Pos { x: 25, y: 20 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 2);
		w.tick(Intent::Wait);      // watched, she holds
		w.tick(Intent::TurnRight); // look away — drawn toward the distant fountain
		assert_eq!(
			w.field.get(MOTH).unwrap().pos, Pos { x: 24, y: 19 },
			"she steps toward the fountain at the origin, not the player beside her",
		);
	}

	#[test]
	fn the_fountain_aura_is_strongest_at_its_heart_and_fades_to_nothing() {
		let w = World::new(Pos { x: 0, y: 0 }, Pos { x: 40, y: 40 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 2);
		assert!((w.aura_at(Pos { x: 0, y: 0 }) - 1.0).abs() < 1e-6, "full at the heart");
		let near = w.aura_at(Pos { x: 1, y: 0 });
		let far = w.aura_at(Pos { x: 2, y: 0 });
		assert!(near < 1.0 && near > far && far > 0.0, "fades with distance, still within the pall");
		assert_eq!(w.aura_at(Pos { x: 5, y: 0 }), 0.0, "nothing beyond the pall");

		let bare = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 });
		assert_eq!(bare.aura_at(Pos { x: 0, y: 0 }), 0.0, "no fountain, no aura");
	}

	#[test]
	fn the_safe_spot_grieves_no_parting() {
		let lore = Lore::parse("lost.moth = She is gone the moment you turn.");
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 })
			.voiced(lore)
			.with_sanctuary(Pos { x: 0, y: 0 }, 2);
		w.tick(Intent::Wait);      // held and watched
		w.tick(Intent::TurnRight); // turn away — she is right there, not lost
		assert_eq!(w.spoken, None, "in the safe spot, turning away parts from nothing");
	}

	#[test]
	fn unwatched_the_moth_breathes_into_the_fountain_and_is_reborn() {
		// The breath, as one cycle: player on the heart of a wide fountain, the moth
		// seeded outside its pall. Hold the gaze aside and she drifts in, fades over the
		// breath, then is reborn at her seed — all driven by events on the bus.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 6, y: 0 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 3); // pall reaches ~3 cells; the seed (6,0) is outside
		w.tick(Intent::TurnRight); // face south; the moth, due east, stays unwatched throughout
		let mut entered = false;
		let mut reborn = false;
		for _ in 0..40 {
			w.tick(Intent::Wait);
			let mp = w.field.get(MOTH).expect("the moth is in the world").pos;
			if w.is_safe(mp) {
				entered = true;
			}
			if entered && mp == (Pos { x: 6, y: 0 }) {
				reborn = true;
				break;
			}
		}
		assert!(entered, "she is drawn into the fountain's pall");
		assert!(reborn, "and once fully faded, she is reborn at her seed");
	}

	#[test]
	fn the_breath_quickens_where_the_magic_is_stronger() {
		// The time-stick shared by the pall's astuteness: a deeper aura fades her faster.
		assert!(
			breath_span(0.9) < breath_span(0.1),
			"stronger magic shortens the breath",
		);
		assert!(breath_span(1.0) >= 1, "even at the heart, a breath takes at least a tick");
	}
}
