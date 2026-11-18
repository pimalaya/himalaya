---
cairn: delta
change: values-refused-before-the-session
---

# Delta

## ADDED Requirements

### Requirement: A value a protocol command refuses is refused with the command line
A value of a protocol command's arguments the command would refuse whatever the server answers SHALL be refused with the command line, coded `usage-invalid`, exit 2, before the configuration is read, the secret command run or the session opened: the sequence set of `imap fetch`, `store`, `copy` and `move`; `--mail-from` without `--rcpt-to`, or the reverse, on `jmap submission create`; `msgraph messages update` naming no change; the method and an off-origin URL of `gmail request` and `msgraph request`; and `never` beside another condition in `smtp send --notify`. A value read from a file or stdin (a raw command, a JMAP request body, a Sieve script) SHALL be read and checked inside the command, a session refusing stdin as `stdin-taken`.

#### Scenario: A sequence set that is none
- GIVEN an IMAP account whose password is a command
- WHEN `himalaya --json imap fetch -m INBOX abc` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, and the password command did not run

## MODIFIED Requirements

### Requirement: The Gmail and Graph escape hatches are requests
When built with the `gmail` or `msgraph` feature, `gmail request <METHOD> <PATH> [JSON]` and `msgraph request <METHOD> <PATH> [JSON]` SHALL send one HTTP request under the account's bearer token, as Cardamum's `gpeople request` and `msgraph request` do: METHOD `get`, `post`, `patch` or `delete`, `put` too on Gmail, in any case, another refused with the command line, exit 2; PATH joined onto `https://gmail.googleapis.com/gmail/v1/` or `https://graph.microsoft.com/v1.0/`, a leading `/` dropped, or a full URL taken as it is when it stays on that API's origin (`https://gmail.googleapis.com`, `https://graph.microsoft.com`), a URL off it refused with the command line, exit 2, the bearer token never leaving for another host; JSON the body of a `post`, `put` or `patch`, `null` when absent. They SHALL print `{response}`, the body the provider answered as it sent it, `null` for an empty one, its text form the body pretty-printed, nothing for an empty one. A refusal SHALL print as the typed commands' refusals do, under `refusal`, exit 3 (*A protocol refusal is a result*).

#### Scenario: A Graph patch
- GIVEN a Microsoft Graph account
- WHEN `himalaya --json msgraph request patch /me/mailFolders/F1 '{"displayName":"Work"}'` runs
- THEN it sends `PATCH /v1.0/me/mailFolders/F1` with that body and prints `{"response": <the folder Graph answered>}`

### Requirement: An IMAP command prints its completion
Every `imap` command but `raw` SHALL print the tagged completion of its exchange: `status: "OK"` beside what the command understood of the server's answer, or `{status, text}` when the server answers `NO` or `BAD`, exiting 3 after it. A command getting no completion (a network, TLS or parse failure, a local error) SHALL fail with the failure object. `status` SHALL request and print the RFC 3501 items alone (`messages`, `recent`, `unseen`, `uidNext`, `uidValidity`), each `null` when the server leaves it out, and no key it never requests. `select` SHALL print the mailbox's state (`flags`, `exists`, `recent`, `firstUnseen`, the number of the first unseen message, `permanentFlags`, `uidNext`, `uidValidity`, `highestModSeq`), `expunge` the expunged message numbers, `store` the FETCH data the server returns, `append` the `APPENDUID` it returns (UIDPLUS), `copy` and `move` the `COPYUID` (`uidValidity`, and `sourceUids` beside `destinationUids`, each source UID at the place of the UID it got), `null` and empty without UIDPLUS or when it counts over a million messages, the mailbox writes `{mailbox}` and `close` and `unselect` the status alone. `fetch` SHALL print envelope addresses as `{name, email}`, and `search` its ids as numbers, as `sort` does. io-imap returns a typed exchange's output whole, so these commands print no events. `thread` SHALL be refused with `capability-missing` before anything is sent when the server does not advertise `THREAD=` the algorithm.

#### Scenario: A mailbox that does not exist
- GIVEN an IMAP account
- WHEN `himalaya --json imap select Nope` runs
- THEN it prints `{"status":"NO","text":"…"}` with the server's text and exits 3

