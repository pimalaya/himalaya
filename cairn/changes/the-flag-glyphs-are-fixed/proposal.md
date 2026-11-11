---
cairn: change
id: the-flag-glyphs-are-fixed
status: landed
created: 2026-11-11
---

# The flag glyphs are fixed

## Why

Lists are plain columns since plain-text-output, yet the four glyph keys still live under `envelope.list.table`, the one table left in the configuration, naming a shape the output no longer has. The owner decided on 2026-11-11 to remove them from the CLI without making the block strict, so a file written for v2 or shared with himalaya-tui keeps loading. himalaya-tui does not read them today (it styles envelopes through its own `envelope-seen` and `envelope-unread` styles) and its account block tolerates the CLI's keys either way.

## What

- The FLAGS and ATT glyphs become constants: `*` unseen, `R` answered, `!` flagged, `@` attachment, the defaults they had.
- `envelope.list.table` is ignored whatever it holds, its glyph and colour keys included; `envelope.list` keeps refusing an unknown key of its own, so a typo in `datetime-fmt` still fails. `mailbox` colour keys stay refused.
- The `jmap email query|get` commands no longer take the account, which they only read the glyphs from.
- config.sample.toml loses the four glyph lines; MIGRATION.md, CHANGELOG.md and the config spec follow.
