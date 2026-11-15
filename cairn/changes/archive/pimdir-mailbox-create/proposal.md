---
cairn: change
id: pimdir-mailbox-create
status: landed
created: 2026-10-04
---

# pimdir creates a mailbox through the sync engine

## Why

A pimdir account cannot create a mailbox: the store mirrors its server, and an enqueue on a new collection id would only add an empty collection to the store. The pimdir spec gains the `collection-create` intent (STORAGE Annex B.2), performed by the source on its server, the collection arriving with the next sync. MOA needs it to offer "Créer un dossier…" next to each mail role.

## What

- `pimdir mailbox create <NAME> [--parent <MAILBOX>]`, queueing the intent with `{v: 1, source, name, parent?}`, anchored on the parent or the account's first mailbox, the performer named as for a send.
- Refused on a store whose sources declare nothing, on a name that is empty or holds a control character, and when no source or several unchosen ones can perform `collection.create`.
- The shared API gains no `mailbox create`: what the command prints is the queue row, a pimdir detail.

io-pimdir 0.6 does not name the intent yet, so the kind and capability are spelled here until the pin moves to the release that does.
