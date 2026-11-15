---
cairn: change
id: the-outputs-become-data
status: landed
created: 2026-11-03
---

# The outputs become data

## Why

pimalaya-cli's every-output-is-data (pimalaya-cli cairn, landed 2026-11-03) removes `Message` and sets the rules every binary's payloads follow. The JSON review of 2026-11-03, made at 2f49f86, found himalaya printing 111 `{"message": ...}` confirmations, 56 of them built from data (new ids, counts, server replies, warnings, message bodies), 110 leaves with no schema, raw bytes written to stdout under `--json`, five shapes for flags, two for `inReplyTo` and addresses, a list-wrapped reply under a generic `response` key, four paging styles, mixed `null` and omitted keys, and an `account check` that dropped each backend's code on failure.

## What

**Shared commands.** Every write prints a typed receipt carrying `queued`, a boolean, beside what it acted on: `message copy` and `message move` print `{count, queued}`, `message send` and a composer's save or send print `{id, saved, sent, queued}` (the saved copy's id, `null` when not saved, queued or not returned), the flag writes print the `{raw, iana}` flags they wrote. A composer neither saving nor sending prints the template under `--json` and the message bytes otherwise. `message read --raw` prints the bytes, `{data, size, mime}` under `--json`. `inReplyTo` is a list in the read view, as on envelopes. An attachment row carries `role` (the read view's), not `inline`, and `path: null` until downloaded. `envelope list` and `search` print `nextPage` (the next page number while a page comes back full, `null` otherwise) and take it back through `--page-token`. `account check` prints every row, `{backend, ok, code, error}`, and exits 2 when a backend fails.

**Protocol namespaces.** Writes with no payload of their own print a receipt, `{id}` or `{ids}`, its sentence kept for the text form: Gmail, Graph, JMAP, maildir, mbox, m2dir and Sieve. Gmail settings writes print the settings the server returns, as the matching `get` does. A Gmail list prints the provider's collection key (`delegates`, `filter`, `forwardingAddresses`), `messages list` its `messages`. Graph's `messages list` takes back the `$skip` of its next link through `--page-token`. JMAP `email read` prints the emails as received, `email export` the bytes, `email import` `{id, blobId}`, and `vacation-response get` a disabled response when the server holds none. Every `imap` command prints its completion: `status: "OK"` beside what it understood (a select's state, an expunge's numbers, a store's FETCH data, an append's `APPENDUID`, a copy's `COPYUID`), or `{status: "NO"|"BAD", text}` exiting 2. `imap raw` prints untagged and continuation responses as events, then the tagged completions, split. `sieve raw` prints data lines as events and the completion as the result, `smtp raw` the reply's code and text, each exiting 2 on a refusal. `smtp send` names the envelope sent, `sieve check` and `put` the server's warnings. `imap fetch` addresses are `{name, email}`, `imap search` ids plain numbers like `sort`'s. maildir and mbox `flag list` rows are `{raw, iana}`.

**pimdir.** A queue row is `row` everywhere (`queueId`, `copyQueueId` and `id` before); `pimdir flag add|set|remove` prints `{messages: [{id, row}]}`, `queued` being a boolean in the contract; tag statuses and `matched` values are kebab-case; `queue show` prints every key, `null` when absent; `search --threads` is described under `pimdir-search`, and `performer` registered.

**Registry.** Every runnable command is described, checked by a test over the clap tree; a command printing one of two shapes registers both through a schema-only `OneOf`; `imap raw` and `sieve raw` register their events. Schemas use the serialize contract, so `Envelope`'s always-printed keys are required.

**MCP.** A tool result carries the events as text content before the result; a partial outcome keeps its result as `structuredContent` and sets `isError`.

## Decided (2026-11-03)

- Raw-faithful applies to JSON payloads; IMAP, SMTP, ManageSieve and the local stores print what himalaya understood, text otherwise, with no `raw` field beside it (no io-imap change now).
- Typed IMAP commands emit no events: io-imap returns their output whole. `imap raw` parses its reply after the fact, so its events come before the result but not progressively, and stay lossy on non-UTF-8 bytes (io-imap decodes the reply with `from_utf8_lossy`).
- A protocol refusal is a result exiting 2; the shared commands keep coded failures.

## Compatibility

Breaking for every `--json` caller, landing in himalaya 3.0.0 with the camelCase keys already on master. MIGRATION.md lists each shape that moved.
