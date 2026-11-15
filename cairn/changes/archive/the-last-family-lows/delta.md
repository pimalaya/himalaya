---
cairn: delta
change: the-last-family-lows
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Unified mailbox writes
`mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST "" ""` reports, `/` on Gmail, Graph and the local stores. NAME SHALL be refused when empty, blank or holding a control character, and per backend when it holds the hierarchy delimiter (IMAP's, `/` on Gmail, Maildir and the unified mbox writes), or names no listable level (Maildir `cur`, `new`, `tmp`, a leading dot on Maildir, a dot on Maildir++, the names the mbox backend refuses). Creating an existing mailbox and renaming onto one SHALL be refused. A create naming a mailbox that exists SHALL fail coded `mailbox-duplicate`, exiting 1 with nothing created nor staged, wherever the backend can tell: a Maildir folder already at the path (a file there being no mailbox, refused uncoded), an mbox file already there (a directory refused uncoded), a mailbox a pimdir store lists or has its creation queued (backends, "pimdir creates a mailbox by queueing an intent"), an IMAP `CREATE` the server refuses with `ALREADYEXISTS` (RFC 5530 section 3), or refuses with no code while `LIST` then shows the mailbox, a JMAP `Mailbox/set` refused while a mailbox of that name sits under the same parent (siblings sharing no name, RFC 8621 section 2) and coded no other way, a Gmail label create answered 409, and a Graph folder Graph answers exists (409 `ErrorFolderExists`) at the top, or one of that name the parent holds, hidden ones included, looked up before the folder is created at the top. A Graph namesake at the top of a folder asked under a parent is no duplicate, its refusal kept uncoded. Every such refusal SHALL read ``Cannot create mailbox `NAME`: it exists already``, NAME the mailbox as the create resolved it (its path under the parent on IMAP and the local stores), what the backend or the store answered (the server's refusal, the pimdir id listing it, its queued creation, the Graph parent holding it) kept in `sources`, as Cardamum and Calendula word a duplicate address book or calendar. The protocol namespaces' own creates (`imap mailbox create`, `jmap mailbox create`, `gmail labels create`, `msgraph mail-folders create`) keep the server's refusal as it is; `maildir create` and `mbox create` code the duplicate as the unified create does. The root of a local store (the root Maildir, however named, and the mbox spool) SHALL never be deleted or renamed. Deleting a local mailbox that holds others SHALL be refused, naming them, and a Maildir holding any directory besides `cur`, `new` and `tmp`, listed or not, SHALL be refused too, naming it, the removal taking it along otherwise; on IMAP a server's refusal of the same SHALL name them too, and a missing mailbox SHALL read ``IMAP mailbox `X` not found`` for rename and delete alike. On IMAP a `--parent` the server marks `\Noinferiors` (RFC 3501 section 7.2.2) SHALL be refused before the `CREATE`, ``IMAP mailbox `X` can hold no other (\Noinferiors)``. On Gmail, where a label nests by name, a rename SHALL also rename every sublabel, as IMAP `RENAME` moves the inferiors; a delete SHALL leave the sublabels under their full names.

The outputs SHALL be `{id, name}` for create and rename, `id` being the mailbox's id afterwards (a rename changes it where the name is the id) and `null` when the write is queued, and `{id}` for delete, the id `mailbox list` prints. `name` SHALL be the full name `mailbox list` prints on IMAP, Maildir and mbox, and NAME as given on the other backends. A write SHALL drop the mailbox index a backend caches, so a later command of the session resolves the new name.

#### Scenario: A Maildir mailbox under a parent
- GIVEN a Maildir account holding `Archive`
- WHEN `mailbox create 2025 --parent Archive`, then `mailbox rename Archive/2025 2026`, then `mailbox delete Archive/2026` run
- THEN `Archive/2025` is created, renamed to `Archive/2026`, then removed

#### Scenario: The root Maildir
- GIVEN a Maildir account whose root is a Maildir holding `Archive`
- WHEN `mailbox delete .`, `mailbox delete INBOX` or `mailbox rename INBOX X` runs
- THEN it is refused, naming the store root, and the store is whole

