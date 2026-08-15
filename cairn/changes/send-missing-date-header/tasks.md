# Tasks

- [x] Locate the send choke point (`EmailClient::send_message`)
- [x] Prepend `Date:` when missing, never touch an existing one
- [x] Unit tests: injection on missing Date, byte-identical
  preservation of an existing Date, LF/CRLF line-ending match
- [x] `cargo test -p himalaya` passes
- [ ] Maintainer review and landing (fold delta into
  `cairn/spec/commands.md`, write the log entry, set `status: landed`)
