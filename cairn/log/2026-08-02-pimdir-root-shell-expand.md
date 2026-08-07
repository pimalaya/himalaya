---
cairn: log
change: pimdir-root-shell-expand
landed: 2026-08-02
---

# Shell-expand `pimdir.root`

A pimdir account pointed at a Neverest store with the natural
`pimdir.root = "~/.local/state/neverest/posteo"` had an empty `mailbox list` (and
every read empty). `pimdir.root` is a `PathBuf` deserialized verbatim with no
`~` expansion, and `PimdirStore::open` creates on absence — so himalaya opened the
literal relative path `./~/.local/…`, made an empty store there, and listed
nothing, with no error (and littered a `~` dir in the cwd).

Fix: `PimdirClient::new` expands `~`/env vars on `root` with `shellexpand::full`
(already a dep, used by the wizard) before opening the store and blobs, falling
back to the raw path on failure. Also: `config.sample.toml` now documents
`pimdir.root` (one line to read a Neverest account) and clarifies that
`pimdir.source` is auto-detected and normally omitted; the `PimdirConfig.source`
doc was corrected likewise.

Verified read-only against the real Neverest-synced Posteo store: `mailbox list`
returns all 16 synced mailboxes; `envelope list -m Notes` renders subjects/senders
from the `v:1` meta; `message read <id>` pulls the body from the content-addressed
blob and renders its MIME parts. Build + fmt clean.

Spec updated: `backends` (ADDED "pimdir store path is shell-expanded").
