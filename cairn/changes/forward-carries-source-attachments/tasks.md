---
cairn: tasks
change: forward-carries-source-attachments
---

# Tasks: forward carries the attachments of the source

- [x] Carry the source's attachment parts into the composed forward,
      each under its own file name, through the shared part walker.
- [x] Add `--no-attachments` to `message forward`, leaving the quoted
      body alone; a reply never carries.
- [x] Cover carrying, opting out and the no-attachment source in
      `tests/message_forward.rs`.
- [x] Record the change in the CHANGELOG under `[Unreleased]`.
- [x] Replace nameless-part skipping with subject-derived `.eml` names
      and generic MIME-based fallback names, per PR #788 review.
- [x] Observe failing regressions before implementing the naming fix;
      verify payloads, quoted bodies and `--no-attachments` afterward.
- [x] Run focused and full Cargo tests, nextest, formatting and Clippy;
      record observed results and any verification limitations.
- [x] Remove `PR_BODY.md` and amend the single local commit for review.

## Observed verification

Commands use `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0` for the server's memory limit.

- Before the implementation, `cargo test --test message_forward`:
  4 passed, 2 failed with `nameless attachment was dropped`.
- During implementation, the same command: 5 passed, 1 failed because
  `mime_guess` chose `.aaf` for octet-stream; fixed the binary fallback
  to `.bin` without changing the expected name in the regression.
- After implementation, the same command: 6 passed, 0 failed.
- `cargo test`: 296 passed, 0 failed, 1 ignored (the existing
  `fifty_thousand_mails_list_one_page` stress test).
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo clippy --locked --all-targets --all-features`: passed without
  warnings; finished in 5m 57s, including native TLS dependencies.
- `cargo nextest run --locked --test message_forward --test-threads 2`:
  6 tests run, 6 passed, 0 skipped.
- The repository's `.github/workflows/tests.yml` delegates to Nix;
  Nix is unavailable locally, so that CI entry point was not run.

The proposal remains active for human review. Folding the delta into
`cairn/spec/` and writing `cairn/log/` belong to upstream landing, outside
the explicitly allowed files for this follow-up. The existing
`CHANGELOG.md` entry under `[Unreleased]` already covers this PR.
