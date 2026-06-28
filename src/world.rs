//! The world and its tick — three phases per press (see `CLAUDE.md`).

use std::collections::HashSet;
use crate::entity::{Entity, EntityId, Heading, Priority, Pos, PLAYER};
use crate::event::Event;
use crate::field::Field;
use crate::haps::Haps;
use crate::lore::Lore;
use crate::recollection::Recollection;
use crate::vision::can_see;

pub const MOTH: EntityId = 1;

/// Clarity the watcher's memory of an unseen being loses each tick.
const FADE: f32 = 0.25;

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
	haps:             Haps,
}

impl World {
	pub fn new(player_pos: Pos, moth_pos: Pos) -> Self {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, player_pos, '@', Priority::High));
		field.add(Entity::new(MOTH,   moth_pos,   'm', Priority::Med));
		World {
			field,
			facing: Heading::East,
			seen: false,
			watching: false,
			recollection: Recollection::new(FADE),
			lore: Lore::default(),
			spoken: None,
			haps: Haps::new(),
		}
	}

	/// Give the world its story. Without it the engine runs in silence.
	pub fn voiced(mut self, lore: Lore) -> Self {
		self.lore = lore;
		self
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
		// The tick she passes out of a gaze that was holding her: the watcher loses her.
		if !sees_moth && self.watching {
			let _ = self.haps.push(Event::Lost { id: MOTH });
		}
		// Unwatched, the moth comes to the light — one step toward the player.
		// Decided here from this tick's snapshot; applied below in the mutation phase.
		if !sees_moth {
			let step = mp.step_toward(pp);
			let (dx, dy) = (step.x - mp.x, step.y - mp.y);
			if (dx, dy) != (0, 0) {
				let _ = self.haps.push(Event::Crept { id: MOTH, dx, dy });
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
}
