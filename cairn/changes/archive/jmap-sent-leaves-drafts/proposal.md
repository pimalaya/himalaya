---
cairn: change
id: jmap-sent-leaves-drafts
status: landed
created: 2026-10-01
---

# A message sent over JMAP leaves the drafts mailbox

## Why

JMAP send imports the message into the drafts mailbox with `$draft`, then submits it, and nothing moved it afterwards: every sent message stayed a draft and never reached the sent mailbox. io-jmap could not express RFC 8621 section 7.5 `onSuccessUpdateEmail`.

## What

- io-jmap gains `JmapEmailSubmissionSetArgs` with `onSuccessUpdateEmail` and `onSuccessDestroyEmail`, additive (0.4.1).
- The submission patches the email once sent: out of the drafts mailbox, into the sent one, `$draft` unset, `$seen` set.
- The sent mailbox is `jmap.sent-mailbox-id`, else the `sent`-role mailbox, read in the same `Mailbox/get` as the drafts one. Without either, the email only loses `$draft`.
