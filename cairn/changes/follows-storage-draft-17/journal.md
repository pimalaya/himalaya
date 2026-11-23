---
cairn: change
id: follows-storage-draft-17
---

# Journal

- 2026-11-23 16:47: step 0, tree clean at acc7e909 (on a2726f56); lane run under .lanes/RUN.md, the lead commits.
- 2026-11-23 16:49: `cargo update -p io-pimdir` (MSRV-aware resolver) also flipped five `windows-sys 0.61.2` dependency lines of errno, rustix, rustls-platform-verifier, tempfile and winapi-util to 0.52.0; reverted by hand so the lock diff is the io-pimdir source and its `libc` line, `cargo metadata --locked` green.
- 2026-11-23 16:51: `cargo fmt --check` was red at HEAD already (tests/envelope_search.rs:112, :128, tests/mcp.rs:317, from bf9f4e2a); formatted in its own commit `style: cargo fmt the search and mcp tests`, ahead of step 1.
- 2026-11-23 16:55: step 1 gate green: 867 passed, 0 failed, 25 ignored; 14 clippy sets `-D warnings`; no-SMTP build 340 passed; fmt clean.
- 2026-11-23 16:57: D5, `cargo update -p url`: url 2.5.8 to 2.5.9, form_urlencoded 1.2.2 to 1.2.3, nothing else; `cargo deny check` advisories, bans, licenses, sources ok.
- 2026-11-23 17:00: step 2 tests fail on dc19b8d (scratch copy .lanes/himalaya/old, target copied from the plan's target-base): `a_reserved_form_is_refused_with_its_code` (`~tilde` listed), `a_reserved_form_is_refused_as_query_refused` (`/x` parsed), `a_flag_term_reads_the_store_while_the_index_lags` (count stays 1); `a_label_is_read_from_the_store` and `a_set_property_row_names_its_message` do not compile there (no `add_property`, no `SetProperty`). `a_set_property_row_names_its_message` queues on imap/Sent, the fixture's one mailbox, not imap/INBOX as tasks.md wrote. `a_malformed_query_is_refused_with_its_code` now asserts exit 1, not only a failure.
- 2026-11-23 17:08: re-pinned to io-pimdir drop 1, cc3021bb (lead: CI fixes, no API change): Cargo.toml rev and the Cargo.lock source line alone, `cargo metadata --locked` green. tests/docs.rs caught a three-line test summary (fixed); the gate now runs the tests `--no-fail-fast` so the counts cover every binary.
