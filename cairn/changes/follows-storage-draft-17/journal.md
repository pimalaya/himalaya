---
cairn: change
id: follows-storage-draft-17
---

# Journal

- 2026-11-23 16:47: step 0, tree clean at acc7e909 (on a2726f56); lane run under .lanes/RUN.md, the lead commits.
- 2026-11-23 16:49: `cargo update -p io-pimdir` (MSRV-aware resolver) also flipped five `windows-sys 0.61.2` dependency lines of errno, rustix, rustls-platform-verifier, tempfile and winapi-util to 0.52.0; reverted by hand so the lock diff is the io-pimdir source and its `libc` line, `cargo metadata --locked` green.
- 2026-11-23 16:51: `cargo fmt --check` was red at HEAD already (tests/envelope_search.rs:112, :128, tests/mcp.rs:317, from bf9f4e2a); formatted in its own commit `style: cargo fmt the search and mcp tests`, ahead of step 1.
- 2026-11-23 16:55: step 1 gate green: 867 passed, 0 failed, 25 ignored; 14 clippy sets `-D warnings`; no-SMTP build 340 passed; fmt clean.
