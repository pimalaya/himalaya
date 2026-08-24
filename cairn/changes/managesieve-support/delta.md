---
cairn: change-delta
id: managesieve-support
status: landed
---

## ADDED Requirements

### Requirement: ManageSieve account transport

An account MAY declare a `sieve` block using `sieve://`, `sieves://`, or
`unix://` server URLs. The transport SHALL support implicit TLS, STARTTLS,
and the account's configured SASL credentials where the mechanism is
supported by ManageSieve.

### Requirement: ManageSieve protocol commands

When built with the `sieve` feature, Himalaya SHALL expose protocol-specific
commands for `capability`, `list`, `get`, `put`, `check`, `activate`,
`deactivate`, `delete`, and `raw`. Commands SHALL use the resolved account
and SHALL keep protocol data output separate from confirmation messages.

### Requirement: ManageSieve wire safety

The client SHALL frame CRLF-terminated commands and RFC 5804 quoted/literal
strings, consume complete server responses, reject unsafe cleartext password
authentication, and preserve server failure status as a user-visible error.

### Requirement: Local ManageSieve verification

Protocol framing and command sequencing SHALL be covered by unit tests and a
local fake ManageSieve server. Live provider tests SHALL remain separate from
the default test suite and SHALL use only explicitly safe test scripts.

## MODIFIED Requirements

### Requirement: Protocol-specific APIs

The protocol-specific API list now includes `sieve`; it is an optional
service protocol and does not become a shared mail-storage backend.
