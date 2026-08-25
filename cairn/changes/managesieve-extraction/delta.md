---
cairn: change-delta
id: managesieve-extraction
status: landed
---

## MODIFIED Requirements

### Requirement: ManageSieve account transport

An account MAY declare a `sieve` block using `sieve://`, `sieves://`, or `unix://` server URLs. The transport SHALL be io-managesieve's, which owns the scheme table, the greeting, the optional STARTTLS upgrade and the SASL exchange; Himalaya SHALL only turn the config block into its arguments.

A bare authority SHALL resolve to `sieve://` with STARTTLS on, RFC 5804 registering one port and no implicit-TLS twin. `sieve.starttls` left unset SHALL follow the scheme.

Every SASL mechanism the other backends accept SHALL reach ManageSieve, the protocol framing every mechanism identically. A mechanism disclosing a reusable credential SHALL be refused over a cleartext connection unless `sieve.allow-cleartext-auth` is set.

### Requirement: ManageSieve protocol commands

When built with the `sieve` feature, Himalaya SHALL expose protocol-specific commands for `capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Commands SHALL use the resolved account and SHALL keep protocol data output separate from confirmation messages. `put` and `check` SHALL report the warning text a server attaches to an accepted script.

### Requirement: ManageSieve wire safety

Wire framing SHALL live in io-managesieve rather than in this repository: the CRLF lines, the ACAP-style quoted strings and literals, the response codes, the refusal of cleartext credentials, and the refusal of bytes arriving past a greeting or a STARTTLS reply. Himalaya SHALL surface a server failure as a user-visible error carrying the response code the library parsed.

### Requirement: Local ManageSieve verification

Protocol framing and command sequencing SHALL be covered by io-managesieve's own tests, whose scripted server asserts the exact bytes each command sends. This repository SHALL keep only the configuration tests for the `sieve` block. Live provider tests SHALL remain separate from the default test suite and SHALL use only explicitly safe test scripts.
