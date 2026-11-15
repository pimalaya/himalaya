---
cairn: delta
change: pimdir-queue-visibility
---

## ADDED Requirements

### Requirement: pimdir reads show what Himalaya staged
The pimdir backend SHALL read through an overlaying reader (pimdir SPEC §15.4), so an action it staged is visible on the next read rather than on the next sync. The overlay covers the kinds addressing an existing item: `set-flags`, `remove`, `move`, `copy` and `update`. Each keeps the message's public id, so a staged write changes what a listing shows and never how a message is addressed.

A parked action SHALL NOT show as staged: it will not be applied without an operator, and reading as pending would promise otherwise.

#### Scenario: A flag added offline stays added
- GIVEN a pimdir account whose store no sync is currently draining
- WHEN a flag is added and the mailbox listed again
- THEN the message carries the flag

#### Scenario: A message deleted offline leaves the listing
- GIVEN the same account
- WHEN a message is deleted and the mailbox listed again
- THEN it is gone from the listing, the action still queued for the owner

### Requirement: A queued create is reported, not listed
A queued create has no public id until the store's owner applies it, so the pimdir backend SHALL NOT project one as an envelope, and SHALL NOT put a placeholder in `Envelope.id`. `add_message` returns the link id it staged, which identifies the create across the window.

An envelope listing SHALL report how many creates the mailbox has queued and name the command that shows them, so a saved message that is not in the list reads as queued rather than as lost. A backend that stages nothing reports none.

#### Scenario: A saved message says where it went
- GIVEN a message saved to a pimdir mailbox and not yet synced
- WHEN that mailbox is listed
- THEN no envelope is added, and the listing reports one queued message and names `himalaya pimdir queue list`

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL render a queued create as a message (sender, subject, and its age from the row's `created_at`) where the kind-agnostic CLI can only print ids and hashes. `queue cancel` SHALL retract one row through io-pimdir's scoped owner operation, confirming first unless `--yes`.

Taking the owner role briefly is what cancelling costs (pimdir SPEC §15.5); the backend read and write paths SHALL NOT reach it. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

A command taking a public id, asked for a queued create, SHALL refuse naming the cancel command rather than reporting an unknown message.

#### Scenario: A queued draft is retracted
- GIVEN a queued create no sync has applied
- WHEN `himalaya pimdir queue cancel` is run for its row
- THEN the row is gone, its body left to the collector, and the mailbox reports no queued message

#### Scenario: Cancelling during a sync
- GIVEN a store Neverest is draining
- WHEN a cancel is attempted
- THEN it fails at once, saying a sync is running and the action may already have been applied
