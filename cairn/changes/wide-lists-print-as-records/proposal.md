---
cairn: change
id: wide-lists-print-as-records
status: landed
created: 2026-11-11
---

# Wide lists print as records

## Why

Since plain-text-output every list prints as `Columns`, which reads well while every cell but the last is short. An envelope list is not such a list: its FROM and SUBJECT are both of unbounded length, so a long sender pushes every subject to the right, a full `Name <email>` does not fit at all, and the FLAGS and ATT glyphs (`*R!`, `@`) have to be learnt. The same holds for the attachment lists (a filename and a download path), the pimdir search hits (sender, subject, snippet), the Microsoft Graph lists (ids of about 150 characters beside a name or a subject) and the Gmail filters (criteria and action). pimalaya-cli now ships `records::Records`, printing a list the way `git log` prints commits, and the owner approved that design on 2026-11-11.

## What

- The rule: a list prints as `Records` when its items carry more than one value of unbounded length (an address, a subject, a path, a long opaque id, criteria), and stays `Columns` when every cell but the last is short and bounded.
- An envelope prints as its id followed by its state words in parentheses (`unread`, `answered`, `flagged`, `attachment`, each only when it applies, an unknown attachment saying nothing), then `From:` (`To:` with `--recipient`) as full addresses, `Date:` as configured and `Size:`, then its subject indented by four spaces. The words replace the FLAGS and ATT glyphs, whose constants go.
- The same shape, id as title, state words for flags or booleans, properties as fields and free prose as body, for `jmap email query|get`, `msgraph messages list`, `pimdir search` (and `--threads`) and `pimdir thread`, `pimdir tag` (its status as the state word), the queued messages of `pimdir queue list`, `attachment list|download`, `pimdir attachment list|search`, `msgraph attachments list`, `msgraph mail-folders list|get|child-folders`, `gmail settings filters list` and `gmail history list` (the message ids each record touched, where it printed counts).
- A line a command prints under its records (the `Next page:` of the pimdir attachment pages, the `Incomplete:` coverage line, the queue note) follows a blank line.
- `format_addresses` prints full addresses, so `imap fetch` prints `Name <email>` again, as v2.2.1 did.
- Every other list stays `Columns`.

## Scope / non-goals

The `--json` output, the MCP tool results and the `json-schema` output do not change: only `#[serde(skip)]` fields and `Display` impls move. The `--has-attachment` help changes, since it named the ATT column, and with it the MCP description of that argument. `Paginated` (pimalaya-cli) still prints `Next page:` straight under the last item; a blank line there belongs to pimalaya-cli.
