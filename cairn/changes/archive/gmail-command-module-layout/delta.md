---
cairn: change
change: gmail-command-module-layout
---

# Delta

## ADDED Requirements

### Requirement: One module per subcommand
A protocol command resource SHALL live in a sibling module file carrying its `pub mod` declarations and its `Command` enum, next to a folder holding one file per subcommand. Types live with the subcommand that owns them; a type serving several subcommands lives in the file of the one that owns it, or in its own module when it belongs to none.

### Requirement: Command types carry a domain and a target
A protocol command type SHALL be named `<Domain><Target><Verb>Command`, so a bare verb is never a type name. The domain prefix stays wherever the bare name would collide across the backends a CLI spans, and is otherwise omitted for the tables and value enums under the cli subtree.

### Requirement: Raw message input is shared
A command taking a raw RFC 5322 message SHALL resolve it through the shared `MessageArg`: a file path, an inline value after `--`, or piped stdin. The resolved message is normalised to CRLF and rejected when empty, so no backend receives a zero-length message.

## MODIFIED Requirements

## REMOVED Requirements
