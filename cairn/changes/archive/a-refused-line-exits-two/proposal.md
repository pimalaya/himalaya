---
cairn: change
id: a-refused-line-exits-two
status: landed
created: 2026-11-13
---

# A refused line exits two

## Why

A command line clap refuses exited 2 with clap's text on stderr whatever `--json` said, while `account check` with a failed backend and every refused protocol completion exited 2 too: a script could not tell a typo from a partial outcome, and a `--json` caller got no failure object for a typo. The local stores campaign (2026-11-13) left it open pending the owner's call, which pimalaya-cli settled the same day (pimalaya-cli a-refused-line-exits-two): 2 is clap's, partial outcomes move to 3. No release carried the old numbering.

## What

- pimalaya-cli pinned at eb37962, which moves `PartialOutcome::default()` to 3 and brings `clap::parse`, `error::usage_error` and the `usage-invalid` code.
- src/main.rs parses through `pimalaya_cli::clap::parse`: a refused command line exits 2, printing the failure object coded `usage-invalid` on stdout under `--json`, clap's text on stderr otherwise.
- src/mcp.rs routes the clap parse of a tool call's argv through `usage_error`, so a value the grammar refuses (a negative `page-size`) is coded `usage-invalid` like the argument refusals pimalaya-cli codes itself.
- `account check` with a failed backend and the protocol refusals (IMAP `NO` or `BAD`, SMTP 4xx or 5xx, ManageSieve `NO` or `BYE`) exit 3: doc comments, the spec, README, CHANGELOG, MIGRATION and the live suites' helper follow.
