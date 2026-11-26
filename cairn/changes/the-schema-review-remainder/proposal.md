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

- **B1, the `$id`.** Done 2026-11-26: Cargo.toml reads 3.0.0 and every schema stamps `v3`, pinned by tests/json_schema.rs.
- **HIM-18, the `imap raw` schema (breaking).** Landed 2026-11-19 on io-imap 0.12 (the-releases-of-2026-11-19): a response that is not UTF-8 prints as an `undecoded` event `{base64}`. Before: io-imap 0.11.2 still returns the raw reply through `from_utf8_lossy` (io-imap/src/rfc3501/raw.rs:254, unchanged at io-imap master dbc5472). Himalaya's local half landed (the-raw-reply-keeps-its-literals: no panic, no forged completion). The base64 shape for a non-UTF-8 literal needs io-imap to hand bytes first. Recommendation: decide before v3 whether to wait for that io-imap release; otherwise v3 ships the string shape, the base64 change waiting v4, and the MIGRATION guide says nothing of it now.
- **B5, `ids` on `message copy|move` (additive).** The receipts print `{count, queued, errors}`; the copies' new ids are returned by IMAP (`COPYUID`), JMAP, Graph and Maildir, not by Gmail (same id) nor pimdir (queued). Recommendation: add `ids`, one string or `null` per id acted on, in the order given, after v3 (additive, a minor).
- **B6, the `smtp send` reply (additive).** io-smtp 0.9's `SmtpMessageSend` returns the refused recipients alone, not the final `DATA` reply carrying the server's queue id. Recommendation: io-smtp returns it first, then `smtp send` prints `response: {code, text}`; after v3.
- **C2, `format: date-time` (additive).** Only the envelope `date` declares it. pimalaya-cli's output.md:52 sets the instant rule but no schema format. Recommendation: the family rule first in pimalaya-cli, then every RFC 3339 key declares it with one phrase; after v3.
- **C6, `code` on an IMAP refusal (additive).** The refusal arm prints `{status, text}` where `imap raw` prints `code`. io-imap's `ImapStatusError` carries the `Code`, but no public wire spelling of it exists (imap-codec's encoder is crate-private). Recommendation: io-imap exposes the code's text, then the arm prints `code`; after v3.

## Rulings of 2026-11-19

- **HIM-18: fix io-imap's raw bytes before v3.** io-imap changes now, unreleased (another agent's work in io-imap); Himalaya implements nothing against it before its release. Once io-imap releases the raw reply as bytes, Himalaya adopts that release (Cargo.toml requirement raised, no `[patch]`) and, before the version bump: `imap raw` prints a reply line or literal that is not UTF-8 as base64 in its own key, the UTF-8 one staying text, the structured-or-text rule of pimalaya-cli's output.md (*Structured when understood*) holding; the schema of `himalaya-imap-raw` declares that alternative; a regression test feeds a non-UTF-8 literal and fails against the `from_utf8_lossy` reading first; the spec (backends.md, the raw IMAP requirement), the CHANGELOG (breaking) and the MIGRATION guide (v2 to v3) say it. The exact key follows what io-imap hands back, decided when it lands.
- **B5, B6, C2, C6 stay after v3**, as recommended: additive, each a minor once its upstream half exists (B6 io-smtp's final `DATA` reply, C2 a pimalaya-cli rule, C6 io-imap spelling a response code; B5 Himalaya's alone).
- **B1** moves with the version bump, as before.

## Review of 2026-11-26

A last pass over the 220 schemas the 3.0.0 release build writes (`json-schema --dir`, master 944ba0ad plus the v3 prep), against pimalaya-cli output.md and commands.md, the rulings above and the archived schema changes left alone. Clean: every `$id` is `v3`, every root an object, every key camelCase and required but one (R7), the shared nouns (`ItemError`, `Address`, `Flag`, `Envelope`) identical wherever they appear, the 59 error codes one kebab-case enum, `nextPage` `string|null` everywhere, ids strings and `row` an integer. Landed: the `IanaFlag` description no longer names Rust's derived `Ord`. Settled and not reopened: `pimdir status` naming a parked row's mailbox `mailboxId` (ruling of 2026-11-19).

### Breaking, for the owner before the tag

- **R1, `jmap email read`.** It re-serializes io-jmap's typed `JmapEmail` (absent properties added as `null`, unmodelled ones dropped), against *Protocol commands are raw-faithful*, and an id the server did not find is only logged (src/jmap/email/read.rs:46, exit 0), against *A failed item is never a failure object*. Recommendation: print `{emails: [raw], texts, errors}` as `jmap email get` does, `message-not-found` per missing id, exit 3.
- **R2, `pimdir tag` `flags`.** It prints `[]` when the placement's flags were never read (status `unknown-flags`), where `pimdir flag list` prints `null` for that state (*Absent is null*). Recommendation: `array|null`, `null` with `unknown-flags`.
- **R3, `sieve deactivate`.** It prints `{active}`, always `null`. Recommendation: print what was deactivated, `{id}` of the script active before or `null`.
- **R4, `pimdir thread` has no `errors`.** One unreadable member fails the command (src/pimdir/index.rs:308), where `pimdir search --threads` lists it under `errors`. Recommendation: add `errors`, exit 3.
- **R5, `imap thread` flags.** Its entries print `[{raw, iana}]` where `imap fetch` and `imap store` print wire strings. Recommendation: accept the richer shape, or align on strings now.

### Not breaking, after the tag

- **R6, descriptions missing** on `imap flag` (`flags`, `permanentFlags`), `imap search|sort` (`ids`, `uidMode`), `imap thread` (`threads`), the `event` key of the `imap raw` and `sieve raw` events, and the root alternatives of `attachment download`, `message parse` and `himalaya-error`'s `ReportCode`; no test checks keys and alternatives, src/json_schema.rs only refusing doc links.
- **R7, `@odata.count`** of `msgraph messages list` typed `integer|null` while it is left out, never `null` (src/msgraph/messages/list.rs:172): type it `integer`.
- **R8, the sieve refusal `status`** an open string in the typed `sieve` schemas where `sieve raw` lists `OK`, `NO`, `BYE`: an enum `NO`, `BYE`.
- **R9, wording.** About 40 nullable keys whose description does not say when they are `null`; `row-not-found` says "nothing was cancelled" though `queue show` raises it; the commands.md code table lacks `account-missing`, `account-default-missing`, `account-default-several` and `backend-not-built`; the `pimdir tag` text statuses (`flags-unknown`, `several-sources`, `not-synced`) differ from their JSON values.
- **R10, additive.** `attachment list|download` `mime` nullable where the part views are not; the Graph list wrappers drop the rest of the page where Gmail's and JMAP's keep it; mbox list rows carry no `id`; `pimdir status` `queued` an integer where receipts keep `queued` a boolean (a family rename, with Cardamum and Calendula).
