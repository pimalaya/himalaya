---
cairn: log
change: the-schema-review-remainder
landed: 2026-11-26
---

# R1 to R5 of the schema review land before the tag

The owner ruled on 2026-11-26 that the five breaking findings of the schema review of 2026-11-26 land before the 3.0.0 tag, each as its recommendation says. The change stays active for its after-v3 items (B5, B6, C2, C6, R6 to R10).

## What landed

- R1, `jmap email read`: it sends its `Email/get` through the raw call (`raw::get_with`, `fetchTextBodyValues` or `fetchHTMLBodyValues`) and prints `{emails, errors}`, each email as the server sent it, every property kept and none added, an id not found an `errors` entry coded `message-not-found`, exit 3; the text form lays each email out from those objects, then the errors. src/jmap/email/read.rs, src/jmap/raw.rs. The JSON leaves the text form out (`texts` stays a text-only field), as the other protocol reads do.
- R2, `pimdir tag`: `flags` is `array|null`, `null` for a placement whose flags were never read (`unknown-flags`). src/pimdir/index.rs, src/pimdir/tag.rs.
- R3, `sieve deactivate`: it lists the scripts first and prints `{id}`, the script active before the `SETACTIVE ""`, `null` when none was. src/sieve/deactivate.rs.
- R4, `pimdir thread`: a member the store cannot read is an `errors` entry, `message-unreadable` unless the store names its code, the other members listed, exit 3, as `pimdir search --threads` lists one. src/pimdir/index.rs, src/pimdir/thread.rs.
- R5, `imap thread`: an entry's `flags` are strings, each as the server spelled it, as `imap fetch` and `imap store` print them. src/imap/envelope/thread.rs.

Tests: tests/json_schema.rs `jmap_email_read_prints_raw_emails_and_errors`, `pimdir_tag_flags_are_nullable`, `sieve_deactivate_prints_the_script_active_before`, `pimdir_thread_prints_errors` and `imap_thread_flags_are_wire_strings`, each failing against c3cdf686 (run from a `git archive` snapshot of it with the new test file); and the behaviour: `an_email_prints_as_sent_and_a_missing_id_is_an_item_error` and `a_get_with_body_values_keeps_the_emails_as_sent` (R1), `a_tag_on_flags_never_synced_stages_nothing_for_them` and `a_tagged_placement_prints_shared_flags` (R2), `the_script_active_before_is_printed` over a scripted ManageSieve server (R3), `a_thread_member_the_store_cannot_read_is_an_item_error` (R4), `replies_follow_what_they_answer` (R5, failing before on `[{raw, iana}]`).

CHANGELOG `[3.0.0]`: `jmap email read --json` and `sieve deactivate --json` breaking entries; the `pimdir tag`, `pimdir thread` and `imap thread` changes folded into their Added entries, those commands being new in 3.0.0. MIGRATION.md: the two protocol outputs a v2 user meets.

## Capabilities moved

- [commands](../spec/commands.md): ManageSieve commands (`deactivate` `{id}`), the `Flag` noun (`pimdir tag` `null`, the IMAP protocol commands printing strings), the item-errors requirement (`pimdir thread`, `jmap email read`), *Protocol listings print the provider's payload* (`jmap email read`).
- [search](../spec/search.md): `pimdir thread` prints `errors`; `pimdir tag` prints `flags` `null` for `unknown-flags`.
