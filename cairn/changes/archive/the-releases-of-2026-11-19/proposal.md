---
cairn: change
id: the-releases-of-2026-11-19
status: landed
created: 2026-11-19
---

# The releases of 2026-11-19

## Why

io-imap 0.12, io-smtp 0.10, io-sasl 0.3, io-managesieve 0.5, io-pimdir 0.18 and pimalaya-cli 0.7 were released on 2026-11-19, carrying what three open lines waited on:

- HIM-18 (the-schema-review-remainder, ruled before v3): io-imap 0.12 returns `imap raw`'s reply as the exact octets of each response, framed with its literals, where 0.11 decoded it lossily.
- C5 of pimdir's audit-2026-11-12: io-pimdir 0.18 exposes the store rules Himalaya restated (io-pimdir cairn/changes/the-store-rules-the-clis-restate), with the owner's ruling that a configured account is accepted on a store holding no collection yet.
- pimalaya-cli 0.7: `error::request_origin` (`request-origin`, the family's rule for a request off its API) and `sanitize::path_component` (the follow-up of a-saved-attachment-name-is-neutralised).

## What

- Every requirement raised; the `imap` and `smtp` features ask `io-imap/std` and `io-smtp/std` (the former `client` feature); one io-sasl in the tree.
- `imap raw` prints each response io-imap framed from its octets: UTF-8 as text, parsed as before; otherwise an `undecoded` event `{base64}` (pimalaya-cli output.md, *Structured when understood*). Its own response splitting and the unframed fallback go. A tagged response that is not UTF-8 exits 3, its status unread. The text form prints such a response lossily.
- The pimdir copies go: `is_mail` for `collection::media_type`, `resolve_account` on `PimdirReader::accounts_of` (messages and codes kept), `scope` on `PimdirQuery::scoped`, the coverage's read conditions on `PimdirSearchCoverage::unread_counted`, `overlay_quiet` for the reader's. Behaviour moves: a configured account on an empty store is read as named (it answered `store-account-unknown`); the unread chip is skipped by the reader's rule (a `copy` out of a mailbox leaves it quiet) and a queue that does not read fails the search, where it fell back to the whole-mailbox read.
- `gmail request` and `msgraph request` refuse a URL off their API through `request_origin`, coded `request-origin`, exit 1, checked before the session opens (`check`, as `smtp send` does), where the clap value parser refused it coded `usage-invalid`, exit 2; the family's rule (pimalaya-cli error.md, *A request off its API is coded request-origin*) asks exit 1 and that code, which the value parser would have pre-empted.
- `attachment download` names a part through `sanitize::path_component`, keeping its trimming, leading dots and `attachment` fallback around it.

Left: C6 (a code on an IMAP refusal through `code_bytes`) is ruled after v3.
