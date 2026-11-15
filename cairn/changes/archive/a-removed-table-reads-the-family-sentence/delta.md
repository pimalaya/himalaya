---
cairn: delta
change: a-removed-table-reads-the-family-sentence
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Unknown keys are refused
Every table of the configuration SHALL refuse a key it does not know, the top level and the account blocks included, the error naming the key and the line. The one exception SHALL be the `tui` table, at the top level and in an account, which holds himalaya-tui's own keys: Himalaya SHALL load it whatever it holds and read none of it. A table a previous major version read and v3 dropped SHALL be refused by name in pimalaya-config's sentence (`removed_key!`), ``The `<name>` table is removed, <why>: <instead>, see MIGRATION.md``: `table`, `attachment`, the top-level `account` and `envelope.list.table`, `the output being plain text: delete it`, and v1's `folder`, which v2 renamed `mailbox`, ``v2 renaming it `mailbox`: move `folder.alias` to `mailbox.alias` and delete the rest``. The v1 `backend` table and the `m2dir` block SHALL load as their requirements say.

#### Scenario: A typo at the top level
- GIVEN a configuration setting `downloads-dri` at the top level
- WHEN any command reads it
- THEN it is refused naming `downloads-dri` and its line

#### Scenario: A v1 folder alias
- GIVEN an account setting `folder.alias.inbox = "INBOX"`
- WHEN any command reads it
- THEN it is refused with ``The `folder` table is removed, v2 renaming it `mailbox`: move `folder.alias` to `mailbox.alias` and delete the rest, see MIGRATION.md``

#### Scenario: himalaya-tui keys
- GIVEN a configuration holding `[tui]` with `keybinds`, `contact-command`, `contact-complete-key` and `theme`, and `[accounts.work.tui]` with `contact-command`
- WHEN any command reads it
- THEN it loads, and the same keys outside a `tui` table are refused

### Requirement: No colour is configured
The configuration SHALL hold no colour, no table shape and no glyph. The v2 blocks holding them SHALL be refused by name as "Unknown keys are refused" says: `table` (`table.preset`, `table.arrangement`) and `attachment` at the top level and the account level, `account` at the top level, and `envelope.list.table`, whatever they hold; `mailbox.list` is an unknown key of `mailbox`. `envelope.list` SHALL keep refusing an unknown key of its own.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN any command reads the configuration
- THEN it is refused with ``The `envelope.list.table` table is removed, the output being plain text: delete it, see MIGRATION.md``

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

## REMOVED Requirements

None.
