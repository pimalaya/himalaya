---
cairn: delta
change: a-utc-instant-ends-with-z
---

# Delta

## ADDED Requirements

### Requirement: An instant has one spelling
Every instant Himalaya writes as RFC 3339 under `--json` (an envelope's `date`, the `date` of `message read` and `message parse`, the `internalDate` of `imap fetch`, the `vacation` dates) SHALL spell a UTC offset `Z`, as pimalaya-cli's output contract does, and keep any other offset as found. A value a provider or a store spells (a Gmail or Graph passthrough, a pimdir date, the `Date:` header `imap fetch` prints in its envelope) SHALL keep its spelling.

#### Scenario: A message sent at UTC
- GIVEN a message whose header reads `Date: Mon, 01 Jan 2024 10:00:00 +0000`
- WHEN `himalaya --json message read` reads it
- THEN `headers.date` is `2024-01-01T10:00:00Z`, as `envelope list` prints its `date`

## MODIFIED Requirements

### Requirement: A read message is a designed view
`headers.date` reads: `date` as RFC 3339 with its offset, `Z` for UTC, or `null`.

## REMOVED Requirements

None.
