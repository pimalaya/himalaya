---
cairn: change
id: the-union-gains-its-verbs
status: landed
created: 2026-11-03
---

# The union gains its verbs

## Why

capability-shared-api (landed 2026-11-02) replaced the intersection rule with a union, but kept the registry to the verbs already shared: "the inventory's protocol-only rows wait for their shared verb". So the clap tree is still the v2 least-common-denominator tree, and the features the union was made for (vacation first among them) are still spelled per protocol. v3 already breaks the JSON shapes, so the verbs belong in it rather than in a 3.x trickle.

## What

Promote the inventory rows that keep one meaning across the backends declaring them, each a new shared leaf, a new capability in src/capability.rs, a `--help` footer, a registered output schema and an MCP tool.

**`mailbox create <NAME> [--parent <MAILBOX>]`** (`mailbox.create`): a mailbox named NAME, one level of the hierarchy, at the top or under the parent. IMAP joins the parent with the delimiter its `LIST` reports, Gmail with `/`, the local backends with `/`; JMAP and pimdir take the parent as such. Graph has no child-folder creation in io-msgraph, so a parent means creating at the top then moving under it: `partial`, two requests. Output `{id, name}`, `id` null when pimdir queues it.

**`mailbox rename <MAILBOX> <NAME>`** (`mailbox.rename`): the mailbox takes NAME as its last level and keeps its parent, as pimdir's `collection.rename` does. A path-based backend keeps the prefix up to the last delimiter. m2dir has no rename: `none`. Output `{id, name}`, the id it has afterwards (it changes on IMAP and the local backends), null when queued.

**`mailbox delete <MAILBOX>`** (`mailbox.delete`): the mailbox goes with the messages it holds. JMAP destroys with `onDestroyRemoveEmails`, a message also in another mailbox staying there: `partial`. Gmail deletes the label, its messages staying under their other labels or in All Mail: `partial`. Graph deletes the folders it holds too: `partial`. Output `{id}`.

**`flag … --flag important|junk`** (`flags.important`, `flags.junk`, RFC 8457 `$Important` and `$Junk`): JMAP, m2dir and pimdir full. IMAP `server`: a keyword is permanent only when the mailbox's `PERMANENTFLAGS` lists it or `\*` (RFC 3501 §7.1), checked after the `SELECT` and refused with `capability-missing` otherwise; a server sending no `PERMANENTFLAGS` allows every flag. Gmail: `IMPORTANT` full; `junk` is the `SPAM` label, which moves the message to Spam: `partial`. Graph: `important` is `importance: high`, the sender's priority rather than the reader's mark: `partial`; no junk mark: `none`. Maildir writes the keyword through the dovecot-keywords file and mbox through `X-Keywords`, seen only by readers of those: `partial`.

**`vacation get` and `vacation set`** (`vacation`): the auto-reply, `{enabled, from, to, subject, text, html}`, dates in RFC 3339 UTC. `set` changes only what it is given (`--enable`/`--disable`, `--from`, `--to`, `--subject`, `--text`, `--html`) and prints the auto-reply as it stands afterwards. JMAP `server` (needs `urn:ietf:params:jmap:vacationresponse`, RFC 8621 §8), Gmail full (dates as epoch milliseconds, the Gmail-only restrictions left to `gmail settings vacation`). Graph `none` until io-msgraph has `mailboxSettings`; IMAP `none`, naming `himalaya sieve` (RFC 5230 has no date window and owns the active script, the reason capability-shared-api gave). Vacation exchanges no message id, so it resolves on its own: to the first configured backend `--backend` allows that serves it, in the storage order, so an account syncing a Maildir from JMAP still reaches JMAP.

## Decided

- **Rename keeps the parent; reparenting is another verb.** The open question of capability-shared-api (`mailbox move` or a path-taking `rename`) is settled for `rename` by following pimdir's `collection.rename` (one level, parent kept), which also gives one meaning on JMAP and Graph, where the name holds no path. A shared `mailbox move` is left for later.
- **Left out, with reasons.** `thread`: IMAP `THREAD` threads one mailbox, JMAP, Gmail and pimdir the account, so one verb would give two results. `identity.list`: the config's aliases are another source of truth for the same answer; it needs its own design. `flags.keywords`: Gmail takes existing label ids only, Graph maps them to categories; a free-form keyword argument needs its own change. `envelope.sort` and `envelope.count` stay as they are.
- **Mailbox writes drop the cached mailbox index** of the backends minting opaque ids (JMAP, Gmail, Graph, pimdir), so a later verb in a REPL or MCP session resolves the new name.
- **Decided while implementing (2026-11-03).** No `--no-from`/`--no-to`: io-jmap's `JmapVacationResponseUpdate` skips a `None` field, so JMAP cannot be sent the `null` that clears a date; clearing waits for io-jmap. m2dir folders are flat, so its `mailbox.create` is `partial`, refusing `--parent`. Gmail renames through `labels.patch`, not `update`, so a label keeps its colour and visibility. `vacation set` is a sending tool in MCP (`--allow-send`), an auto-reply once on answering mail.
- **A pimdir write is queued**, so its output carries a null id and its prose says queued, as cairn/spec/commands.md "A queued write says it is queued" requires.

## Compatibility

Additive: new leaves and new flag values. One behaviour change: on IMAP, a keyword flag (`$Forwarded` after a forward, and now `important`, `junk`) a mailbox does not keep permanently fails with `capability-missing` instead of being stored for the session only; the forward already logs a refused `$Forwarded` as a warning.
