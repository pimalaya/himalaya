---
cairn: delta
change: unknown-keys-are-refused
---

# Delta

## ADDED Requirements

### Requirement: Unknown keys are refused
(config) Every table of the configuration SHALL refuse a key it does not know, the top level and the account blocks included, the error naming the key and the line. The one exception SHALL be the `tui` table, at the top level and in an account, which holds himalaya-tui's own keys: Himalaya SHALL load it whatever it holds and read none of it. A block a previous major version read and v3 dropped (`table`, `attachment`, the top-level `account`, `envelope.list.table`) SHALL be refused naming the block and MIGRATION.md. The v1 `backend` table and the `m2dir` block SHALL load as their requirements say.

#### Scenario: A typo at the top level
- GIVEN a configuration setting `downloads-dri` at the top level
- WHEN any command reads it
- THEN it is refused naming `downloads-dri` and its line

#### Scenario: himalaya-tui keys
- GIVEN a configuration holding `[tui]` with `keybinds`, `contact-command`, `contact-complete-key` and `theme`, and `[accounts.work.tui]` with `contact-command`
- WHEN any command reads it
- THEN it loads, and the same keys outside a `tui` table are refused

## MODIFIED Requirements

### Requirement: A v1 account layout is named, not silently ignored
(config) When no configured backend matches `--backend`, the error SHALL name the account and, under `auto`, the backends the command supports in this build; under a named backend, the missing block, or that the backend is unsupported. An account carrying the v1 `backend` table, which no binary reads since v2, SHALL load as if the table were absent, and that error SHALL name the table and point at MIGRATION.md.

### Requirement: Sent copies are configured per account
(config) `message.send.save-copy`, global or per account, SHALL be the mailbox a sending command (`message send`, and `message compose`, `message reply` and `message forward` with `--send`) appends a copy to when `--save` is not passed. It takes a mailbox name, alias or role, resolved as `--save` is; `true` SHALL stand for `sent` and `false` for no copy. `--no-save` SHALL skip it for one call and conflict with `--save`. The `message` table SHALL refuse an unknown key as every table does, the rest of a v1 `[message]` table (`read`, `write`, `send.backend`) included.

### Requirement: No colour is configured
(config) The configuration SHALL hold no colour, no table shape and no glyph. The v2 blocks holding them SHALL be refused by name as "Unknown keys are refused" says: `table` (`table.preset`, `table.arrangement`) and `attachment` at the top level and the account level, `account` at the top level, and `envelope.list.table`, whatever they hold; `mailbox.list` is an unknown key of `mailbox`. `envelope.list` SHALL keep refusing an unknown key of its own.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN any command reads the configuration
- THEN it is refused with ``The `envelope.list.table` block is removed since v3: delete it, see MIGRATION.md``

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

## REMOVED Requirements
