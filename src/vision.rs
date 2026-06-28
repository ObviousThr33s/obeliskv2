//! Line-of-sight visibility test — the foundation for "watchers move only when
//! unwatched." Answers one question: can the viewer, facing a given direction,
//! currently see a target cell?
//!
//! Pure function over a `blocked` predicate; easy to test in isolation and
//! reusable by both game logic and rendering.

/// Returns true if `target` is visible from `origin` while facing `facing`
/// (radians; 0 = +x, +y = down on screen). All three gates must pass:
///
/// 1. **Range** — the target is no farther than `range` cells away.
/// 2. **Field of view** — the bearing to the target is within `fov / 2` of facing.
/// 3. **Line of sight** — no `blocked` cell lies between viewer and target.
///
/// `blocked(x, y)` reports whether a cell is opaque. The caller should exclude
/// the viewer and the target themselves.
///
/// ```
/// use obelisk::vision::can_see;
/// use std::f32::consts::PI;
///
/// assert!( can_see((0, 0), 0.0, PI / 3.0, 20.0, (5, 0), |_, _| false));
/// assert!(!can_see((0, 0), 0.0, PI / 3.0, 20.0, (5, 0), |x, y| (x, y) == (3, 0)));
/// ```
pub fn can_see(
	origin:  (i32, i32),
	facing:  f32,
	fov:     f32,
	range:   f32,
	target:  (i32, i32),
	blocked: impl Fn(i32, i32) -> bool,
) -> bool {
	use std::f32::consts::{PI, TAU};

	let (ox, oy) = (origin.0 as f32, origin.1 as f32);
	let (tx, ty) = (target.0 as f32, target.1 as f32);
	let (dx, dy) = (tx - ox, ty - oy);
	let dist = (dx * dx + dy * dy).sqrt();

	if dist > range {
		return false;
	}
	if dist == 0.0 {
		return true;
	}

	let mut diff = dy.atan2(dx) - facing;
	while diff <= -PI { diff += TAU; }
	while diff >   PI { diff -= TAU; }
	if diff.abs() > fov / 2.0 {
		return false;
	}

	let steps = (dist * 2.0).ceil() as i32;
	for i in 1..steps {
		let t    = i as f32 / steps as f32;
		let cell = ((ox + dx * t).round() as i32, (oy + dy * t).round() as i32);
		if cell == target { break; }
		if blocked(cell.0, cell.1) { return false; }
	}

	true
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::PI;

	const FOV:   f32 = PI / 3.0;
	const RANGE: f32 = 20.0;

	#[test]
	fn sees_target_in_clear_line_ahead() {
		assert!(can_see((0, 0), 0.0, FOV, RANGE, (5, 0), |_, _| false));
	}

	#[test]
	fn wall_between_blocks_sight() {
		assert!(!can_see((0, 0), 0.0, FOV, RANGE, (5, 0), |x, y| (x, y) == (3, 0)));
	}

	#[test]
	fn target_behind_viewer_is_unseen() {
		assert!(!can_see((0, 0), 0.0, FOV, RANGE, (-5, 0), |_, _| false));
	}

	#[test]
	fn target_beyond_range_is_unseen() {
		assert!(!can_see((0, 0), 0.0, FOV, 4.0, (5, 0), |_, _| false));
	}
}
