---
cairn: log
change: live-night-fixes
landed: 2026-11-23
---

# Two fixes from the live test night

Found on 2026-11-23 by the live night's himalaya lane (.lanes/live/journal.md), against the local Stalwart and the lead's shared pimdir store.

## What landed

- `envelope list` over JMAP sent `Email/query` without a sort, so the server chose the order: Stalwart listed the oldest message first on page 1, the newest on the last page, while the help promises page 1 holds the most recent. The listing now asks for the newest `sentAt` first, as `envelope search` does without `--sort`. Pinned by `a_listing_asks_for_the_newest_first`.
- `pimdir performer collection.delete stalwart/imap` printed that the source now performs the intent although no source declared it; the sync engine parked the row. A source outside the candidates is now refused coded `no-performer`. The row anchored on the account's first collection, an attachments one, which `pimdir queue list`, `queue show` and `queue cancel` never read, so the parked row could not be seen or cancelled; it now anchors on the account's first mailbox. Pinned by `a_performer_choice_is_a_candidate_queued_on_a_mailbox`.

## Capabilities moved

- [backends](../spec/backends.md): the JMAP listing order; the `pimdir performer` source check and anchor, with a scenario.
