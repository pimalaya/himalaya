---
cairn: change
id: the-tui-runs-on-himalaya
status: landed
created: 2026-11-14
---

# The TUI runs on himalaya

> Cross-repo change, same id in himalaya-tui. Order: **himalaya** (this side) → **himalaya-tui**, which pins the revision landing it.

## Why

The owner decided on 2026-11-14 that himalaya-tui becomes a frontend of himalaya: it runs himalaya's own commands in-process, as the REPL and the MCP server do, over a himalaya crate it pins by git revision under `[patch.crates-io]`. How himalaya is linked for releases is decided later. The TUI then drops its own backend layer, a copy of the retired io-email that drifts from this one (himalaya-tui cairn/changes/archive/the-tui-runs-on-himalaya).

Three things stand in the way here:

- Himalaya builds no library: packaging.md requires "a binary with no public library API and no lib target".
- `envelope list` prints `nextPage` and no total, while the TUI names its page as `(page n/p)` from the total the backend reports (himalaya-tui envelopes.md), and `nextPage` alone cannot tell a full last page from a full page with more after it (commands.md: "A full last page leads to an empty one").
- A session keeps its clients but has no way to keep a connection alive: an IMAP connection idle past the server's autologout dies, and the first command after it fails, a failed command not being retried (repl.md). The TUI pings its connection every 60 s of inactivity today.

## What

### A library target

- src/lib.rs declares the crate's modules, `pub mod` each, and carries the architecture header; src/main.rs keeps `main` alone, over the library. No re-export is added: a consumer names items by their module path (`himalaya::cli::Command`, `himalaya::session::Session`).
- What a consumer needs is already `pub`: `cli::Command` and `Command::execute(self, &mut impl Printer, &mut Session)`, `cli::resolve_account`, `session::{Session, SessionAccount}`, `backend::Backend`, `error::ErrorCode`, `config::{Config, AccountConfig}`, `account::context::Account`. The 87 `pub(crate)` items stay crate-private.
- The library carries no semver promise of its own: the contract stays the CLI's (its grammar and its versioned `--json` output), and a consumer pins a git revision. The README says so.
- The build script still sets `cfg(backend)` for the library, a dependency's build script running for it as well.

### `total` on `envelope list`

- `envelope list` prints `total` beside `envelopes`, `nextPage` and `errors`: how many messages the mailbox holds, when the backend answers it within the listing's own exchanges, `null` otherwise. No backend makes a request for it alone.
- IMAP reads it from the `EXISTS` of the `SELECT` the listing already sends (src/imap/backend.rs `list_envelopes`). JMAP asks `calculateTotal` on the `Email/query` it sends. Maildir, mbox and pimdir count what they list or index. Gmail answers `null`, its `resultSizeEstimate` being an estimate. Microsoft Graph answers `null`, its count being a request option of its own (`$count`).
- The plain text output does not change. `envelope search` prints `total` as `null`: a search counts its matches, not the mailbox.

### A session keepalive

- `Session::keep_alive(&mut self)`, library only, no command: sends `NOOP` on the unified client's IMAP connection when the session holds one, and does nothing for a stateless backend or an empty slot. A failure empties the slot, as a failed command does, so the next command builds the client again.
- The REPL and the MCP server do not call it; a caller idle on purpose does.

## Non-goals

- A stable Rust API, a published library, or splitting the crate: decided later, with how the TUI links himalaya for releases.
- A `noop` or `ping` command in the CLI grammar.
- A total for `envelope search`, which prints `null`.

## Open questions

- A mailbox picked from a listing is passed back to `-m` by its id, which `mailbox.alias` resolves first (src/account/context.rs `resolve_mailbox`): an alias key equal to another mailbox's id redirects it. Rare, and the same on the CLI. A verbatim form of `-m` would close it for the TUI; left for the owner.
