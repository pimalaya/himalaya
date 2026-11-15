---
cairn: log
change: the-released-libraries
landed: 2026-11-15
---

# The released libraries

Owner order of 2026-11-15: every app builds on released libraries, no `[patch.crates-io]` left.

## What landed

- pimalaya-cli and pimalaya-config 0.5, io-imap 0.11, io-jmap 0.10, io-maildir 0.5, io-gmail 0.9 and mime-rs 0.2, all from crates.io; the patch section is gone. io-jmap's "display every set error" and mime-rs's first-occurrence rule for a structural field stated twice changed no test: 693 pass.
- io-pimdir 0.12 still depends on mime-rs 0.1, so the lock holds both until io-pimdir moves.

No capability moved.
