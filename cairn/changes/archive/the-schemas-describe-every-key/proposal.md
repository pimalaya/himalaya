---
cairn: change
id: the-schemas-describe-every-key
status: landed
created: 2026-11-19
---

# The schemas describe every key

## Why

The non-breaking findings of the schema review of 2026-11-14 (.github/cairn/changes/cli-alignment-2026-11-15/reports/schema-review-2026-11-14.md) still open in Himalaya, taken in the v3 pre-release pass: B15 (a command printing one of two shapes described by a generic `OneOf` saying nothing of either), B16 (keys without a description), C1 (descriptions speaking Rust: `None`, intra-doc links), C4 (one address noun under two schema names, `Address` and `MailAddress`), and the closed-set part of B3.

## What

- B15: the generic `OneOf` gives way to one enum per kind of choice, each alternative saying when it prints: `ViewOrRaw` (`message read`, `gmail messages get`, `gmail drafts get`, `msgraph messages get`), `TemplateOrRoute` (the composers), `BytesOrSaved` (`gmail` and `msgraph` `attachments get`), `HitsOrThreads` (`pimdir search`), `CandidatesOrChoice` (`pimdir performer`).
- B16: every key described: the read view's address headers and their unreadable entries, `message delete` `action` and `count`, `maildir|mbox message add` `id` and `path`, `maildir|mbox flag list` `flags`, `jmap query` `methodResponses`, a read part's `role`.
- C1: the printed descriptions say `null`, without intra-doc links.
- C4: the read view prints its addresses through the envelopes' `Address`, `MailAddress` gone; the keys are the same.
- B3: `sieve raw` `status` and `pimdir performer` `capability` list their values as an `enum`. A mailbox `role` (a verbatim unknown role is kept) and a queue row's `kind` (the store's spelling, pimalaya-cli output.md: "it SHALL NOT be re-spelled") are open sets and stay strings.

No printed value changes.
