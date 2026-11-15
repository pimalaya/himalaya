---
cairn: delta
change: pimdir-follows-draft-10
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir is a read and write backend over an io-pimdir store (backends)

- `SetFlags` carries the whole set the message is left with. An add or a remove, the `\Seen` a read stages included, SHALL state as `base` the set it was read from (pimdir STORAGE §15.3); a `set` SHALL state none.
- A `Remove` SHALL name no `if_match` and no `resolves`: the owner parks it over a message a sync left in conflict.
- The relayed codes gain `store-reset` and `lock-replaced`.

#### Scenario: A flag synced while an add waits
- GIVEN a message carrying no flag, and `himalaya flag add` staging a keyword on it
- WHEN a sync lands `\Seen` on the message before the owner drains the queue
- THEN the message holds `\Seen` and the keyword once drained
- AND the same change staged by `flag set` leaves the keyword alone

### Requirement: The pimdir subcommand shows and cancels queued rows (backends)

- `queue show` states: `pending`, `parked`, `unconfirmed`, `applied`, `unknown`, `reset`. An unconfirmed row carries `collection`, `kind`, `attempts` and, as `error`, what its performer recorded.
- An applied row whose mailbox is gone since reads `applied` with `collection` `null`, where it read `unknown`.
- `queue list` carries `unconfirmed` on each parked row and lists one as `unconfirmed` in the plain output.
- `queue cancel` prints `{"row", "kind", "unconfirmed"}` and says, in its prompt and its plain output, that an unconfirmed row was sent and may have taken effect.

### Requirement: pimdir flags are staged per message (backends)

- The rows are appended as one batch, each passing the gate of STORAGE §15.6 once inside the enqueue; a refusal stages nothing and names the message refused.

### Requirement: A pimdir answer says what it covered (search)

- The coverage gains `missing`, `unparseable` and `encrypted`, the counts of the scope whatever the query, which leave the answer incomplete, and are said in the plain output, only while a term reads what they hide.
- `complete` is io-pimdir's `is_complete`.

### Requirement: pimdir searches through the store's index (search)

- The query language is that of search-draft-10 §8; the scenario An index of an earlier draft stamps `search-draft-09`.

### Requirement: A failure carries a stable code (commands)

- The table gains `store-reset` and `lock-replaced`.

## REMOVED Requirements

None.
