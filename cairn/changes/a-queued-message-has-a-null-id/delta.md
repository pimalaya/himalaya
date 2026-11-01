---
cairn: delta
change: a-queued-message-has-a-null-id
---

## MODIFIED Requirements

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued creation as a message (flags, subject, recipient, and when it was queued, from the row's `created_at`) where the kind-agnostic `pimdir` binary can only print ids and hashes. The queued action carries no summary, so the row SHALL be derived from the body the action pins, through the same derivation the owner applies; an action pinning no body renders with its flags alone.

Under `--json` the message SHALL be printed as an `envelope` with the shared envelope's keys and `"id": null`, a queued message having no id until the owner applies its action. No placeholder SHALL stand in for it (no `""`, `0` nor row id): the row id is the row's own `id`, shown under ROW.

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output. A row anchored on a collection of another account, or one neither pending nor parked, SHALL be refused before anything is opened, and stays as it was.

Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema, which another owner of another revision keeps. A store that is not exactly this io-pimdir's schema SHALL be refused, saying the sync engine has to open it first. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is rendered from its body
- GIVEN a queued `add` pinning a body whose headers carry a subject and a recipient
- WHEN the queue is listed
- THEN the row shows that subject and recipient, and its envelope's `id` is `null` under `--json`

#### Scenario: A cancel on a store of another revision
- GIVEN a store whose schema another io-pimdir revision wrote
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails saying the sync engine has to open the store first, and neither the schema nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued

