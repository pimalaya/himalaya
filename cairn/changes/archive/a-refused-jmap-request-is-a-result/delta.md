---
cairn: delta
change: a-refused-jmap-request-is-a-result
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: A protocol refusal is a result
Inside the `gmail`, `msgraph`, `jmap`, `sieve` and `smtp` namespaces, a completion the server refuses SHALL print as the command's result and exit 3, as `imap` and the `raw` commands do (pimalaya-cli cairn/spec/output.md, *A protocol refusal is a result*); a command getting no completion (a network, TLS or decoding failure, a local error, a value refused before sending) SHALL print the failure object and exit 1. An HTTP 4xx or 5xx a `gmail`, `msgraph` or `jmap` command meets (the `request` commands included), a JMAP request refused whole (RFC 8620 §3.6.1), a JMAP method error (§3.6.2) and the SetError refusing the one write of a `jmap` command (§5.3) SHALL print `{refusal: {status, reason, message, code, retryAfter}}`: `status` the HTTP status, `null` for a refusal inside a 2xx; `reason` the provider's name for it in its spelling (Gmail's first reason, else its canonical status; Graph's `error.code`; the JMAP error type, the problem type's URN namespace stripped), `null` when it names none; `message` the provider's text, the type standing in when a JMAP error carries none; `code` the code the failure would carry (*A server's refusal codes the whole command*), `null` when none applies; `retryAfter` the seconds the server asked to wait, `null` when it asked none. A typed `sieve` command answered `NO` or `BYE` SHALL print the completion `{status, code, text}` as `sieve raw` prints it. `smtp send` refused SHALL print `{refusals: [{rcptTo, code, text}]}`, one reply per refusal in the order received (a refused `MAIL FROM` or `DATA` with `rcptTo` `null`, one per recipient when the server refused them all), the message reaching nobody. Each command's JSON Schema SHALL describe its own result and the refusal as two alternatives told apart by their keys.

#### Scenario: A Gmail id that does not exist
- GIVEN a Gmail account answering 404 `notFound`
- WHEN `himalaya --json gmail request get users/me/labels/nope` runs
- THEN it prints `{"refusal":{"status":404,"reason":"notFound","message":"Requested entity was not found.","code":null,"retryAfter":null}}` and exits 3

#### Scenario: A recipient the server refuses
- GIVEN an SMTP server answering `RCPT TO:<refused@example.org>` with `550 5.1.1 No such user`
- WHEN `himalaya --json smtp send -f t@example.org -t refused@example.org <message>` runs
- THEN it prints `{"refusals":[{"rcptTo":"<refused@example.org>","code":550,"text":"5.1.1 No such user"}]}` and exits 3

#### Scenario: A JMAP request refused whole
- GIVEN a JMAP server answering a request whose `using` names `urn:bogus` with HTTP 400 and the problem type `urn:ietf:params:jmap:error:unknownCapability`
- WHEN `himalaya --json jmap request -- '{"using":["urn:bogus"],"methodCalls":[["Mailbox/get",{},"0"]]}'` runs
- THEN it prints `{"refusal":{"status":400,"reason":"unknownCapability","message":<the problem's detail>,"code":null,"retryAfter":null}}` and exits 3

## REMOVED Requirements
