---
cairn: log
date: 2026-08-28
change: pimdir-collection-id-is-the-mailbox
---

# A pimdir mailbox is its collection id

The pimdir backend named a mailbox by stripping a `<namespace>/` prefix off its collection id, so `imap/INBOX` showed and resolved as `INBOX`, and both spellings addressed it. That mixed an addressing key with a display convenience over a convention the store cannot be asked about: `collections.id` is an opaque `TEXT PRIMARY KEY`, io-pimdir splits it nowhere, pimdir SPEC 9.2 says the store neither parses nor validates it, and hierarchy is modelled by the `parent` foreign key rather than by a separator. The `/` belongs to Neverest, whose own spec calls the namespace internal and non-configurable.

The guessing showed in the code: the derivation only worked because it filtered to `message/rfc822` first, it needed `pimdir.namespace` for the store it could not decide, and it needed an ambiguity error for two collections stripping to one name. Himalaya already had the answer in another backend: JMAP mailbox ids are opaque server strings carried verbatim, and `[mailbox.alias]` is how a user avoids typing one.

## What landed

**The id is the mailbox.** `-m imap/INBOX` is the spelling, and a listing shows `imap/INBOX` in both columns. `pimdir.namespace` is gone from `PimdirConfig` and from config.sample.toml, `resolve_namespace` and the client's `namespace` field with it, and `mailbox_name` is deleted.

**`hub_id` kept the half that was doing work.** It is now a membership check over the account's mail collections: a known id passes through, an unknown one is refused naming the ones the account holds. An id passed to the store unresolved reads as a mailbox that exists and is empty, which is what the check exists to prevent, and that was never the part the stripping provided.

**`Mailbox.name` carries the collection row's own name** rather than a derived one, which is JMAP's shape. Neverest seeds that column to the id today, so both columns read the same; a store that later carries a real display name shows it with no further change here.

**The listing sorts by id** rather than by the derived name, there being no longer two orders to choose between.

## Capabilities moved

- backends: *pimdir names a mailbox the way its server does* is replaced by *A pimdir mailbox is its collection id*

## Verification

Built clean and run against the live Neverest store at `~/.local/state/neverest/posteo`, which holds three namespaces (`imap`, `caldav`, `carddav`) under one account. `mailbox list` renders all sixteen mail collections as `imap/…` in both columns; `envelope list -m imap/INBOX` returns envelopes; `envelope list -m INBOX` is refused naming the sixteen collections the account holds. Both delta scenarios, checked against real data rather than a fixture.

No unit test was added for `hub_id`: it is a membership check whose two outcomes are exactly the two scenarios above, and covering it would have meant building a store fixture this crate has no harness for to re-assert what the live run already showed.

## Not done

Whether Neverest should write a short display name into `collections.name` is a question about the producer, left open. This change reads whatever is there, so either answer lands without touching Himalaya again.
