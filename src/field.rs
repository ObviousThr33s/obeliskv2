//! The spatial container: entities indexed by id and by position.

use std::collections::HashMap;
use crate::entity::{Entity, EntityId, Pos};

/// A 2D field of placed entities — the world's spatial truth.
pub struct Field {
	pub entities:     HashMap<EntityId, Entity>,
	/// Position → id fast lookup. Kept in sync with `entities` on every write.
	spatial_index:    HashMap<(i32, i32), EntityId>,
	/// Next id `mint` hands out. Starts at `1` so minted ids never collide with
	/// the player's fixed [`PLAYER`] (`0`).
	next_id:          EntityId,
}

impl Field {
	pub fn new() -> Self {
		Field { entities: HashMap::new(), spatial_index: HashMap::new(), next_id: 1 }
	}

	/// Hands out a fresh unique [`EntityId`].
	pub fn mint(&mut self) -> EntityId {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	pub fn add(&mut self, entity: Entity) {
		let key = (entity.pos.x, entity.pos.y);
		let id  = entity.id;
		if let Some(old_id) = self.spatial_index.insert(key, id) {
			self.entities.remove(&old_id);
		}
		self.entities.insert(id, entity);
	}

	/// Move entity `id` by `(dx, dy)` if the destination is free. Returns whether
	/// it moved. Every entity is solid, so any occupied cell blocks the step.
	pub fn move_entity(&mut self, id: EntityId, dx: i32, dy: i32) -> bool {
		let mut e = match self.entities.get(&id).cloned() {
			Some(e) => e,
			None    => return false,
		};
		let new_pos = Pos { x: e.pos.x + dx, y: e.pos.y + dy };
		if self.at(new_pos).is_some_and(|other| other.id != id) {
			return false;
		}
		self.spatial_index.remove(&(e.pos.x, e.pos.y));
		e.pos = new_pos;
		self.spatial_index.insert((new_pos.x, new_pos.y), id);
		self.entities.insert(id, e);
		true
	}

	pub fn get(&self, id: EntityId) -> Option<&Entity> {
		self.entities.get(&id)
	}

	pub fn at(&self, pos: Pos) -> Option<&Entity> {
		self.spatial_index.get(&(pos.x, pos.y))
			.and_then(|id| self.entities.get(id))
	}
}

impl Default for Field {
	fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::entity::{Priority, PLAYER};

	fn player(pos: Pos) -> Entity { Entity::new(0, pos, '@', Priority::High) }
	fn wall(id: EntityId, pos: Pos) -> Entity { Entity::new(id, pos, '#', Priority::Low) }

	#[test]
	fn move_entity_steps_into_open_space() {
		let mut f = Field::new();
		f.add(player(Pos { x: 2, y: 2 }));
		assert!(f.move_entity(PLAYER, 0, 1));
		assert_eq!(f.get(PLAYER).unwrap().pos, Pos { x: 2, y: 3 });
		assert!(f.at(Pos { x: 2, y: 3 }).is_some());
		assert!(f.at(Pos { x: 2, y: 2 }).is_none());
	}

	#[test]
	fn move_entity_is_blocked_by_a_wall() {
		let mut f = Field::new();
		f.add(player(Pos { x: 2, y: 2 }));
		f.add(wall(1, Pos { x: 2, y: 1 }));
		assert!(!f.move_entity(PLAYER, 0, -1));
		assert_eq!(f.get(PLAYER).unwrap().pos, Pos { x: 2, y: 2 });
	}
}
