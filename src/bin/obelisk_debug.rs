//! `obelisk_debug` — the terminal build, debug on. The same driver as `obelisk`
//! ([`obelisk::terminal::run`]); the only difference is the `Debug` switch it
//! passes. As `Debug` grows from a boolean into tags, only that argument changes.

fn main() -> std::io::Result<()> {
	obelisk::terminal::run(obelisk::terminal::Debug::ON)
}
