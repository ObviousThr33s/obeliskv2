//! The watcher's memory of what it has seen — vision accumulated over time.
//!
//! [`crate::world::vision::can_see`] answers "is she visible right now?" This answers
//! the gentler question the map needs: "where do I last remember her, and how
//! clearly?"
//!
//! A being you can see is known exactly — full clarity, at her true position.
//! The instant she slips from view she does not vanish; she becomes a *place*
//! you remember, frozen where you last saw her, and that memory fades tick by
//! tick until it is gone.

use std::collections::{BTreeMap, BTreeSet};
use crate::world::entity::EntityId;

/// One remembered sighting: a glyph at the place it was last seen, and how
/// clear that memory still is. `clarity` starts at `1.0` and fades toward `0.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sighting {
	pub glyph:   char,
	pub x:       i32,
	pub y:       i32,
	pub clarity: f32,
}

/// What the watcher remembers of the beings it has seen, keyed by [`EntityId`].
pub struct Recollection {
	sightings: BTreeMap<EntityId, Sighting>,
	/// Clarity lost per tick that a remembered being goes unseen.
	fade: f32,
}

impl Recollection {
	pub fn new(fade: f32) -> Self {
		Self { sightings: BTreeMap::new(), fade }
	}

	/// Record a being seen right now: full clarity at her true position.
	pub fn glimpse(&mut self, id: EntityId, glyph: char, x: i32, y: i32) {
		self.sightings.insert(id, Sighting { glyph, x, y, clarity: 1.0 });
	}

	/// Let one tick of forgetting pass. Every remembered being whose id is not
	/// in `seen_now` fades by `fade`; memories that reach zero are forgotten.
	/// Call once per tick, after glimpsing everything visible this tick.
	pub fn fade_unseen(&mut self, seen_now: &BTreeSet<EntityId>) {
		let fade = self.fade;
		self.sightings.retain(|id, s| {
			if seen_now.contains(id) { return true; }
			s.clarity -= fade;
			s.clarity > 0.0
		});
	}

	/// What the watcher currently remembers, for the map to paint.
	pub fn recalled(&self) -> impl Iterator<Item = (EntityId, &Sighting)> {
		self.sightings.iter().map(|(&id, s)| (id, s))
	}

	pub fn recall(&self, id: EntityId) -> Option<&Sighting> {
		self.sightings.get(&id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn seen(ids: &[EntityId]) -> BTreeSet<EntityId> {
		ids.iter().copied().collect()
	}

	fn clarity(r: &Recollection, id: EntityId) -> f32 {
		r.recall(id).expect("expected a surviving memory").clarity
	}

	#[test]
	fn a_glimpse_is_remembered_exactly() {
		let mut r = Recollection::new(0.25);
		r.glimpse(1, 'F', 10, 10);
		let s = r.recall(1).expect("just glimpsed her");
		assert_eq!((s.glyph, s.x, s.y), ('F', 10, 10));
		assert_eq!(s.clarity, 1.0);
	}

	#[test]
	fn while_seen_she_stays_perfectly_clear() {
		let mut r = Recollection::new(0.25);
		r.glimpse(1, 'F', 10, 10);
		r.fade_unseen(&seen(&[1]));
		assert_eq!(clarity(&r, 1), 1.0);
	}

	#[test]
	fn unseen_she_freezes_in_place_and_fades() {
		let mut r = Recollection::new(0.25);
		r.glimpse(1, 'F', 10, 10);
		r.fade_unseen(&seen(&[]));
		let s = r.recall(1).expect("still faintly remembered");
		assert_eq!((s.x, s.y), (10, 10));
		assert!((s.clarity - 0.75).abs() < 1e-6);
	}

	#[test]
	fn fully_faded_is_forgotten() {
		let mut r = Recollection::new(0.5);
		r.glimpse(2, 'w', 3, 4);
		r.fade_unseen(&seen(&[]));
		assert!(r.recall(2).is_some());
		r.fade_unseen(&seen(&[]));
		assert!(r.recall(2).is_none(), "a memory at zero clarity is no longer anywhere you know");
	}

	#[test]
	fn seeing_her_again_refreshes_and_follows_her() {
		let mut r = Recollection::new(0.25);
		r.glimpse(1, 'F', 1, 1);
		r.fade_unseen(&seen(&[]));
		r.glimpse(1, 'F', 2, 2);
		let s = r.recall(1).expect("seen again");
		assert_eq!((s.x, s.y), (2, 2));
		assert_eq!(s.clarity, 1.0);
	}
}
