---
cairn: log
change: compose-signature
landed: 2026-08-16
---

# The signature comes from the config, and means one thing in both binaries

[compose-from-identity](2026-08-16-compose-from-identity.md) landed `email` and `display-name` earlier today and left `signature` and `signature-delim` to their own change, on the grounds that they are body rather than address. The ergonomic argument was always the same one from [issue 721](https://github.com/pimalaya/himalaya/issues/721): a per-account constant that has to be retyped on every invocation is a configuration field missing.

## The reason it was not two more fields

The two binaries disagreed about what the value holds. The CLI hardcoded the separator in `compose_body` (`"\n\n-- \n"` then the text), so a config value would be the signature alone. himalaya-tui handed its configured `signature` straight to mml's template builders, which append what they are given verbatim, so there the value had to carry `-- \n` itself, and `signature-delim` was parsed and never read. Adding the CLI field without settling that would have shipped one file rendering two different bodies.

## What landed

`signature` and `signature-delim` at both levels, account over global, and one meaning for them: `signature` is the text, `signature-delim` is what introduces it, defaulting to the RFC 3676 §4.3 `"-- \n"`.

**The delimiter is written verbatim,** its own trailing newline included. [`compose_body`](../../src/shared/message/builder.rs) now pushes `"\n\n"`, the separator, then the text, which is byte-identical to the old hardcoded form under the default and lets a delimiter that deliberately ends without a newline exist at all.

**himalaya-tui assembles the same block,** through `signature_block` in its cli.rs, before mml sees it. That makes its `signature-delim` live and reinterprets its `signature`: a value that carried `-- \n` now gets one prepended. Taken deliberately, and taken now, that binary being at v0.1.0 with its initial release unreleased. Stripping a delimiter found inside the value would have been worse than the breakage. mml's own template body already separates segments with a blank line, so the layout matches the CLI's without either side arranging it.

**Precedence mirrors `--from`,** in [`Account::resolve_signature`](../../src/account/context.rs): the flag wins, and `--signature-file` wins too by having the config stand down, since the file is read later in the builder and a configured signature would otherwise shadow it. clap keeps the two flags mutually exclusive, so no third case exists.

## Left out

Reading the signature from a file when the value happens to name one, as v1 did. `--signature-file` covers that explicitly, and a literal that silently becomes a path read is a surprise waiting for the first single-word signature that matches a filename.

The blank line between body and signature stays fixed. Both composers write one and neither user asked for the other thing.

## Capabilities moved

- **config**: added the account signature requirement, with the value split between text and separator and the cross-binary assembly rule.
- **commands**: the composers requirement now covers the signature and its flag precedence alongside the `From` header.
