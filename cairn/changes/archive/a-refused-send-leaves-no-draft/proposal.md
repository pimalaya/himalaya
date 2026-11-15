---
cairn: change
id: a-refused-send-leaves-no-draft
status: landed
created: 2026-11-14
---

# A refused send leaves no draft

## Why

A JMAP `message send` imports the message into the drafts mailbox with `$draft`, then creates an `EmailSubmission` whose `onSuccessUpdateEmail` files it as sent (RFC 8621 section 7.5). That patch runs on success only, so a submission the server refuses, `invalidRecipients` included, leaves the imported email in the drafts mailbox. On a server allowing duplicates every failed retry adds another. Recorded in cairn/changes/archive/a-result-carries-its-errors (Decisions) and in cairn/spec/backends.md ("A JMAP send files the message as sent"), found reading the code on 2026-11-14, not reproduced live.

## What

- A submission the server refuses explicitly, an entry in `notCreated`, of an email this send's import created (`Email/import` `created`) destroys that email with `Email/set { destroy: [id] }`, then fails with the submission's error and code as today (`recipient-refused` for `invalidRecipients`).
- An email the import found already there (`alreadyExists`, RFC 8621 section 4.8) is never destroyed: it was the account's before the send.
- A submission request failing without a `notCreated` answer (the connection, an HTTP status, a method error) destroys nothing, the message having possibly gone out: the send fails as today and a warning names the draft's id.
- A destroy that fails is a warning naming the id; the refused send's error stays the result.

## Scope / non-goals

- One extra round trip, on the refusal path only. The send is not redesigned (no `onSuccessDestroyEmail`, no submission before import).
- Not tested live: no writable JMAP test account. Offline tests drive `send_message` against a scripted JMAP server.

## Decisions

- **Design approved by the maintainer on 2026-11-14**, as above.
