---
cairn: change
id: gmail-command-module-layout
status: landed
created: 2026-08-14
---

# Gmail command module layout

## Why

The gmail subtree kept every subcommand of a resource in one file: src/gmail/messages.rs held eleven commands, a table and two output types in 488 lines. msgraph, imap and jmap had already moved to one file per subcommand, so gmail was the outlier and the files kept growing with each output type added by gmail-structured-json-output.

Three smaller divergences travelled with it. `FormatArg` and `read_message` were copy-pasted across messages, drafts and threads. Twelve command types in the settings subtree were named for their verb alone (`Get`, `List`, `Create`, `Delete`, declared three times over), which naming-006 forbids and which only compiles because the modules separate them. And gmail resolved raw message input through a local helper while every other backend used the shared `MessageArg`.

## What

Each resource becomes a sibling module file carrying its `pub mod` declarations and its `Command` enum, next to a folder holding one file per subcommand. Types follow the code that owns them: `MessageIdsTable` into messages/list.rs, the message outputs into messages/get.rs, `HistoryTypeArg` into history/list.rs.

`FormatArg` is gathered into src/gmail/format.rs, shared by the three `get` commands that take it. The bare-verb command types are renamed `GmailSettings<Target><Verb>Command`, and the send-as commands regain the `Settings` target their own enum already carried. `read_message` is dropped for the shared `MessageArg`.

Blank lines between struct fields and enum variants are removed across the subtree, per inline-003.

## Scope / non-goals

The CLI surface is unchanged apart from raw message input. clap variant names, command paths, aliases and help text all stay as they were.

Adopting `MessageArg` does change one thing users can see: it sets `raw = true`, so the inline message form now takes the `--` separator, as it already did on the shared, IMAP, JMAP, Maildir, SMTP and Graph commands. Piped stdin, the scripted path, is unaffected. Gmail gains file-path input, CRLF normalisation and up-front rejection of an empty message.

The `Gmail` prefix is deliberately not added to the tables and args (`LabelsTable`, `MessageIdsTable`, `FormatArg`, `DispositionArg`). cli-001 keeps a domain prefix only where the bare name would collide, and none of these do. The bare verbs were renamed precisely because they did collide.

src/gmail/mod.rs and src/gmail/cli.rs keep the aggregator-plus-cli shape, since folding them would put gmail out of step with imap, jmap and msgraph at the backend level.
