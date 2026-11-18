---
cairn: change
id: a-protocol-refusal-is-a-result
status: landed
created: 2026-11-18
---

# A protocol refusal is a result

## Why

Findings COH-5, HIM-21 and the `request` half of D26 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18 (ruling D26): pimalaya-cli's output contract (cairn/spec/output.md, *A protocol refusal is a result*) makes a completion the server refuses inside a protocol namespace the command's result, exit 3, and a failure object only for a command that got no completion. Himalaya applied it to `imap` and the three `raw` commands alone. The `gmail`, `msgraph` and `jmap` commands (their `request` escape hatches included), the typed `sieve` commands and `smtp send` printed a server's refusal as the failure object, exit 1, and a JMAP set the server applied in part printed only the failure of what it refused, naming nothing it did: `jmap email delete E1 E2 Egone` destroyed two emails and said nothing of them.

## What

- An HTTP 4xx or 5xx a `gmail`, `msgraph` or `jmap` command meets, a JMAP request error, a JMAP method error and the SetError refusing the one write of a `jmap` command print as the result `{refusal: {status, reason, message, code, retryAfter}}` and exit 3: `status` the HTTP status (`null` inside a 2xx), `reason` the provider's name for it, `message` its text, `code` Himalaya's code of it, `retryAfter` the seconds asked. io-gmail and io-msgraph hand the parsed envelope, not the body, so the provider's fields are printed, not its bytes.
- A typed `sieve` command the server answers `NO` or `BYE` prints `{status, code, text}`, the completion `sieve raw` prints, and exits 3.
- `smtp send` refused (a `MAIL FROM`, a recipient, every recipient, `DATA`) prints `{refusals: [{rcptTo, code, text}]}` and exits 3, the message having reached nobody.
- The six JMAP sets acting on several objects (`jmap email delete|update|copy`, `jmap mailbox destroy`, `jmap identity delete`, `jmap submission cancel`) print `{ids, errors}`: the objects done, then an item error per object refused, exit 3 when one was.
- Each command's JSON Schema admits the refusal beside its own result.
- Breaking: exit 1 becomes 3 in five namespaces, the failure object becomes a result, and six receipts gain `errors`.
