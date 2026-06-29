//! A simple GLSL parser — extracts the constructs the engine cares about so
//! shader-authored data can cross in. Deliberately minimal: it reads `vecN(...)`
//! float literals and the identifier each is bound to (if any). It is *not* a full
//! GLSL grammar — a starting point that grows as the lensing work needs it. Pure
//! and never panics (the safe subset).

/// A parsed GLSL vector literal: its float components, and the name it was bound to
/// (e.g. `amber` in `const vec3 amber = vec3(...)`), if any.
#[derive(Clone, Debug, PartialEq)]
pub struct GlslVec {
	pub name:       Option<String>,
	pub components: Vec<f32>,
}

/// Parse every `vecN(...)` literal in `src` — one [`GlslVec`] per line that has one.
pub fn parse(src: &str) -> Vec<GlslVec> {
	src.lines().filter_map(parse_vec).collect()
}

/// Parse the first `vec2/3/4(...)` literal on one line, with its bound name if any.
fn parse_vec(line: &str) -> Option<GlslVec> {
	let (at, kw_len) = ["vec2(", "vec3(", "vec4("]
		.iter()
		.find_map(|kw| line.find(kw).map(|pos| (pos, kw.len())))?;
	let after = line.get(at + kw_len..)?;
	let close = after.find(')')?;
	let inside = after.get(..close)?;
	let components: Vec<f32> = inside
		.split(',')
		.filter_map(|t| t.trim().trim_end_matches('f').parse::<f32>().ok())
		.collect();
	if components.is_empty() {
		return None;
	}
	let name = line.get(..at).and_then(name_before);
	Some(GlslVec { name, components })
}

/// The identifier bound on the left of a `vecN(...)` — the last name-like token
/// before it (handling `name =`, `const vec3 name =`, `#define NAME`). Type keywords
/// are not names. `None` if there is no plain identifier there.
fn name_before(lhs: &str) -> Option<String> {
	let token = lhs
		.split(|c: char| c == '=' || c == ' ' || c == '\t')
		.filter(|t| !t.is_empty())
		.last()?;
	let is_ident = !token.is_empty() && token.chars().all(|c| c.is_alphanumeric() || c == '_');
	if is_ident && !is_keyword(token) {
		Some(token.to_string())
	} else {
		None
	}
}

fn is_keyword(s: &str) -> bool {
	matches!(s, "vec2" | "vec3" | "vec4" | "const" | "uniform" | "in" | "out")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_a_named_vec3() {
		let v = parse("const vec3 amber = vec3(1.0, 0.61, 0.13);");
		assert_eq!(v.len(), 1);
		assert_eq!(v.first().and_then(|g| g.name.as_deref()), Some("amber"));
		assert_eq!(v.first().map(|g| g.components.clone()), Some(vec![1.0, 0.61, 0.13]));
	}

	#[test]
	fn parses_a_define_and_a_bound_output() {
		let src = "#define BG vec4(0.1, 0.2, 0.3, 1.0)\ngl_FragColor = vec3(0.0f, 0.0f, 0.0f);";
		let v = parse(src);
		assert_eq!(v.len(), 2);
		assert_eq!(v.first().and_then(|g| g.name.as_deref()), Some("BG"));
		assert_eq!(v.first().map(|g| g.components.len()), Some(4));
		assert_eq!(v.get(1).and_then(|g| g.name.as_deref()), Some("gl_FragColor"));
	}

	#[test]
	fn a_line_without_a_vec_yields_nothing() {
		assert!(parse("float x = 3.0;").is_empty());
	}
}
