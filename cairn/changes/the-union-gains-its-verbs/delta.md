---
cairn: delta
change: the-union-gains-its-verbs
---

Folded into cairn/spec/commands.md, cairn/spec/backends.md and cairn/spec/mcp.md on 2026-11-03.

## ADDED Requirements

### Requirement: Shared mailbox writes
`mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST` reports, `/` on Gmail and the local stores. The outputs SHALL be `{id, name}` for create and rename, `id` being the mailbox's id afterwards (a rename changes it where the name is the id) and `null` when the write is queued, and `{id}` for delete. A write SHALL drop the mailbox index a backend caches, so a later command of the session resolves the new name.

#### Scenario: A Maildir mailbox under a parent
- GIVEN a Maildir account holding `Archive`
- WHEN `mailbox create 2025 --parent Archive`, then `mailbox rename Archive/2025 2026`, then `mailbox delete Archive/2026` run
- THEN `Archive/2025` is created, renamed to `Archive/2026`, then removed

### Requirement: The important and junk flags are shared
The shared flag arguments SHALL include `important` (`$Important`, RFC 8457) and `junk` (`$Junk`), each its capability, `flags.important` and `flags.junk`.

### Requirement: An IMAP keyword is kept or refused
An IMAP flag write SHALL refuse, under `capability-missing` and before the `STORE`, a keyword the selected mailbox's `PERMANENTFLAGS` lists neither by name, compared ignoring ASCII case, nor by `\*` (RFC 3501 section 7.1). A server sending no `PERMANENTFLAGS` SHALL be taken to keep every flag. The system flags are left to the server.

### Requirement: The auto-reply is shared
`vacation get` SHALL print the account's auto-reply as `{enabled, from, to, subject, text, html}`, the dates RFC 3339 in UTC, and the default one, off with nothing set, when the backend holds none. `vacation set` SHALL change only what it is given (`--enable` or `--disable`, `--from`, `--to` as RFC 3339 or a `YYYY-MM-DD` read as midnight UTC, `--subject`, `--text`, `--html`) and print the auto-reply as it then stands. JMAP serves it when its session advertises `urn:ietf:params:jmap:vacationresponse` (RFC 8621 section 8), Gmail through `settings.getVacation` and `updateVacation`, its dates epoch milliseconds, the settings read first so the Gmail-only ones survive.

### Requirement: An account feature resolves on its own
A capability of the account rather than of a message, the auto-reply, SHALL resolve to the first backend of the storage order the account configures and `--backend` allows that serves it, else to the storage backend, whose refusal names what serves it instead.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `vacation get` runs
- THEN it reads JMAP's auto-reply, where `envelope list` reads the Maildir

## MODIFIED Requirements

### Requirement: Shared operation set
The shared adapters SHALL cover, per backend: `list_mailboxes`, `create_mailbox`, `rename_mailbox`, `delete_mailbox`, `list_envelopes`, `search_envelopes`, `store_flags`, `get_message`, `add_message`, `copy_messages`, `move_messages`, and `send_message`, and JMAP and Gmail `get_vacation` and `set_vacation`. A backend that cannot model an operation opts out of it rather than emulating it, declaring the capability `none` in its table (cairn/spec/commands.md), so the resolver refuses it before connecting.

### Requirement: Writing and sending are opt-in
Without flags the server SHALL list reading tools only, without the arguments that write. `--allow-write` SHALL add the tools and arguments that change the store, `mailbox_create`, `mailbox_rename` and `mailbox_delete` among them, and `--allow-send` those that send mail, `vacation_set` among them, an auto-reply once on answering mail.

### Requirement: A server-decided capability is refused once connected
A capability declared `server` SHALL be checked by the adapter once connected, before any write. A JMAP send whose session lacks `urn:ietf:params:jmap:submission` (RFC 8621 section 7) SHALL fail with `capability-missing` before the upload, leaving no draft; a JMAP `vacation` whose session lacks `urn:ietf:params:jmap:vacationresponse` SHALL fail the same way. An IMAP removal without UIDPLUS SHALL flag the messages `\Deleted` and say so. An IMAP keyword the mailbox would not keep SHALL be refused as "An IMAP keyword is kept or refused" says.

### Requirement: Verbs exchanging ids share one transport
The shared verbs that take or return message, envelope or mailbox ids (mailbox, envelope, message, flag, attachment) SHALL resolve to the same storage transport for a given account and `--backend`, so an id one of them returns is valid in the others. A verb exchanging no such id, sending or the auto-reply, SHALL resolve on its own.

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the confirmation of a shared write SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy`, `flag add|set|remove` and `mailbox create|rename|delete`. The JSON shapes SHALL NOT change, beyond a mailbox write's `id` being `null`, and no queue detail SHALL reach a shared output; the prose of a `message` confirmation carries the wording. Every other backend SHALL keep confirming the write as done.
