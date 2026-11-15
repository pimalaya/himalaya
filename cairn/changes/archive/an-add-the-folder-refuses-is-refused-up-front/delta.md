---
cairn: delta
change: an-add-the-folder-refuses-is-refused-up-front
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A pimdir body is written under the staging lock
A pimdir write carrying a body (`add_message`, `send_message`) SHALL hand the body to its producer with the action in one step (`enqueue_with_body_checked`): the capability gate and, for an add, the duplicate check run on the bytes in hand, then the body is written inside the producer's staging window (pimdir STORAGE §8) and the row pinning it appended, in one transaction. A write refused by the gate or the duplicate check SHALL leave no body in the blob tree. No body SHALL reach the blob tree while a collector holds the lock: the write waits for it. The gate reads a collection's own row before its source's (pimdir STORAGE §15.6): a source declaring `mail.message.add` `none` and `full` on the folder of its drafts role, as neverest declares a Graph source, SHALL see an add to any other mailbox refused with `unsupported`, naming the mailbox and the declared detail, nothing queued, rather than queued for every sync to reject.

#### Scenario: An add where the source adds nothing
- GIVEN a pimdir account whose `msgraph` source declares `mail.message.add` `none` with the detail `Drafts only`, and `full` on `msgraph/Drafts`
- WHEN `himalaya message add -m msgraph/Archive` stages a message
- THEN the command fails with the code `unsupported`, naming `msgraph/Archive` and `Drafts only`, and neither a row nor a body is left; the same add to `msgraph/Drafts` is queued

## REMOVED Requirements

None.
