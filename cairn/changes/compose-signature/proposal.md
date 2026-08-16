---
cairn: change
id: compose-signature
status: landed
created: 2026-08-16
---

# Give the signature back to the config, and settle what the value means

## Why

[compose-from-identity](../compose-from-identity/proposal.md) put `email` and `display-name` back and left `signature` and `signature-delim` out, saying they belonged to their own change. This is that change, and the argument for it is the same one: a signature is a per-account constant, `--signature` exists on all three composers, and retyping it on every invocation is what [issue 721](https://github.com/pimalaya/himalaya/issues/721) is about.

What makes it more than two more `Option<String>` is that the two binaries currently disagree about what the value holds. The CLI hardcodes the block in `compose_body`: `"\n\n-- \n"` then the text, so the value would be the signature alone. himalaya-tui hands its configured `signature` straight to mml's template builders, which append it verbatim with no separator of their own, so there the value has to carry `-- \n` itself, and `signature-delim` is parsed and inert. Adding the field to the CLI without settling this would ship one file rendering two different bodies, which is the exact failure a shared configuration file exists to prevent.

## What

`signature` and `signature-delim` on `[accounts.<name>]` and at the top level, with the account overriding the global as every other pair does.

**`signature` is the text alone, `signature-delim` is what introduces it,** defaulting to the RFC 3676 §4.3 `"-- \n"` the CLI used to hardcode. The delimiter is written verbatim, its own trailing newline included: a rule that appends one for you cannot express a delimiter that deliberately has none.

**himalaya-tui assembles the block the same way**, from the same two keys, before handing it to mml. That makes its `signature-delim` live and reinterprets its `signature`: a value that used to carry `-- \n` now gets one prepended. It is a breaking reinterpretation, and the moment to take it is now, himalaya-tui being at v0.1.0 with its initial release unreleased and no installed base to break. Detecting a delimiter already present in the value and stripping it would be worse than the breakage.

**Precedence mirrors `--from`.** `--signature` wins; `--signature-file` names the file the builder reads, so the configured signature stands down rather than shadowing it; with neither, the account answers.

## What this is not

The signature is not read from a file when the value happens to name one. v1's `signature` did that, if memory serves, and it is not worth reviving: `--signature-file` covers the case explicitly, and a literal that silently becomes a path read is a surprise waiting for the first user whose signature is a single word that matches a filename.

The blank line between the body and the signature is not configurable. Both composers write one, mml because its template builder separates every segment that way and the CLI because it always has.
