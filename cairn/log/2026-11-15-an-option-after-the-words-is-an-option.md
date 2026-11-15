---
cairn: log
change: an-option-after-the-words-is-an-option
landed: 2026-11-15
---

# An option after the words is an option

Task H27 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (part2-pimdir.md, F2): an option after a pimdir query was read as query words, so `pimdir search from:x -s 3` found nothing, a trailing `--json` printed text, and `pimdir tag --add X <query> --dry-run` staged for real, `--dry-run` becoming the text term "dry run".

## What landed

- src/pimdir/search.rs, count.rs and tag.rs: the query takes its words alone, `trailing_var_arg` and `allow_hyphen_values` gone; the help says a word opening on `-` goes after `--`, or reads `not`.
- The same trap closed in src/shared/message/arg.rs (the message of `message add|send` and the protocol saves: `message send "<message>" --save Sent` sent ` --save Sent` as body text), src/sieve/script.rs (`sieve put|check`) and src/jmap/query.rs (`jmap query`). No other positional in himalaya carries either attribute. MCP puts its positionals after `--`, so its calls read as before.
- src/cli.rs tests `an_option_after_a_pimdir_query_is_the_option`, `an_option_after_a_message_is_the_option`, `an_option_after_a_sieve_script_is_the_option`, `an_option_after_jmap_method_calls_is_the_option`: each failed with the old attributes put back. tests/pimdir_search.rs `an_option_after_the_query_is_the_option` through the binary.

## Capabilities moved

- [commands](../spec/commands.md): *An option after the words is an option* added; *Raw message input is shared* reads options wherever they stand.
