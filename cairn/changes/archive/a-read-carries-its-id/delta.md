---
cairn: delta
change: a-read-carries-its-id
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A read message is a designed view
`message read --json` SHALL print `{id, etag, headers, text, html, parts}`, `id` the id the command took and `etag` `null`, a message's body never changing and no write taking a version to match, beside a view built from the raw message so it is the same on every backend, rather than the parser's own model; `message parse --json` SHALL print the same view without `id` and `etag`. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset, `Z` for UTC, or `null`, `messageId` without angle brackets, `inReplyTo` and `references` as lists without angle brackets, as envelopes print them, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, a `format=flowed` part unwrapped (RFC 3676 §4.2), joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The message SHALL be parsed by mime-rs, whose structure is the one RFC 2046 delimits: a line opening with a boundary but carrying more than transport padding is no delimiter, an outer delimiter closes the parts nested in it, and a field whose name holds a blank (`Sub ject:`) is not the field it spells. The text output SHALL show the same headers, parts, sizes and names as the JSON view. Its From, To and Cc lines SHALL read the addresses as the JSON lists them, followed by each entry `headers.invalid` lists as `(unreadable: <entry>)`, a header holding neither being left out. `--raw` SHALL write the message bytes verbatim, and print `{data, size, mime}` under `--json`.

### Requirement: A message can be read without an account
Its first sentence reads:

`message parse [EML]` SHALL read a raw message from a path, or from stdin with `-` or when omitted, and SHALL print what `message read` prints for the same bytes, without the `id` and `etag` a read carries.

## REMOVED Requirements

None.
