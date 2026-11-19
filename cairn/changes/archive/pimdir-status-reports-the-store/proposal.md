---
cairn: change
id: pimdir-status-reports-the-store
status: landed
created: 2026-11-19
---

# pimdir status reports the store

## Why

Task X18 of the family's cli-alignment-2026-11-15 plan (from N30, N31): Himalaya listed the refusals standing in a store beside `pimdir queue list` (key `refused`) and had no `pimdir status`, where Calendula and Cardamum carry their refusals, parked rows and per-collection counts in `pimdir status`. The owner ruled on 2026-11-19: keep the `refused` key and its row shape, and add `pimdir status` to Himalaya at 3.0, mirroring the twins.

## What

`himalaya pimdir status` prints `{account, accounts, mailboxes, parked, refused}` as the twins print `{account, accounts, calendars|addressbooks, parked, refused}`: one `{id, name, total, hydrated, queued}` per mailbox, each parked row `{row, handle, collection, kind, error}`, each refusal in the shape `pimdir queue list` prints under its `refused`, which it keeps.

Two spellings differ from the twins, both to keep Himalaya's own: the mailbox a parked row or a refusal stands in is `collection`, as in Himalaya's queue rows and refusals, where the twins print `calendarId` and `addressbookId`; and the text prints through `Listing` (`Account:`, `Store account:`, `Grouped accounts:` above, no header row), as every Himalaya list does, where the twins print labelled `Columns`. The mailboxes sort by id, as `mailbox list` sorts them.

The X16 origin refusal of the same task (`request-origin`) belongs to pimalaya-cli's shared table and is not in this change: Himalaya's refusal (src/shared/raw_json.rs, `api_url`) is an uncoded `bail!`, to be raised under that code once pimalaya-cli releases it.
