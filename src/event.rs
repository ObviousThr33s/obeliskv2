use crate::entity::EntityId;

/// What travels the bus. Per the engine wards (see `CLAUDE.md`): a payload
/// carries values and [`EntityId`]s only — never a reference, lifetime, or heap
/// allocation. That keeps [`Event`] `Copy`, trivially queueable, and aliasing-free.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
	/// A being has entered the field of view. The seen entity is named by id.
	Seen { id: EntityId },
	/// A being has crept one cell while unwatched — drawn toward the light.
	/// Carries the mover's id and the step as plain deltas: values only, so the
	/// event stays `Copy` and reference-free (ward 1).
	Crept { id: EntityId, dx: i32, dy: i32 },
	/// A being has passed out of a gaze that was holding it — the watcher loses
	/// her. Named by id only, so the event stays `Copy` and reference-free (ward 1).
	Lost { id: EntityId },
}
