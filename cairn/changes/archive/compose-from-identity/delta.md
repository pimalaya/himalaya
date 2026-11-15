---
cairn: delta
change: compose-from-identity
---

# Delta

## ADDED Requirements

### Requirement: Account identity
An account MAY declare the address it sends as, `email`, and the name that address carries, `display-name`. `display-name` MAY also be declared at the top level, where it applies to every account that does not override it. Neither is required, and neither is validated as an addr-spec: an account that never composes has no use for them.

Both keys SHALL also be accepted under the spellings himalaya-tui writes, `from` and `from-name`, and himalaya-tui SHALL accept `email` and `display-name` in turn, the two binaries sharing one configuration file.

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

### Requirement: The prompted address is kept
When the wizard's single prompt is answered with an email address rather than a server URL or a folder path, that address SHALL be written as the generated account's `email`. The wizard SHALL NOT prompt for a display name: it discovers, and a name is not discoverable.

## MODIFIED Requirements

### Requirement: A generated account reads in a deliberate order
The serializer SHALL decide what a generated account holds, so a defaulted field is omitted and no field is enumerated twice, but the rendering SHALL order what it emits: the groups run most-defining first (`default`, the identity, the storage backend, the transport, the mailboxes, the rendering options), an unrecognised group renders after them rather than being dropped, a group's `server` key reads before the credentials qualifying it, and a blank line separates groups.