#### Scenario: A mailbox created twice
- GIVEN a Maildir account holding the folder `Archive` and an mbox account holding the file `Archive`
- WHEN `himalaya --json mailbox create Archive`, `maildir create Archive` or `mbox create Archive` runs on either
- THEN it fails coded `mailbox-duplicate`, exiting 1, and the folder and the file are as they were


### Requirement: A part can be read without touching the disk
`attachment download <MESSAGE-ID> <PART-ID> --stdout` SHALL take exactly one part id, any leaf part `message read` lists, and write that part's decoded bytes to stdout with nothing else, touching no file; `--dir` SHALL be refused beside it. Under `--json` it SHALL print `{id, mime, filename, size, data}`, `data` in base64. Without `--stdout`, the parts named SHALL be written to the download directory as the same decoded bytes; given no id, the parts `attachment list` lists, the inline ones only with `--inline` (`-i` a hidden alias of it, `-i` meaning `--interactive` across the family).


### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL list the mailbox's queued creations (`add`) and sends (`submit`), and its parked rows of any kind, where the kind-agnostic `pimdir` binary can only print ids and hashes.

Under `--json` it SHALL print `{"messages": [<row>], "parked": [<row>]}`, the pending rows then the parked ones, each in append order and in the family's one queue row (pimalaya-cli cairn/spec/output.md, *A queue row keeps one shape*), every key printed: `{"row", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject"}`. `row` is the queue row id `queue show` and `queue cancel` take; `kind` the row's action as the store holds it, read from io-pimdir (`PimdirAction::kind` for a pending row, `PimdirParkedAction::action` for a parked one), never re-spelled; `collection` the mailbox's id as `mailbox list` prints it; `linkId` the link id of the message an `add` creates, the bare `Message-ID` its receipt printed, `null` for any other row, a `submit` among them, the store creating no message for it; `itemId` the public id of the stored message a `set-flags`, `move`, `copy`, `remove` or `update` acts on, as `envelope list` prints it, `null` for any other row; `producer` the process that staged the row; `queuedAt` the instant the store stamped it; `attempts` how many times the sync engine tried it, an integer from 0 (`uint64`, minimum 0, in its JSON Schema); `unconfirmed` `true` for a row whose outcome is unknown (pimdir STORAGE §15.2); `error` `null` while the row is pending and the store's text once it is parked; `subject` the subject of the body the row pins, else of the message it acts on, a parked `submit` naming no body giving its payload's subject, `null` when there is none to read. The queued action carries no summary, so a subject SHALL be derived from the body the action pins, through the same derivation the owner applies. A parked payload that does not decode addresses nothing: `linkId`, `itemId` and `subject` are `null`. No placeholder SHALL stand in for the id a queued message lacks (no `""`, `0` nor row id).

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output: `{"row", "kind", "unconfirmed"}`. An unconfirmed row (pimdir STORAGE §15.2) is cancelled as a parked one, which undoes nothing its performer sent: the prompt and the plain output SHALL say it was sent and may have taken effect, and `unconfirmed` is `true`. A row anchored on a collection of another account, or one neither pending nor parked, SHALL be refused before anything is opened, and stays as it was.

Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema. A store of another draft SHALL be refused as `store-version` and a damaged one as `store-corrupt`, each left as it is. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is listed from its body
- GIVEN a queued `add` of the message `d1@x.org` pinning a body whose `Subject:` is `hi`
- WHEN the queue is listed with `--json`
- THEN its row prints `"kind":"add"`, `"linkId":"d1@x.org"`, `"itemId":null`, `"error":null` and `"subject":"hi"`, and no `send` nor `envelope`

#### Scenario: A parked move names its message
- GIVEN a `move` of message 7, whose subject is `hi`, the sync engine parked
- WHEN the queue of its mailbox is listed with `--json`
- THEN the row is under `parked` with `"kind":"move"`, `"itemId":"7"`, `"linkId":null`, `"subject":"hi"` and the store's error text

#### Scenario: A cancel on a store of another draft
- GIVEN a store whose `store_meta.format_version` names another draft
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails with the code `store-version`, pointing at neverest's MIGRATION.md, and neither the store's stamp nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued


## REMOVED Requirements

None.
