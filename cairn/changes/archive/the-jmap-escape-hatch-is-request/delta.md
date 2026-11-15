---
cairn: delta
change: the-jmap-escape-hatch-is-request
---

# Delta

## ADDED Requirements

### Requirement: The JMAP escape hatch is a request
When built with the `jmap` feature, `jmap request [--using <URN>]... [METHOD_CALLS]...` SHALL send one JMAP request (RFC 8620 section 3.3) whose `methodCalls` is the JSON array given, read from stdin when it is `-` or absent (refused inside a session as `stdin-taken`), whose `using` is the core and mail capabilities plus each `--using`, and whose calls naming no `accountId` get the session's primary mail account. It SHALL print `{methodResponses}`, each response `[name, arguments, callId]` as the server answered it, its text form the same array. `request` is the escape hatch of an HTTP API across the family, `raw` the one of a line protocol (pimalaya-cli cairn/spec/commands.md). `jmap query`, its former name, SHALL run the same command, hidden from the help and the manual, and keep its schema key `himalaya-jmap-query` beside `himalaya-jmap-request`.

#### Scenario: The former name
- GIVEN a JMAP account
- WHEN `himalaya --json jmap query '[["Mailbox/get", {}, "c0"]]'` runs
- THEN it sends and prints what `jmap request` does for the same array, and `himalaya jmap --help` lists `request` alone

## MODIFIED Requirements

### Requirement: An option after the words is an option
Its first sentence names `jmap request` for `jmap query`:

A positional taking several words (the query of `pimdir search`, `pimdir count` and `pimdir tag`, the message `MessageArg` resolves, the script of `sieve put` and `sieve check`, the method calls of `jmap request`) SHALL leave every option an option wherever it stands, before, between or after the words, the global `--json` included, as `envelope search` does. A word opening on a hyphen SHALL go after `--`: a pimdir negation is `-- -from:alice`, or `not from:alice`; given before `--`, it is refused as an unknown option.

## REMOVED Requirements

None.
