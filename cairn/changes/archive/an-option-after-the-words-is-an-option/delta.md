---
cairn: delta
change: an-option-after-the-words-is-an-option
---

# Delta

## ADDED Requirements

### Requirement: An option after the words is an option
A positional taking several words (the query of `pimdir search`, `pimdir count` and `pimdir tag`, the message `MessageArg` resolves, the script of `sieve put` and `sieve check`, the method calls of `jmap query`) SHALL leave every option an option wherever it stands, before, between or after the words, the global `--json` included, as `envelope search` does. A word opening on a hyphen SHALL go after `--`: a pimdir negation is `-- -from:alice`, or `not from:alice`; given before `--`, it is refused as an unknown option.

#### Scenario: A dry run after the query
- GIVEN a pimdir store whose index holds messages from `alice@x.org`
- WHEN `himalaya pimdir tag --add flagged from:alice@x.org --dry-run` runs
- THEN it reports the plan for the query `from:alice@x.org` and stages nothing

#### Scenario: An option after a pimdir query
- GIVEN a pimdir index holding three messages `report` matches
- WHEN `himalaya pimdir search report -s 2 --json` runs
- THEN it prints the first two hits as JSON with `"nextPage":"2"`

## MODIFIED Requirements

### Requirement: Raw message input is shared
A command taking a raw RFC 5322 message SHALL resolve it through the shared `MessageArg`: its last positional, a file path or the message itself (after `--` when a word of it opens on a hyphen), options read wherever they stand ("An option after the words is an option"), else piped stdin read whole. A positional naming no file SHALL be the message only when it opens on a header field, so a mistyped path is refused rather than stored. Options naming flags SHALL take one value per occurrence, so the positional after them is never read as a flag. The message SHALL be kept as bytes, never decoded, so 8-bit content in any charset reaches the backend intact. The resolved message is normalised to CRLF and rejected when empty, so no backend receives a zero-length message.

## REMOVED Requirements

None.
