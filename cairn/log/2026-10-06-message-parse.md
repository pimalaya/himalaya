---
cairn: log
change: message-parse
date: 2026-10-06
---

# `message parse`: the read view of a message without an account

A client holding a raw message (a draft it reopens, a message it forwards, bytes fetched once with `--raw`) had to go through an account and a backend to see the `message read` view of it. `message parse` gives that view straight from the bytes, with no account and no configuration.

## What landed

- src/shared/message/parse.rs (new): `message parse [EML]`, reading the bytes as found from a path, or from stdin with `-` or when omitted (a bare call at a terminal is refused rather than left waiting), refusing a source of only whitespace by naming it; `--part <PART-ID>` writing that part's decoded bytes, or `{id, mime, filename, size, data}` under `--json`; `MessageParseOutput` (the view or the part) for the schema.
- src/shared/message/read.rs: `MessageReadOutput::from_raw`, the one path from raw bytes to the view (bounds included) that `message read` and `message parse` both print.
- src/shared/attachment/download.rs: `PartBytes::new` and `write_stdout` shared with `message parse --part`, so the bytes are those of `--stdout`.
- src/cli.rs: `message parse` dispatched before any account is resolved or configuration read; src/shared/message/cli.rs lists it beside the other `message` subcommands.
- src/json_schema.rs: `himalaya-message-parse`, the `message read` view or the `attachment download --stdout` part.
- tests/message_parse.rs (new): the built binary run with `HOME` and `XDG_CONFIG_HOME` at an empty directory: path, stdin and `-` give the same output; `--part` bytes and JSON; unknown and container ids refused; blank path and blank stdin refused by name; `message-too-complex` on the view and on `--part`; on a Maildir holding the same bytes, the JSON and text views equal `message read`'s and every part equals `attachment download --stdout`'s, raw and JSON.

Capabilities moved: **commands** (a message can be read without an account; a parsed message hands one part over).

## Verification

`cargo test --all-features`: 222 unit tests and 6 integration tests passed. `cargo clippy --all-features --all-targets` clean. `cargo fmt` applied. `cargo build --no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir` builds, with `message parse`. The 13 raw messages of MOA's parity vectors, on a Maildir: `message parse --json <file>` with an empty `HOME` equals `message read --json <id>` byte for byte for all 13, the text rendering too, and every part of every message equals `attachment download --stdout`, raw and JSON.
