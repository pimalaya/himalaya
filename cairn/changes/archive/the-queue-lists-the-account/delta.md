---
cairn: delta
change: the-queue-lists-the-account
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL list the account's queued creations (`add`) and sends (`submit`), and its parked rows of any kind, mailbox by mailbox in store order, or those of the one mailbox `-m/--mailbox` names (a name, alias, role or id), where the kind-agnostic `pimdir` binary can only print ids and hashes.

Under `--json` it SHALL print `{"messages": [<row>], "parked": [<row>], "refused": [<refusal>]}`, the pending rows then the parked ones, each in append order and in the family's one queue row (pimalaya-cli cairn/spec/output.md, *A queue row keeps one shape*), every key printed: `{"row", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject"}`. `row` is the queue row id `queue show` and `queue cancel` take; `kind` the row's action as the store holds it, read from io-pimdir (`PimdirAction::kind` for a pending row, `PimdirParkedAction::action` for a parked one), never re-spelled; `collection` the mailbox's id as `mailbox list` prints it; `linkId` the link id of the message an `add` creates, the bare `Message-ID` its receipt printed, `null` for any other row, a `submit` among them, the store creating no message for it; `itemId` the public id of the stored message a `set-flags`, `move`, `copy`, `remove` or `update` acts on, as `envelope list` prints it, `null` for any other row; `producer` the process that staged the row; `queuedAt` the instant the store stamped it; `attempts` how many times the sync engine tried it, an integer from 0 (`uint64`, minimum 0, in its JSON Schema); `unconfirmed` `true` for a row whose outcome is unknown (pimdir STORAGE §15.2); `error` `null` while the row is pending and the store's text once it is parked; `subject` the subject of the body the row pins, else of the message it acts on, a parked `submit` naming no body giving its payload's subject, `null` when there is none to read. The queued action carries no summary, so a subject SHALL be derived from the body the action pins, through the same derivation the owner applies. A parked payload that does not decode addresses nothing: `linkId`, `itemId` and `subject` are `null`. No placeholder SHALL stand in for the id a queued message lacks (no `""`, `0` nor row id).

`refused` SHALL list every refusal standing in what is listed, the account or the one mailbox, (io-pimdir's `PimdirReader::refusals`, pimdir STORAGE §15.4), per message and source, `{"collection", "itemId", "source", "reason", "refusedAt"}`: a change the store applied that a source's server refused for good, which the sync engine holds until the message is changed again or removed. The text form SHALL list each after the parked rows, `refused` in the state column, its instant, the message id, then the source and the reason; `Cancel:` SHALL print only when a pending or parked row is listed, a refusal being no row to cancel. The header SHALL name the mailbox only when `--mailbox` narrows the listing.

#### Scenario: A queued send
- GIVEN a pimdir account holding `imap/INBOX` and `imap/Sent`, and a send queued on `imap/Sent`
- WHEN `himalaya --json pimdir queue list` runs, then `himalaya --json pimdir queue list -m imap/INBOX`
- THEN the first lists the `submit` row of `imap/Sent`, and the second lists no row

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output: `{"row", "kind", "unconfirmed"}`. An unconfirmed row (pimdir STORAGE §15.2) is cancelled as a parked one, which undoes nothing its performer sent: the prompt and the plain output SHALL say it was sent and may have taken effect, and `unconfirmed` is `true`. A row anchored on a collection that is not one of the account's mailboxes (another account's, or an address book or calendar another app of the store queued on), or one neither pending nor parked, SHALL be refused before anything is opened, and stays as it was.

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

#### Scenario: A flag change a server refused
- GIVEN a pimdir flag change the sync engine applied, then the IMAP source's server refused for good
- WHEN `himalaya --json pimdir queue show <ROW>` and `pimdir queue list` run on its row and its mailbox
- THEN the row reads `applied` on its message with one entry of `refusals` naming `imap` and the server's reason, and the mailbox lists the message under `refused`

### Requirement: The pimdir queue view shows parked rows
`himalaya pimdir queue list` SHALL list the parked rows of what it lists, any action kind, beside the queued creations and sends, in the one queue row ("The pimdir subcommand reads and retracts the queue"), under `parked` in JSON: the row id, the action kind, the message it acts on, when it was queued, whether it is unconfirmed (`unconfirmed`, pimdir STORAGE §15.2) and the error that parked it. The plain output SHALL list an unconfirmed row as `unconfirmed` where it lists another as `parked`. `queue cancel` SHALL take a parked row as it takes a pending one.

#### Scenario: A duplicate draft parks
- GIVEN a staged `add` the sync engine parked because the source listed the same `Message-ID` before applying it
- WHEN `himalaya pimdir queue list -m imap/Drafts` runs
- THEN the row is listed as parked with its error, and `pimdir queue cancel` on its id removes it

### Requirement: The queue view shows queued sends
`himalaya pimdir queue list` SHALL list a queued `submit` beside the queued creates, in the account's listing as in its mailbox's, its `kind` `submit`, its subject derived from the body it pins as a create's is.

