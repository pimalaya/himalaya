---
cairn: change
id: pimdir-queued-send
status: landed
created: 2026-09-30
---

# A pimdir account sends through the queue

## Why

On a pimdir account every write is an action queued for the store's owner, except one: `message send` (and `reply`, `forward`, `compose` with send) falls through `EmailClient::send_message` to the account's SMTP transport, and fails when there is none. A pimdir account without an `smtp` section cannot send at all, and one with it sends online, from Himalaya's credentials, outside the store. The offline story stops at the one action users most expect to work offline.

Neverest already owns the other half. Since 0.2 it drains `submit` intents from the queue (neverest `cairn/spec/sync.md`, "A queued submission is a `submit` queue intent"): a `v: 1` payload `{object, from, rcpts, subject}`, the body pinned by the row, performed through the account's `smtp` table or Graph `sendMail`, at-least-once, 4xx pending, 5xx parked, each attempt reported with its row id in `sync --json`. Since 2026-09-30 its SMTP channel strips `Bcc:` before `DATA`. Nothing produces those intents; this change makes Himalaya the producer.

MOA is the first consumer that needs it: its approvals bind to the queued action, and "sent" is shown when Neverest's report names the row.

## What

- `send_message` on the pimdir backend writes the raw bytes to the blob store and enqueues one `submit` action (`PimdirAction::Unknown { kind: "submit", .. }`), through the same producer `add_message` uses. pimdir becomes a sending backend, alongside JMAP, Gmail and Graph.
- The payload is the envelope Himalaya's SMTP adapter already derives: `from` is the first `From:` address, `rcpts` every `To:`, `Cc:` and `Bcc:` address, `subject` the decoded `Subject:`. The derivation moves out of `src/smtp/backend.rs` into one function both call, so the two paths cannot disagree on who receives a message.
- The body is stored as given, `Bcc:` included. Graph's `sendMail` derives its recipients from the headers and needs it; Neverest's SMTP channel removes it before `DATA`.
- The command reports the queue row id, the only handle a queued send has (`himalaya pimdir queue cancel <id>`, Neverest's `submitted[].id`). The text says "queued for sending", not "sent": a confirmation of something that has not happened is the failure this whole change exists to avoid.
- `himalaya pimdir queue list` shows queued sends beside queued creates, recipients and subject from the payload, so what is waiting to leave is visible before the next sync.
- `--save <mailbox>` stays what it is on every backend: an independent `Add` to that mailbox, staged before the `submit`.

## Decisions for review

1. **An `smtp` section on a pimdir account.** Recommended: ignored, the way it is on JMAP, Gmail and Graph, since the backend sends by itself. This is a behaviour change for anyone reading a pimdir store and sending through Himalaya's SMTP today; the changelog says so. The alternative, queueing only when no `smtp` is configured, keeps them working but makes the send path depend on the presence of a section, and a MOA-style setup would have to know to leave it out.
2. **The anchor collection.** The row must anchor on a collection, and `enqueue` creates one that does not exist, so the anchor cannot be a made-up name. Recommended: the `--save` mailbox when given, else the mailbox `mailbox.alias.sent` names, else refuse, naming the alias to set. Picking an arbitrary collection would work for Neverest (it scans every queue) but put the send in a `queue list` nobody looks at.
3. **`--save` with a Graph channel.** Graph files its own copy in Sent, so `--save Sent` on an account Neverest sends through Graph ends with two copies on the server. Recommended: document it and leave `--save` alone; special-casing it needs Himalaya to know Neverest's channel, which it cannot.

## Not in scope

- Reporting the outcome. Neverest's report already carries each row's result; Himalaya does not poll or wait.
- A `Message-ID` check. Neverest's at-least-once relies on the receiving side deduplicating by `Message-ID`. Himalaya's composer always writes one; a raw message without it is sent as given, as on every other backend.
- Retrying or editing a parked submit. That is `pimdir queue cancel` and a new send today, and an operator concern after.
