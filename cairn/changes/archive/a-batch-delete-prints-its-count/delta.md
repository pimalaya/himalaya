---
cairn: delta
change: a-batch-delete-prints-its-count
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A unified write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message send`, `message add` (`saved` always true) and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. `message delete` SHALL print the receipt of a batch delete, `{action, count, queued, errors}`: it takes several ids, so it counts the messages it acted on as `message copy` and `message move` do, where a delete of one item names it (`{id, queued}`), and `action` says what became of them, `moved-to-trash`, `deleted` from the trash, or `flagged` `\Deleted` with an expunge owing. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

## REMOVED Requirements

None.
