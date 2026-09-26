---
cairn: log
date: 2026-09-03
change: pimdir-merged-engine
---

# The pimdir backend over the merged io-pimdir

io-replica is retired and its sync engine lives inside io-pimdir, one crate for both parts of the pimdir standard. Himalaya's pimdir backend still built against io-pimdir 0.3 plus io-replica through two git patches, read a store shape the merged crate no longer writes, and carried two things that are now the library's: a private deserialisation of the `v: 1` meta JSON into an envelope, and a call into `io_pimdir::conventions`, which no longer exists.

## What landed

**Typed summaries in, `MetaView` out.** A read joins the item's `mail_summary` row and its addresses, and the envelope is built from those fields: every `From` and `To` address with its display name where the meta view kept one bare email per header, the `In-Reply-To` list, the date, the size and the attachment flag. Nothing parses JSON, and serde leaves the backend.

**The store's order is the listing's order.** `list_summaries` pages newest first, the mail sort key being the date, so the in-memory sort over a link-id scan is gone and a listing is one keyset walk.

**One derivation, called twice.** An added message derives its link id through `summary::mail::derive`, and names it on the queued `Add`. The action carries no summary any more: the owner derives it, and the sort key, from the same body through the same call when it applies the action. `pimdir queue list` reads the body the action pins and derives the row it shows the same way, so a queued message still renders with its subject and recipient.

**A mailbox declares its kind.** The allowance for a kind-less collection, left by a sync predating declared kinds, is deleted: the store cannot summarise such a collection, and a store from that draft is refused on open by io-pimdir, which says to delete it and resync. Himalaya adds no shim and no migration, the draft offering none.

**Manifest.** The `pimdir` feature drops `dep:io-replica`, io-pimdir moves to 0.4 through `[patch.crates-io] io-pimdir = { path = "../io-pimdir" }` until it is released, the io-replica patch is gone, and `mail-parser` stays: a pimdir-only build compiles the shared attachment, search and compose code under the `backend` cfg, and that code parses bodies with it. Confirmed by building `--no-default-features --features pimdir,rustls-ring` with the dependency removed, which fails in four shared modules.

**Producer API.** `enqueue` takes no timestamp any more, SQLite stamping the row, so the RFC 3339 clock stamp and its chrono call are deleted; chrono stays for parsing the summary's date.

## Capabilities moved

- backends: *Local storage backends*, *A pimdir mailbox is its collection id*, *pimdir is a reader and a producer, never the owner* and *The pimdir subcommand reads and retracts the queue* are restated for the merged crate

## Verification

`cargo build --features pimdir` clean; `cargo test --all-features` 119 passed, the seven pimdir tests among them, re-based on `PimdirMailSummary` and on a queued row derived from a body; `cargo clippy --all-features --all-targets` clean after a one-line fix in the wizard search test that a newer clippy lint (`cloned_ref_to_slice_refs`) had started flagging; `cargo fmt`. A pimdir-only build (`--no-default-features --features pimdir,rustls-ring`) compiles, with pre-existing unused-variable and unreachable-code warnings in account/check.rs, wizard/discover.rs and shared/client.rs that belong to the wizard's feature gating and are not touched here.

Two tests are deleted rather than ported: the bare `Message-ID` spelling and the `alt:` fallback of an added message now exercise io-pimdir's own derivation, which the library tests against the standard's vectors, and re-asserting it here would test the dependency.

Not run against a store: the merged crate's store format is unreleased and no local store carries it yet. The listing walk, the queued-row derivation and the stale-store refusal are exercised the first time Neverest writes a store at the new version.

## Not done

Pagination still scans the whole collection before cutting a page, as the file backends do. Now that the store pages in the order the listing shows, `list_envelopes` could stop after `page * page_size` items; left for a change of its own, the search path needing the whole set regardless.
