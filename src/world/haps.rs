use crate::world::event::Event;

/// A fixed-capacity ring buffer of [`Event`]s — the engine's event bus.
///
/// Allocated once; never grows. `CAP` slots, zero heap traffic in the hot loop
/// (ward 2 in `CLAUDE.md`). Events come back FIFO via [`pop`](Self::pop).
/// An overflowing [`push`](Self::push) returns the event as `Err` rather than
/// growing the buffer, so saturation is explicit and observable.
pub struct Haps<const CAP: usize = 64> {
	/// A live slot is `Some`, a free one `None`. `Option` lets the array
	/// initialise to empty slots without `unsafe` and without a sham Nil variant.
	slots: [Option<Event>; CAP],
	/// Index of the oldest queued event — the next one [`pop`](Self::pop) returns.
	head: usize,
	/// Index the next [`push`](Self::push) writes to.
	tail: usize,
	/// How many slots are live, so head/tail wrap without aliasing.
	len: usize,
}

impl<const CAP: usize> Haps<CAP> {
	/// An empty queue. `const` so it can be built in a `const` context.
	pub const fn new() -> Self {
		Self { slots: [None; CAP], head: 0, tail: 0, len: 0 }
	}

	/// Queue an event at the tail. Returns `Err(event)` if the ring is full —
	/// the caller decides what to do with overflow rather than the buffer growing.
	pub fn push(&mut self, event: Event) -> Result<(), Event> {
		if self.len == CAP {
			return Err(event);
		}
		match self.slots.get_mut(self.tail) {
			Some(slot) => *slot = Some(event),
			None => return Err(event),
		}
		self.tail = (self.tail + 1) % CAP;
		self.len += 1;
		Ok(())
	}

	/// Take the oldest queued event, or `None` if empty (FIFO).
	pub fn pop(&mut self) -> Option<Event> {
		if self.len == 0 {
			return None;
		}
		let event = self.slots.get_mut(self.head).and_then(Option::take);
		self.head = (self.head + 1) % CAP;
		self.len -= 1;
		event
	}

	/// How many events are currently queued.
	pub fn len(&self) -> usize {
		self.len
	}

	/// Whether the queue holds no events.
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Whether the queue is at capacity — the next [`push`](Self::push) will `Err`.
	pub fn is_full(&self) -> bool {
		self.len == CAP
	}
}

impl<const CAP: usize> Default for Haps<CAP> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pops_events_in_the_order_they_arrived() {
		let mut haps = Haps::<4>::new();
		haps.push(Event::Seen { id: 1 }).unwrap();
		haps.push(Event::Seen { id: 2 }).unwrap();

		assert_eq!(haps.pop(), Some(Event::Seen { id: 1 }));
		assert_eq!(haps.pop(), Some(Event::Seen { id: 2 }));
		assert_eq!(haps.pop(), None, "draining an empty queue yields None");
	}

	#[test]
	fn a_full_ring_rejects_the_overflow_instead_of_growing() {
		let mut haps = Haps::<2>::new();
		haps.push(Event::Seen { id: 1 }).unwrap();
		haps.push(Event::Seen { id: 2 }).unwrap();
		assert!(haps.is_full());

		assert_eq!(haps.push(Event::Seen { id: 3 }), Err(Event::Seen { id: 3 }));
		assert_eq!(haps.len(), 2, "a rejected push must not grow the ring");
	}

	#[test]
	fn the_ring_wraps_around_its_capacity() {
		let mut haps = Haps::<2>::new();
		haps.push(Event::Seen { id: 1 }).unwrap();
		haps.push(Event::Seen { id: 2 }).unwrap();
		assert_eq!(haps.pop(), Some(Event::Seen { id: 1 }));

		haps.push(Event::Seen { id: 3 }).unwrap();
		assert_eq!(haps.pop(), Some(Event::Seen { id: 2 }));
		assert_eq!(haps.pop(), Some(Event::Seen { id: 3 }));
		assert!(haps.is_empty());
	}
}
