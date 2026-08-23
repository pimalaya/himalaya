---
cairn: change
change: capability-shared-api
---

# Delta

## ADDED Requirements

### Requirement: Capability registry
Each backend SHALL declare which shared capabilities it implements. A backend SHALL register a capability only when its implementation produces the same observable result as every other registrant; an implementation whose outcome differs SHALL stay protocol-specific rather than register a shared verb with a second meaning.

### Requirement: Static and dynamic capability tiers
Capability SHALL be resolved in two tiers. The static tier, derived from the backend kind and the account's configured transports, SHALL be consulted before any connection, so an unsupported verb fails without resolving credentials, opening TLS or authenticating. The dynamic tier, read from what the server advertises (IMAP `CAPABILITY`, the JMAP session capabilities, the ManageSieve `SIEVE` line), SHALL be consulted only after connection and only for operations whose availability varies per server.

### Requirement: Capability errors are actionable
A shared command with no registering transport SHALL fail with an error naming the account, the resolved backend, the missing capability, and the protocol-specific command that serves it, if one exists.

### Requirement: The command tree is feature-independent
Argument parsing SHALL NOT depend on cargo features; only dispatch is gated. A verb whose implementations are all compiled out still parses and fails with the capability error, so `json-schema` output and shell completions are identical across builds. `--help` SHALL state which backends serve each shared command, generated from the registry.

### Requirement: The capability surface is backward compatible
The move to capability-based resolution SHALL be additive. No existing shared command changes name, arguments, defaults, resolution order or output, and success paths stay byte-identical in table and JSON form. The only permitted observable change is that a command which previously did not exist fails with the capability error instead of a clap unknown-subcommand error.

## MODIFIED Requirements

### Requirement: Shared commands over a selected backend
The shared commands SHALL run over an `EmailClient` that owns one backend-client variant per compiled-in backend, and SHALL resolve each verb to a configured transport that registers the verb's capability, rather than to a single backend chosen for all verbs. For the storage verbs this resolution SHALL reproduce the existing order: the first configured storage backend the global `--backend` flag allows, preferring local backends over network ones, plus the SMTP transport for storage backends that cannot send (IMAP, Maildir, m2dir). `--backend` SHALL remain a filter over resolution and the explicit override when several configured transports register the same capability.

## REMOVED Requirements
