//! `obelisk` — the terminal build, debug off. A thin leaf: the whole driver now
//! lives in [`obelisk::terminal`]; this binary only chooses the `Debug` switch.

fn main() -> std::io::Result<()> {
	obelisk::terminal::run(obelisk::terminal::Debug::OFF)
}
