---
cairn: change
id: an-option-after-the-words-is-an-option
status: landed
created: 2026-11-15
---

# An option after the words is an option

## Why

Task H27 of the family's cli-alignment-2026-11-15 plan, a defect live testing found on 2026-11-15 (live-testing-2026-11-15, reports/himalaya/part2-pimdir.md, F2). The query of `pimdir search`, `pimdir count` and `pimdir tag` was `trailing_var_arg` and `allow_hyphen_values`, so every word after the first query word was query: `pimdir search from:x -s 3` searched `from:x -s 3` and found nothing, a trailing `--json` was swallowed into the query, and a trailing `--dry-run` became a text term (`pimdir count kind:mail` 3833, `... --dry-run` 0), so `pimdir tag --add X <query> --dry-run` staged for real on every hit whose text held "dry run". The message of `message add|send` and the protocol saves, the script of `sieve put|check` and the method calls of `jmap query` had the same trap: `message send "<message>" --save Sent` sent ` --save Sent` as body text and saved nothing.

## What

The words of those positionals are the positional values alone, clap reading every option wherever it stands, the global `--json` included. A word opening on a hyphen goes after `--`: a pimdir negation `-from:x` is written `-- -from:x`, or `not from:x`. A query word opening on a hyphen is the one thing that breaks: it was read as a word, it is now refused as an unknown option until moved after `--`.
