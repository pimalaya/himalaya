---
cairn: change
id: pimdir-adopts-the-io-pimdir-release
status: landed
created: 2026-10-30
---

# The pimdir backend adopts the io-pimdir release API

## Why

The pimdir stable-release plan (2026-10-30, Apps Phase A) gives io-pimdir an API the apps then move onto, so each stops hand-rolling what the library should own. Himalaya is affected on five points, all waiting for that io-pimdir revision; none can land before it.

- **Store errors lose their type** (report-himalaya.md should-fix 5): every `PimdirError` is stringified through `anyhow!("...{err}")` (src/pimdir/client.rs:65, :102, :123-131), so the JSON error has no `code`; a stale store relays io-pimdir's "delete the store and let it resync", which silently drops queued sends and other apps' local-only items; a newer store never says to upgrade himalaya. io-pimdir gains `PimdirError::code()` and a Display without that advice, `Version` saying upgrade.
- **Duplicate creates**: a staged `add` whose link id the mailbox already holds, or one already pending, is accepted and parks later. io-pimdir gains a duplicate-link refusal helper (`seq_for_link` and the pending creates) to refuse it at staging time.
- **Body staging**: a-staged-body-is-written-under-the-lock orders the producer open and the blob write by hand. io-pimdir gains `PimdirProducer::stage_body`, hashing and writing under the lock in one call, with reader blobs read-only.
- **Hand-rolled submit payload** (should-fix 9): `SubmitPayload` (src/pimdir/backend.rs) duplicates neverest's `SubmitMeta`, carries an `object` field Annex B.2 does not list, and omits `source` on undeclared stores; `const SUBMIT` duplicates `io_pimdir::intent::SUBMIT`. io-pimdir gains a typed `PimdirSubmit`.
- **Release plumbing**: the requirement `io-pimdir = "0.8"` (Cargo.toml) becomes the released version (0.9 or what io-pimdir ships as), and the `[patch.crates-io]` pins go once the user publishes the dependency chain. Moved to v2-release, as it changes no behaviour.
- **`account check` skips pimdir** (found reading the code on 2026-10-30): src/account/check.rs has no pimdir arm, so an account declaring only a pimdir block is refused as declaring no backend.

## What

- Map store errors to `ErrorCode` variants from `PimdirError::code()` (`store-stale`, `store-newer`, `store-busy`, ...), with no "delete the store" advice anywhere in himalaya's output.
- Refuse a duplicate create before staging it, through io-pimdir's helper.
- Stage bodies through `stage_body`.
- Build the `submit` intent from `PimdirSubmit`, dropping `SubmitPayload` and `SUBMIT`.
- Check a pimdir block in `account check`: list the mailboxes, open and drop a producer.

## Depends on

io-pimdir's release-preparation change (stage_body, `PimdirError::code()`, the duplicate-link refusal, `PimdirSubmit`).
