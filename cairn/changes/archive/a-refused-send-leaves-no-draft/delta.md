---
cairn: delta
change: a-refused-send-leaves-no-draft
---

# Delta

## MODIFIED Requirements

### Requirement: A JMAP send files the message as sent
A JMAP send SHALL stage the message in the drafts mailbox with `$draft`, then submit it with an `onSuccessUpdateEmail` patch (RFC 8621 section 7.5) moving it to the sent mailbox, unsetting `$draft` and setting `$seen`. The drafts mailbox SHALL be `jmap.drafts-mailbox-id`, else the `drafts`-role one, the send failing without either; the sent mailbox SHALL be `jmap.sent-mailbox-id`, else the `sent`-role one, the message only losing `$draft` without either. A patch the server refuses, or answers with an error, once the message is submitted SHALL be logged as a warning saying the message stays a draft, the send having succeeded. A submission the server refuses (an entry in `notCreated`) SHALL fail the send with its error and code, after destroying the staged email with `Email/set` when this send's import created it; an email the import found already there (`alreadyExists`) SHALL be left as it was. A destroy that fails SHALL be logged as a warning naming the email's id, the refusal staying the result. A submission request failing without such an answer (the connection, an HTTP status, a method error) SHALL fail the send and destroy nothing, the message possibly sent, a warning naming the id of an email this send created.

#### Scenario: A refused submission
- GIVEN a JMAP server importing the message as a new email `E1`, then refusing its submission with `invalidRecipients`
- WHEN `message send` runs
- THEN it sends `Email/set` destroying `E1`, and fails coded `recipient-refused`
