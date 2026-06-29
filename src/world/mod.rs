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

use std::collections::{BTreeMap, BTreeSet};
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
	/// Step one cell in a cardinal direction, facing that way — simple directional
	/// control. (Tank-style `Forward`/`TurnLeft`/`TurnRight` stay, for a later mode.)
	Step(Heading),
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
	watchers:         BTreeMap<EntityId, watcher::Watcher>,
	/// How far each watcher has faded into the fountain this breath (0 = fully present),
	/// keyed by id — every watcher breathes her own cycle.
	breath:           BTreeMap<EntityId, u32>,
}

impl World {
	pub fn new(player_pos: Pos, moth_pos: Pos) -> Self {
		let mut field = Field::new();
		field.add(Entity::new(PLAYER, player_pos, '@', Priority::High));
		field.add(Entity::new(MOTH,   moth_pos,   'm', Priority::Med));
		field.reserve_past(MOTH); // generation mints past the principals, never over them
		// The moth is a watcher too, so she breathes by her own astuteness like the rest.
		let mut watchers = BTreeMap::new();
		watchers.insert(MOTH, watcher::Watcher { name: "the moth".to_string(), health: 8, vigor: 8, power: 6 });
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
			watchers,
			breath: BTreeMap::new(),
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

	/// Re-emerge a faded watcher from the fountain — at a clear cell just outside its
	/// pall, so she drifts back inward and the breath repeats. No fountain, no change.
	fn emerge_from_fountain(&mut self, id: EntityId) {
		let Some(s) = self.sanctuary else {
			return;
		};
		let Some(cur) = self.field.get(id).map(|e| e.pos) else {
			return;
		};
		let r = s.radius + 1;
		for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
			let to = Pos { x: s.center.x + dx, y: s.center.y + dy };
			if to == cur || self.field.at(to).is_none() {
				self.field.move_entity(id, to.x - cur.x, to.y - cur.y);
				return;
			}
		}
	}

