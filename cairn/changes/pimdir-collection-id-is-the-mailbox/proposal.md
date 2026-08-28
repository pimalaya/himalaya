---
cairn: change
id: pimdir-collection-id-is-the-mailbox
status: landed
created: 2026-08-28
---

# A pimdir mailbox is its collection id

## Why

The pimdir backend named a mailbox by stripping a `<namespace>/` prefix off its collection id, so `imap/INBOX` showed and resolved as `INBOX`. It reads well and it is the wrong shape: it mixes an addressing key with a display convenience, and it makes one mailbox answer to two spellings.

The id is not a composite. `collections.id` is an opaque `TEXT PRIMARY KEY` and io-pimdir splits it nowhere; pimdir SPEC 9.2 says the store neither parses nor validates it, and models hierarchy through the `parent` foreign key instead. The `/` is Neverest's convention, which its own spec calls internal and non-configurable. So the prefix is not something the store can be asked about, only something a consumer can guess at, and the guessing showed: the derivation needs a kind filter to work at all, needs `pimdir.namespace` for the case it cannot decide, and needs an ambiguity error for the case where two collections strip to one name.

Himalaya already has a backend whose ids are not names. JMAP mailbox ids are opaque server strings, `Mailbox.id` carries them verbatim, and `[mailbox.alias]` is how a user avoids typing one. That mechanism exists, is documented as existing for exactly this reason, and covers pimdir with no new concept.

## What

The collection id is the mailbox, verbatim, everywhere: `-m imap/INBOX` is the spelling, and `imap/INBOX` is what a listing shows in both columns. `pimdir.namespace` is removed, along with the derivation, the stripping and the ambiguity path.

`Mailbox.name` carries the collection row's own `name` rather than a derived one, which is the JMAP shape. Today Neverest seeds that column to the id, so both columns read `imap/INBOX`; a store that later carries a display name shows it without another change here.

A mailbox matching no collection is still refused naming the ones the account holds, which is the half of `hub_id` worth keeping: an unresolved id passed to the store reads as a mailbox that exists and is empty.

## Not in scope

Whether Neverest should write a short display name into `collections.name` is a separate question about the producer, and this change is deliberately compatible with either answer.
