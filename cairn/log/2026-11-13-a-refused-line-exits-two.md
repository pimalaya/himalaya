---
cairn: log
change: a-refused-line-exits-two
landed: 2026-11-13
---

# A refused line exits two

Himalaya follows pimalaya-cli's exit contract (pimalaya-cli a-refused-line-exits-two), closing the usage errors under `--json` the local stores leftovers left open. pimalaya-cli is pinned at eb37962. src/main.rs parses through `pimalaya_cli::clap::parse::<Cli>()`: a refused command line still exits 2, and with a `--json` word before any `--` prints the failure object coded `usage-invalid` on stdout instead of clap's text on stderr. src/mcp.rs maps the clap error of a tool call's argv through `pimalaya_cli::error::usage_error`, so a value the grammar refuses (`page-size: -1`) is coded `usage-invalid` like the undeclared, mistyped and miscounted arguments pimalaya-cli codes itself; a refused REPL line is coded by pimalaya-cli alone. `PartialOutcome::default()` now exits 3, which moves `account check` with a failed backend and every protocol refusal (IMAP `NO` or `BAD`, SMTP 4xx or 5xx, ManageSieve `NO` or `BYE`) from 2 to 3; no code of Himalaya named the status, only doc comments, the `account check` unit test and the live suites' `json_check_failure` did.

Tests: tests/exit.rs (new, run on the built binary: `--json envelope list --nope` coded and exiting 2, the same without `--json` on stderr, `--json` after `--` left a positional, a Maildir `account check` on a missing root exiting 3), tests/mcp.rs (an undeclared argument and a negative `page-size` coded `usage-invalid`), tests/repl.rs (a refused line coded). Against the old pin all four of the coded or renumbered assertions failed; with the new pin alone the MCP `page-size` one still failed until src/mcp.rs mapped the error.

Capabilities moved: commands ("The exit status tells the outcome" added; "The JSON contract is described and versioned" and its code table, "Raw passthrough is byte-verbatim", "account check covers every declared backend", "An IMAP command prints its completion"), repl ("A session runs the CLI over one account"), mcp ("A tool's input is the command's grammar"), packaging ("Live provider suites"). README gains an exit code FAQ; CHANGELOG and MIGRATION renumber and name `usage-invalid`.

Left: a REPL line setting a session flag (``Cannot set `-a` inside a session``) is refused by pimalaya-cli uncoded, not `usage-invalid`; `attachment download --stdout` given several ids is a command refusal (exit 1, uncoded), not clap's.