	/// Hallow the fountain's ground: clear any stone within the pall and its rim, so a
	/// watcher can drift through and re-emerge — the fountain made safe. Call after
	/// terrain has grown; the principals (player, watchers) are never touched.
	pub fn hallow_sanctuary(&mut self) {
		let Some(s) = self.sanctuary else {
			return;
		};
		let r = s.radius + 1;
		let stones: Vec<EntityId> = self
			.field
			.entities
			.values()
			.filter(|e| {
				e.priority == Priority::Low
					&& (e.pos.x - s.center.x).abs() <= r
					&& (e.pos.y - s.center.y).abs() <= r
			})
			.map(|e| e.id)
			.collect();
		for id in stones {
			self.field.remove(id);
		}
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
			Intent::Step(heading) => {
				// Simple directional: face the way you go, then step there.
				self.facing = heading;
				let (dx, dy) = heading.delta();
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
		// Every watcher, unwatched, is drawn to the fountain and breathes there: in the
		// pall she fades — faster where the aura is strong and her astuteness great — and
		// is reborn; outside it she drifts toward the wisp (or the player, with no fountain).
		// Decided from this snapshot; applied in the mutation phase.
		let breathers: Vec<EntityId> = self.watchers.keys().copied().collect();
		for id in breathers {
			let wp = match self.field.get(id) {
				Some(e) => e.pos,
				None    => continue,
			};
			if self.sees(pp, wp) {
				continue; // watched: she holds (observer-collapse)
			}
			let strength = self.aura_at(wp);
			if strength > 0.0 {
				let astute = self.watchers.get(&id).map_or(1.0, watcher::Watcher::astuteness);
				let progress = self.breath.get(&id).copied().unwrap_or(0);
				if progress + 1 >= breath_span(strength * astute) {
					// Reborn from the fountain: mended whole. The clamp lives in
					// `Watcher::toll`, so a full mend is just "+vigor" — overshoot is honest.
					let vigor = self.watchers.get(&id).map_or(0, |w| w.vigor);
					let _ = self.haps.push(Event::Reborn { id });
					let _ = self.haps.push(Event::Toll { id, delta: vigor });
				} else {
					// One tick deeper into the wisp drains a point of vigor.
					let _ = self.haps.push(Event::Fade { id });
					let _ = self.haps.push(Event::Toll { id, delta: -1 });
				}
			} else {
				let target = self.sanctuary.map_or(pp, |s| s.center);
				let step = wp.step_toward(target);
				let (dx, dy) = (step.x - wp.x, step.y - wp.y);
				if (dx, dy) != (0, 0) {
					let _ = self.haps.push(Event::Crept { id, dx, dy });
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
				Event::Fade { id } => {
					*self.breath.entry(id).or_insert(0) += 1;
				}
				Event::Reborn { id } => {
					// Re-emerge from the fountain; the breath begins again.
					self.breath.insert(id, 0);
					self.emerge_from_fountain(id);
				}
				Event::Toll { id, delta } => {
					// The one place a stat moves: a queued change lands on her vigor,
					// clamped honest. Mutation only, as the ward demands.
					if let Some(w) = self.watchers.get_mut(&id) {
						w.toll(delta);
					}
				}
				_ => {}
			}
		}

		// Mutation: the watcher's memory follows what it can see this tick — glimpse
		// the visible, then let the rest fade. Shared state, so it moves only here.
		let mut seen_now = BTreeSet::new();
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
	fn a_directional_step_moves_and_faces_that_way() {
		let mut w = World::new(Pos { x: 5, y: 5 }, Pos { x: 40, y: 40 });
		w.tick(Intent::Step(Heading::South));
		assert_eq!(w.field.get(PLAYER).unwrap().pos, Pos { x: 5, y: 6 }, "south is +y");
		assert_eq!(w.facing, Heading::South, "and you face the way you moved");
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
	fn unwatched_a_watcher_breathes_into_the_fountain_and_re_emerges() {
		// One breath: player on the heart of a wide fountain, the moth east and outside
		// the pall. Hold the gaze aside and she drifts in, fades, then re-emerges from the
		// fountain's rim (radius + 1) — all driven by events on the bus.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 6, y: 0 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 3);
		w.tick(Intent::TurnRight); // face south; the moth, due east, stays unwatched throughout
		let mut entered = false;
		let mut re_emerged = false;
		for _ in 0..40 {
			w.tick(Intent::Wait);
			let mp = w.field.get(MOTH).expect("the moth is in the world").pos;
			if w.is_safe(mp) {
				entered = true;
			}
			if entered && mp == (Pos { x: 4, y: 0 }) {
				re_emerged = true; // reborn at the rim, having faded inside
				break;
			}
		}
		assert!(entered, "she is drawn into the fountain's pall");
		assert!(re_emerged, "and once faded, she re-emerges from the fountain's rim");
	}

	#[test]
	fn the_breath_is_shared_by_all_watchers_not_only_the_moth() {
		// The fairy, in a pall and unwatched, is drawn in and breathes too.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 40, y: 40 }) // the moth far off
			.with_sanctuary(Pos { x: 0, y: 0 }, 3)
			.with_fairy(Pos { x: 6, y: 0 }, 0xFA12); // the fairy east, outside the pall
		w.tick(Intent::TurnRight); // face south; the fairy (due east) is unwatched
		let mut fairy_breathed = false;
		for _ in 0..40 {
			w.tick(Intent::Wait);
			let fp = w.field.get(FAIRY).expect("the fairy stands in the world").pos;
			if w.is_safe(fp) {
				fairy_breathed = true;
				break;
			}
		}
		assert!(fairy_breathed, "the fairy is drawn into the pall and breathes, like the moth");
	}

	#[test]
	fn fading_into_the_fountain_drains_her_vigor_then_rebirth_mends_it() {
		// The stat interaction, carried entirely on the bus: unwatched in the pall she
		// fades — each fade tolls a point of vigor — and when she is reborn from the rim
		// the toll restores her whole. Health dips below her vigor, then returns to it.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 6, y: 0 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 3);
		let vigor = w.watcher(MOTH).expect("the moth carries stats").vigor;
		w.tick(Intent::TurnRight); // face south; the moth, due east, stays unwatched

		let mut dipped = false;
		let mut mended_after_dip = false;
		for _ in 0..60 {
			w.tick(Intent::Wait);
			let health = w.watcher(MOTH).expect("the moth carries stats").health;
			assert!(health >= 0 && health <= vigor, "vigor stays within 0..=vigor at all times");
			if health < vigor {
				dipped = true; // a fade has tolled her below whole
			}
			if dipped && health == vigor {
				mended_after_dip = true; // a rebirth has mended her back to full
				break;
			}
		}
		assert!(dipped, "fading into the wisp drains her vigor");
		assert!(mended_after_dip, "reborn from the fountain, she is mended whole");
	}

	#[test]
	fn a_watched_watcher_is_never_tolled() {
		// Stats move only by the breath, and the breath only turns when she is unwatched.
		// Hold the moth in the gaze and her vigor never stirs.
		let mut w = World::new(Pos { x: 0, y: 0 }, Pos { x: 5, y: 0 })
			.with_sanctuary(Pos { x: 0, y: 0 }, 3);
		let before = w.watcher(MOTH).expect("stats").health;
		for _ in 0..10 {
			w.tick(Intent::Wait); // facing east, she is dead ahead and watched throughout
		}
		assert_eq!(w.watcher(MOTH).expect("stats").health, before, "under the gaze, her vigor holds");
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
