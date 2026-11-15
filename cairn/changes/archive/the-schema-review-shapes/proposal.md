---
cairn: change
id: the-schema-review-shapes
status: landed
created: 2026-11-15
---

# The schema review's shapes

## Why

Round X10 of the family's cli-alignment-2026-11-15 plan takes the JSON shape findings of the schema review of 2026-11-14 that stayed open in Himalaya (reports/schema-review-status-2026-11-15.md), before 3.0: a key that means two things, a string where the family prints an object, an empty string where the family prints `null`, and three shapes of one flag.

## What

- A7: `vacation get|set` print the window as `start` and `end` (`Start:`, `End:` in text), `from` and `to` naming addresses everywhere else. The `--from` and `--to` flags stay.
- A8: a composer's template prints `from` as an address `{name, email}` or `null`, `to`, `cc` and `bcc` as lists of them, as the envelopes and the read view do; the text template is unchanged.
- A9: `subject` is `null` for a message without one in every envelope, search hit, thread entry and `pimdir tag` row (the queued rows already were).
- A10: `pimdir attachment list|search` rows spell their keys as `attachment list` does: `filename`, `mime`, `message` for the message id, `from` an address or `null` in place of `sender` and `senderName`.
- B4: `message add` drops `saved`, always true: `{id, sent, queued}`.
- B8: one `Flag` noun `{raw, iana}`, `iana` nullable: `pimdir flag list` and the `pimdir tag` rows print it in place of bare strings, `maildir flag list` prints it as is (its `FlagRow` gone), `mbox flag list` beside `header`.
