---
cairn: change
id: the-schema-review-remainder
status: active
created: 2026-11-19
---

# The schema review's remainder

## Why

The v3 pre-release pass of 2026-11-19 walked every finding of the schema review of 2026-11-14 (.github/cairn/changes/cli-alignment-2026-11-15/reports/schema-review-2026-11-14.md) that names Himalaya, and HIM-18's schema half from the family analysis of 2026-11-15, against 0cb2ff4d and a fresh `json-schema --dir` dump. This change holds what is left for the maintainer's review of the shapes (v3-release, "Review every `json-schema` entry's shape"); the rest is closed.

## Closed

| Finding | State | Where |
|---|---|---|
| A1 capability report | landed, one noun from pimalaya-cli; name grammars ruled D22 | b70d52ae |
| A4 queue row | landed, D19 and D21 | c7c43045 |
| A5 `pimdir search` paging | landed, D9 | c7016650 |
| A6 protocol payloads | landed, D18; the single-resource reads (`msgraph messages get`, `gmail profile get`, `gmail drafts get`, the Gmail settings reads, `jmap vacation-response get`) print the provider's object, checked on the dump (`additionalProperties: true`) | X10, X11 (3a077814) |
| A7 to A10, B4, B8 | landed | the-schema-review-shapes |
| B2 collection noun | ruled D20: mailboxes keep `{id, name, role, total, unread}` | none owed |
| B3 closed sets | `sieve raw` `status` and `pimdir performer` `capability` typed; `role` and the queue `kind` are open sets by spec (a verbatim unknown role, the store's kind never re-spelled) and stay strings | the-schemas-describe-every-key |
| B14 protocol gets | landed | 3a077814 |
| B15, B16, C1, C4 | landed | the-schemas-describe-every-key |
| C3, C6 `warnings`, C12 | landed | the-schema-leftovers |
| C16 paging | landed | 5b17916b, 8e8bdb37 |

## Left, each with a recommendation

- **B1, the `$id`.** The dump stamps `v2` (Cargo.toml 2.2.1). It moves to `v3` with the version bump, a v3-release task already. Nothing else.
- **HIM-18, the `imap raw` schema (breaking).** io-imap 0.11.2 still returns the raw reply through `from_utf8_lossy` (io-imap/src/rfc3501/raw.rs:254, unchanged at io-imap master dbc5472). Himalaya's local half landed (the-raw-reply-keeps-its-literals: no panic, no forged completion). The base64 shape for a non-UTF-8 literal needs io-imap to hand bytes first. Recommendation: decide before v3 whether to wait for that io-imap release; otherwise v3 ships the string shape, the base64 change waiting v4, and the MIGRATION guide says nothing of it now.
- **B5, `ids` on `message copy|move` (additive).** The receipts print `{count, queued, errors}`; the copies' new ids are returned by IMAP (`COPYUID`), JMAP, Graph and Maildir, not by Gmail (same id) nor pimdir (queued). Recommendation: add `ids`, one string or `null` per id acted on, in the order given, after v3 (additive, a minor).
- **B6, the `smtp send` reply (additive).** io-smtp 0.9's `SmtpMessageSend` returns the refused recipients alone, not the final `DATA` reply carrying the server's queue id. Recommendation: io-smtp returns it first, then `smtp send` prints `response: {code, text}`; after v3.
- **C2, `format: date-time` (additive).** Only the envelope `date` declares it. pimalaya-cli's output.md:52 sets the instant rule but no schema format. Recommendation: the family rule first in pimalaya-cli, then every RFC 3339 key declares it with one phrase; after v3.
- **C6, `code` on an IMAP refusal (additive).** The refusal arm prints `{status, text}` where `imap raw` prints `code`. io-imap's `ImapStatusError` carries the `Code`, but no public wire spelling of it exists (imap-codec's encoder is crate-private). Recommendation: io-imap exposes the code's text, then the arm prints `code`; after v3.

## Rulings of 2026-11-19

- **HIM-18: fix io-imap's raw bytes before v3.** io-imap changes now, unreleased (another agent's work in io-imap); Himalaya implements nothing against it before its release. Once io-imap releases the raw reply as bytes, Himalaya adopts that release (Cargo.toml requirement raised, no `[patch]`) and, before the version bump: `imap raw` prints a reply line or literal that is not UTF-8 as base64 in its own key, the UTF-8 one staying text, the structured-or-text rule of pimalaya-cli's output.md (*Structured when understood*) holding; the schema of `himalaya-imap-raw` declares that alternative; a regression test feeds a non-UTF-8 literal and fails against the `from_utf8_lossy` reading first; the spec (backends.md, the raw IMAP requirement), the CHANGELOG (breaking) and the MIGRATION guide (v2 to v3) say it. The exact key follows what io-imap hands back, decided when it lands.
- **B5, B6, C2, C6 stay after v3**, as recommended: additive, each a minor once its upstream half exists (B6 io-smtp's final `DATA` reply, C2 a pimalaya-cli rule, C6 io-imap spelling a response code; B5 Himalaya's alone).
- **B1** moves with the version bump, as before.
