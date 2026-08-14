---
cairn: log
change: gmail-command-module-layout
landed: 2026-08-14
---

# Gmail command module layout

The gmail subtree moved to one file per subcommand, the shape msgraph, imap and jmap already used. Sixteen files became seventy-eight: each resource is now a sibling module carrying its `pub mod` declarations and its `Command` enum, next to a folder of subcommands. src/gmail/messages.rs alone was 488 lines holding eleven commands, a table and two output types, and it was still growing as gmail-structured-json-output added more.

Types followed the code that owns them: `MessageIdsTable` into messages/list.rs, the message outputs into messages/get.rs, `HistoryTypeArg` into history/list.rs. Three helpers crossed subcommands and got a home of their own: `FormatArg`, previously copy-pasted three times, into src/gmail/format.rs; the filter criteria and action summaries into settings/filters/summary.rs; and `LabelsTable::new`, lifted out of the labels list and get commands that were building it identically.

Twelve command types in the settings subtree were named for their verb alone, `Get`, `List`, `Create` and `Delete` declared three times over across delegates, filters and forwarding addresses. They compiled only because the modules separated them, and cli-001 allows dropping a domain prefix only where the bare name would not collide, which these plainly did. They are now `GmailSettings<Target><Verb>Command`. The send-as commands regained the `Settings` target their own parent enum already carried.

The `Gmail` prefix was deliberately not added to the tables and value enums, `LabelsTable`, `MessageIdsTable`, `FormatArg`, `DispositionArg` and the rest. None of them collide, so cli-001 leaves them bare. The bare verbs were renamed precisely because they did collide, which is the distinction the rule turns on.

Gmail's local `read_message` was dropped for the shared `MessageArg`, leaving gmail no longer the only backend resolving message input its own way. Gmail gains file-path input, CRLF normalisation and up-front rejection of an empty message rather than sending it and reading back an opaque Gmail error. One user-visible consequence: `MessageArg` sets `raw = true`, so the inline form now takes the `--` separator, as it already did on the shared, IMAP, JMAP, Maildir, SMTP and Graph commands. Piped stdin, the scripted path, is unaffected. No changelog entry, since this is convergence on an existing contract rather than a new one.

Blank lines between struct fields and enum variants were removed across the subtree per inline-003. The rest of the tree, msgraph, imap, jmap and shared, has not been swept.

src/gmail/mod.rs and src/gmail/cli.rs keep the aggregator-plus-cli shape. Folding them into a src/gmail.rs would have put gmail out of step with imap, jmap and msgraph at the backend level, which is a wider decision than this change.

Verified: the CLI surface is unchanged, checked against the command tree and per-command help; the schema registry still generates the same 23 Gmail schemas; build, fmt and clippy clean; 88 tests pass.

Spec updated: commands (ADDED: One module per subcommand, Command types carry a domain and a target, Raw message input is shared).
