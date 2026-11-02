---
cairn: log
change: pimdir-mailbox-rename-delete
landed: 2026-11-02
---

# pimdir renames and deletes a mailbox through the sync engine

`himalaya pimdir mailbox rename <MAILBOX> <NAME>` queues a `collection-rename` intent, `{"v":1,"source","name"}`, and `himalaya pimdir mailbox delete <MAILBOX>` a `collection-delete` intent, `{"v":1,"source"}` (pimdir STORAGE Annex B.2, pimdir b63b39a), each anchored on the mailbox it addresses, its performer named as for a creation through io-pimdir's `enqueue_collection_rename` and `enqueue_collection_delete`. They print `{"queueId","mailbox","name","source"}` and `{"queueId","mailbox","source"}`. A store whose sources declare nothing, a capability no source or several unchosen ones perform, a mailbox the account does not hold and a rename name empty or holding a control character are refused before anything is staged. The store records neither hierarchy separators nor mailbox parents, so a name holding the server's separator, a sibling already so named and a mailbox holding others are left to the performer, which parks the row.

The backend's `PimdirCreated` is now `PimdirQueuedIntent`, returned by `create_mailbox`, `rename_mailbox` and `delete_mailbox`. `pimdir queue show` reads the rows as it reads any kind; its help now says that a renamed mailbox's receipt follows the collection to its new id, and that a deleted mailbox's receipt leaves the store with the collection (the receipts cascade), the row then reading `unknown`, the mailbox missing from `pimdir mailbox list` being what says it was deleted. `pimdir queue list` lists a parked rename or delete as it lists any parked row.

io-pimdir moves from 925b8a1 to 908db2b (Cargo.lock: io-pimdir now takes mime-rs, at the rev Himalaya already pins, in place of mail-parser). Its gate now decides `$Important` and `$Junk` by `mail.flags.important` and `mail.flags.junk` where a source states them, `mail.flags.keywords` otherwise (Annex B.1); Himalaya's flag staging names no capability of its own, so only a test was added. The shared API gains no mailbox verb; capability-shared-api's task for the protocol-only verbs stays open.

Verified: 411 tests pass with all features (6 new). The new tests do not compile against the 925b8a1 pin, which lacks `enqueue_collection_rename`, `enqueue_collection_delete` and `MAIL_FLAGS_JUNK`. Clippy is clean.

Capabilities moved: **backends** (pimdir renames or deletes a mailbox by queueing an intent added; pimdir writes name their queue row and pimdir stages flags by their raw spelling modified).
