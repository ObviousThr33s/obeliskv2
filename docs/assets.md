# Assets — the doctrine

One rule, for every asset Obelisk uses:

> **Every asset is a human-authored data file under `assets/`, baked into the binary
> (`include_str!` / `include_bytes!`), and turned into runtime form by a pure parser
> (`&str`/`&[u8] → T`) that returns a typed error, never panics. Nothing is read from
> disk at runtime.**

Covers lore, entities (`.being`), and glyphs (the bitmap atlas) alike — one philosophy,
no exceptions.

*Why:*
- **Single self-contained exe.** Baked-in means nothing to ship alongside, nothing to
  misplace (see [distribution.md](distribution.md)).
- **Content is data, not code.** The story/art lives at the file boundary, never hardcoded
  (CLAUDE.md). A new being, line, or glyph is *new data*, not new code.
- **Identical everywhere.** No host file lookup, no font resolution — the bytes are ours.

*The seam:* `assets/` holds the data; `src/content/` and `src/render/atlas` hold the
parsers. Authoring edits a data file; the parser turns it into runtime form; the build
bakes it in.

*Lineage:* ported from Obelisk v1's `.being` pattern — but **baked**, not `std::fs`-loaded
at runtime (see [aetherium-spinoff.md](aetherium-spinoff.md) for what else came from v1).
