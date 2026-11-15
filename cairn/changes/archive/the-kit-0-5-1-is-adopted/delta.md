---
cairn: delta
change: the-kit-0-5-1-is-adopted
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A session runs the CLI over one account
`himalaya repl` SHALL start a session bound to one account, its configuration and one `--backend`, and run pimalaya-cli's shared REPL loop over it: read one command per stdin line, split as a shell splits it, a blank line or a line whose first word starts a `#` comment skipped, parse it with the one-shot grammar plus `--json`, subcommand names inferred from a unique prefix, and run it, until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it by pimalaya-cli, by name, alias or inferred prefix, with `Cannot start a session inside a session` coded `usage-invalid`, nothing started. A line whose quote is not closed, a line that is not UTF-8, a parse error or a command error SHALL be printed, on stdout as the failure object when piped, a parse error coded `usage-invalid` with clap's message without its `error: ` prefix, a command error coded as a one-shot run codes it, and the loop SHALL continue.

#### Scenario: A session inside a session
- GIVEN a piped session started with `--json`
- WHEN the lines `repl`, `mcp --allow-write` and `envelope list` are read
- THEN the first two answer the failure object `Cannot start a session inside a session`, coded `usage-invalid`, and the third lists the inbox

### Requirement: Help keeps JSON lines whole
A line asking for help or the version SHALL print clap's text on stdout in a text session, and on stderr when the session's `--json` or a `--json` word of the line before any `--` is set, so stdout keeps one JSON value per line. The version SHALL be the one the command enum carries, the binary's: `-V` its version, `--version` its long version, after a command too, named after the command as the command line names it (`himalaya-envelope-list v<version> ...`). Its usage line SHALL name the binary before the command, as `Usage: himalaya envelope list [OPTIONS]`. A failing line SHALL hint at restarting the session with `--log-level` for more verbose logs, the flag being fixed for the session.

#### Scenario: The version in a session
- GIVEN a text session
- WHEN the lines `-V` and `envelope list --version` are read
- THEN stdout holds `himalaya <version>`, then the long version the command line prints for `himalaya envelope list --version`

### Requirement: A tool's input is the command's grammar
A tool's `inputSchema` SHALL be generated from its clap command by pimalaya-cli's `mcp::grammar::Grammar`, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command from the argv the grammar writes. The one exception SHALL be the server-only arguments its tool declares, which clap never sees. An argument the `inputSchema` does not declare SHALL be refused before the command runs as a tool error ``Unknown argument `<key>`, <tool> takes <properties>``, the server-only ones named among them. A value of the wrong type, and a count other than one for an argument the tool takes one value of, SHALL be refused the same way. A required argument left out or `null` SHALL be refused before the command runs as ``Missing argument `<name>` ``. Each of these refusals, and a value the command's grammar refuses once the argv is parsed, SHALL carry the code `usage-invalid`, the last named as the tool's input names it through pimalaya-cli's `Grammar::usage_error`, ``Invalid value `-1` for `page-size`: <reason>`` for a negative `page-size`, clap's message without its `error: ` prefix when it names no property. A tool's property takes a lone value where the command repeats one, so `query` is a string or an array of words; `page` and `page-size` are integers; `attachment_download` takes `attachment-id` as one string. A tool naming one message SHALL take it as `message-id`, as the CLI names it: `message_read`, `message_reply`, `message_forward`, `attachment_list` and `attachment_download`.

#### Scenario: A message named by its id
- GIVEN a server over a Maildir holding the message `m1`
- WHEN `message_read` is called with `{"message-id": "m1"}`, then with `{"id": "m1"}`
- THEN the first answers the read, `id` `m1` and `etag` `null`, and the second is refused coded `usage-invalid`, naming `message-id` among the properties the tool takes

#### Scenario: A required argument left out
- GIVEN a server over a Maildir
- WHEN `message_read` is called with `{}`
- THEN it is refused coded `usage-invalid` with ``Missing argument `message-id` ``, the command not run

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`, and `msgraph messages get --raw`. The byte writer SHALL write the bytes verbatim in text mode and print `{data, size, mime}`, `data` in base64, under `--json`, so no command writes raw bytes to a JSON stdout; given a path, it SHALL write the file and print pimalaya-cli's `output::SavedOutput`, `{path, size}`, `Saved <size> bytes to <path>` in text. `gmail attachments get`, `msgraph attachments get` and `jmap email export` SHALL print through it.

## REMOVED Requirements

None.
