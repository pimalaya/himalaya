---
cairn: log
change: a-test-store-is-held-while-it-is-filled
landed: 2026-11-19
---

# A test store is held while it is filled

Landed without a proposal: test-only, no behaviour moved. tests/pimdir_refusals.rs failed once per full parallel `cargo test` at line 36, the second `PimdirStore::open(&root).unwrap()` of the `account` fixture, a different test each run.

Evidence, on 2026-11-19: the test binary run 400 times under 8 parallel loops failed 26 times, every one `Owned("/tmp/.tmp…/store")`, 24 at line 36 and 2 at line 143 (the reopen adding `imap/Home` after the fixture returned).

Cause: the class calendula found the same day (calendula cairn/log/2026-11-19-a-test-store-is-held-while-it-lives.md). io-pimdir's owner lock is a `flock` on the open file description of `owner.lock`, released when the last handle drops, and a `himalaya` child another test spawns holds a copy of every descriptor until it executes, so a store just dropped can still read as owned when reopened in the same process. The first handle was not merely shadowed: `for_account` consumes it and the chain dropped it at the end of its statement, so line 36 was a reopen.

Fix, in the test alone: `account` takes the collections to declare (`WORK_AND_HOME`, plus `imap/Home` for the ambiguous case) and keeps every handle open until the last is declared, so each later open shares the lock io-pimdir's per-process registry holds instead of taking it anew. No handle is held while the `himalaya` children run, as they open the store themselves. After: 0 failures in 400 then 800 runs of the binary under 8 parallel loops, and full runs clean.

## Capabilities moved

None.
