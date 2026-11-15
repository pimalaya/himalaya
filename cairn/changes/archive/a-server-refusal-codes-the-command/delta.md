---
cairn: delta
change: a-server-refusal-codes-the-command
---

# Delta

## ADDED Requirements

### Requirement: A server's refusal codes the whole command
A command failing whole because a server refused it SHALL carry the code of that refusal, read from the typed error of the protocol that answered it anywhere in the error chain: an IMAP completion naming `NOPERM` `permission-denied` and one naming `OVERQUOTA` `quota-exceeded` (RFC 5530 §3); a Gmail or Microsoft Graph answer as the same answer codes an item; a JMAP HTTP 429 `rate-limited` and 403 `permission-denied`, whichever request it answered, a method error `forbidden` `permission-denied`, and the SetError refusing the one write of a command by its type (`forbidden`, `overQuota`, `rateLimit`). The code SHALL be given at every exit: a one-shot run, a REPL line, an MCP call, and the row of an `account check`. A command reading one message (`message read`, `attachment list`, `attachment download`, the source of `message reply` and `message forward`) whose id the backend does not hold SHALL fail coded `message-not-found` on every backend, a server's answer kept in `sources`.

#### Scenario: A mailbox the server forbids
- GIVEN an IMAP server answering `CREATE` with `NO [NOPERM]`
- WHEN `himalaya --json mailbox create Denied` runs, or the line `mailbox create Denied` in a `repl`, or the MCP call `mailbox_create`
- THEN each answers the failure object coded `permission-denied`

#### Scenario: A message the mailbox does not hold
- GIVEN a Maildir account
- WHEN `himalaya --json message read 999` runs
- THEN it exits 1 and prints the failure object coded `message-not-found`, naming `999` and the mailbox

## MODIFIED Requirements

### Requirement: The JSON contract is described and versioned
Its code table reads, for these rows:

| Code | Raised when |
|---|---|
| `message-not-found` | a message id given is not in the mailbox: an `errors` entry beside what was done; for the raw `maildir` and `mbox` commands, one of them is not, or the one message a command reads is not (every backend), and nothing is done |
| `permission-denied` | the server refused the item, or the command, to the account: a JMAP `forbidden` (SetError or method error), an HTTP 403 that is no rate limit (Gmail) or quota (Microsoft Graph), a JMAP HTTP 403, an IMAP `NOPERM` |
| `rate-limited` | the server still throttled the item, or the command, after the retries: an HTTP 429, a Gmail 403 `rateLimitExceeded`, `userRateLimitExceeded` or `quotaExceeded`, a JMAP `rateLimit` |

### Requirement: A session runs the CLI over one account
`himalaya repl` SHALL start a session bound to one account, its configuration and one `--backend`, and run pimalaya-cli's shared REPL loop over it: read one command per stdin line, split as a shell splits it, a blank line or a line whose first word starts a `#` comment skipped, parse it with the one-shot grammar plus `--json`, subcommand names inferred from a unique prefix, and run it, until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it with `Cannot start a session inside a session`, nothing started. A line whose quote is not closed, a line that is not UTF-8, a parse error or a command error SHALL be printed, on stdout as the failure object when piped, a parse error coded `usage-invalid` with clap's message without its `error: ` prefix, a command error coded as a one-shot run codes it, and the loop SHALL continue.

## REMOVED Requirements

None.
