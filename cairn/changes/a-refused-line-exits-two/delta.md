---
cairn: delta
change: a-refused-line-exits-two
---

# Delta

The requirements below are folded as they now read in the spec; each entry names its capability and what changed.

## ADDED Requirements

### Requirement: The exit status tells the outcome
(commands) A command SHALL exit 0 when it succeeds and 1 when it prints the failure object. A command line the grammar refuses (an unknown flag or command, a missing argument, a bad value) SHALL exit 2, parsed through pimalaya-cli's `clap::parse`: with a `--json` word before any `--`, it SHALL print on stdout the failure object coded `usage-invalid`, its `error` clap's message without its `error: ` prefix or styles, and clap's rendering on stderr otherwise. A command that printed its result without fully succeeding (a refused protocol completion, an `account check` with a failed backend) SHALL exit 3, printing nothing after its result. Help and version SHALL print and exit 0 as clap does.

#### Scenario: A mistyped flag under --json
- WHEN `himalaya --json envelope list --nope` runs
- THEN stdout holds the failure object `{"code":"usage-invalid","error":"unexpected argument '--nope' found…","sources":[],"backtrace":null}`, stderr is empty, and the exit status is 2

#### Scenario: A failed check
- GIVEN an account whose Maildir root does not exist
- WHEN `himalaya --json account check` runs
- THEN it prints the report, the `maildir` row `ok: false`, and the exit status is 3

## MODIFIED Requirements

### Requirement: The JSON contract is described and versioned
(commands) The failure object's `code` is typed by the code vocabulary and pimalaya-cli's `usage-invalid`; the code table gains `usage-invalid`, a command line, a REPL line or an MCP tool's arguments the grammar refuses, nothing run.

### Requirement: Raw passthrough is byte-verbatim
(commands) A refused completion (an IMAP `NO` or `BAD`, a ManageSieve `NO` or `BYE`, an SMTP 4xx or 5xx) SHALL exit 3, its result printed (was 2).

### Requirement: account check covers every declared backend
(commands) When any backend fails, it SHALL exit 3 after the report (was 2).

### Requirement: An IMAP command prints its completion
(commands) A `NO` or `BAD` prints `{status, text}`, exiting 3 after it (was 2); the scenario exits 3.

### Requirement: A session runs the CLI over one account
(repl) A parse error printed as the failure object when piped SHALL be coded `usage-invalid`, with clap's message without its `error: ` prefix.

### Requirement: A tool's input is the command's grammar
(mcp) An undeclared argument, a value of the wrong type, a count other than one, and a value the command's grammar refuses once the argv is parsed (a negative `page-size`) SHALL carry the code `usage-invalid`, the last with clap's message without its `error: ` prefix.

### Requirement: Live provider suites
(packaging) The anonymous modules assert 3 with the `account check` report (was 2), 1 with the failure object.
