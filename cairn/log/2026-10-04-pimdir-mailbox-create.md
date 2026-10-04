---
cairn: log
change: pimdir-mailbox-create
landed: 2026-10-04
---

# pimdir creates a mailbox through the sync engine

`himalaya pimdir mailbox create <NAME> [--parent <MAILBOX>]` queues a `collection-create` intent, `{"v":1,"source","name","parent"?}`, anchored on the parent or the account's first mailbox, its performer named as for a send, and prints `{"queueId","name","parent"?,"source"}`. A store whose sources declare nothing, a name empty or holding a control character, and a capability no source or several unchosen ones perform are refused before anything is staged. The kind and capability are spelled in Himalaya until io-pimdir names them.

Capabilities moved: **backends** (pimdir creates a mailbox by queueing an intent).
