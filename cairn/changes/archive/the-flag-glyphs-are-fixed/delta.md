---
cairn: delta
change: the-flag-glyphs-are-fixed
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: No colour is configured
The configuration SHALL hold no colour, no table shape and no glyph: `*.list.table.*-color`, `table.preset` and `table.arrangement` are unknown keys, refused where their table denies unknown keys (`mailbox`) and ignored at the top level and the account level, which tolerate unknown keys for himalaya-tui. The `envelope.list.table` block SHALL be ignored whatever it holds, its v2 glyph and colour keys included, while `envelope.list` keeps refusing an unknown key of its own. A FLAGS cell SHALL hold only the glyphs that apply, `*` for a message lacking `\Seen`, `R` for `\Answered` and `!` for `\Flagged`, and an ATT cell `@` for a message with an attachment.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN `envelope list` runs
- THEN the configuration loads and an unread message shows `*`

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

## REMOVED Requirements

None.
