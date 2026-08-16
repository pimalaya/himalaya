---
cairn: delta
change: compose-signature
---

# Delta

## ADDED Requirements

### Requirement: Account signature
An account MAY declare a `signature` and the `signature-delim` introducing it, and both MAY also be declared at the top level, where they apply to every account that does not override them.

`signature` SHALL be the signature alone. `signature-delim` SHALL default to the RFC 3676 §4.3 `"-- \n"` and SHALL be written verbatim, its own trailing newline included, so a delimiter meant to stand on its own line says so rather than relying on a rule. Both binaries SHALL assemble the block from the same two keys, so one configured value reads the same whichever composes.

## MODIFIED Requirements

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.
