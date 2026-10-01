---
cairn: log
change: jmap-sent-leaves-drafts
landed: 2026-10-01
---

# A message sent over JMAP leaves the drafts mailbox

A JMAP send used to leave every sent message in the drafts mailbox, still carrying `$draft`. The `EmailSubmission/set` call now carries an `onSuccessUpdateEmail` patch keyed `#outgoing`, moving the email from the drafts mailbox to the sent one, unsetting `$draft` and setting `$seen`.

`resolve_send_mailbox_ids` replaces `resolve_drafts_mailbox_id`, reading both roles in one `Mailbox/get` when either id is not configured. `jmap.sent-mailbox-id` is new; without it and without a `sent` role, the email only loses `$draft`.

io-jmap gained `JmapEmailSubmissionSetArgs` (`onSuccessUpdateEmail`, `onSuccessDestroyEmail`), additive: `new` and `email_submission_set` take it or the create map alone. Himalaya builds against it once io-jmap 0.4.1 is published.

Capability moved: **backends** (a JMAP send files the message as sent).

Verified against Stalwart 0.16: drafts left empty, the sent message in `Sent Items` with `$seen` and no `$draft`, and `sent-mailbox-id` overriding the role.
