---
cairn: change
id: capability-shared-api
status: active
created: 2026-08-24
---

# Capability-based shared API (union surface, no major bump)

## Why

The shared API is defined today as a strict least-common-denominator: a verb enters it only when every backend can serve it. That rule has held the surface coherent, but it pushes any partially-supported operation into the protocol namespaces, where the user has to know which protocol their provider speaks in order to find a feature.

Vacation is the case that exposes it. `VacationResponse` is a typed singleton in JMAP (RFC 8621 §8, already covered by io-jmap), Gmail exposes it as a settings resource (already covered by io-gmail), Graph exposes it as `mailboxSettings.automaticRepliesSetting` (not yet in io-msgraph), and ManageSieve can express it by rewriting a script. Under the LCD rule this can never be one command, so the same user-visible feature would be spelled `himalaya jmap vacation` and `himalaya gmail vacation`, and a user who does not know their provider's protocol cannot discover either.

The rule is also already bent where reality required it. `message send` and `compose` exist for any storage backend because SMTP is a separate transport, and `Backend::Auto` already resolves a backend rather than selecting one. The surface additionally varies with cargo features, so `himalaya json-schema` output is not stable across builds, which matters to himalaya-emacs, himalaya-vim and himalaya-tui.

## What

Replace the intersection rule with a **capability registry** and a resolver, so the shared surface is the union of what the configured account can actually do.

**Two tiers.** A static tier declares, per backend kind, which shared capabilities it implements; it is known without any connection, so an unsupported verb fails before config resolution, TLS and authentication. A dynamic tier reads what the server advertises once connected (IMAP `CAPABILITY`, the JMAP session capabilities, the ManageSieve `SIEVE` line) for operations a given server may or may not carry.

**Resolution, not selection.** An account can configure several transports at once (imap + smtp + sieve). Each shared verb resolves to the transport that registers its capability. `--backend` keeps its current meaning as a filter over that resolution, and stays the explicit override when more than one configured transport can serve the same verb.

**One meaning per verb.** A backend registers a capability only when its implementation produces the same observable result as the others. An implementation with a different observable outcome stays protocol-only. Vacation over ManageSieve is the worked example: it has no date window (RFC 5230 offers `:days`, not `fromDate`/`toDate`), and enabling it means owning or rewriting the account's single active script. It therefore does not register the shared capability and remains under `himalaya sieve`, even though the union rule would otherwise admit it.

## Compatibility

This is the hard part, and the constraint the design has to satisfy: the refactor is a minor release, not a 3.0.

- **Purely additive.** No existing shared command changes name, arguments, defaults, resolution order or output shape. The `EmailClient` rule (first configured storage backend the `--backend` flag allows, local preferred over network, plus an optional SMTP transport) becomes the registered capability of the storage verbs, expressed differently, decided identically.
- **Success paths stay byte-identical**, table and JSON alike. Existing `*Output` types are untouched; a new verb brings its own type and its own `json_schema.rs` entry.
- **The one observable change is the failure of a command that did not previously exist.** Where `himalaya vacation set` used to be a clap unknown-subcommand error, it becomes a capability error naming the account, the resolved backend, the missing capability, and the protocol-specific command that does work. Both are non-zero exits on a command that never functioned, so no working invocation changes behaviour.
- **Protocol namespaces are not renamed or removed.** A shared verb delegates to the same per-protocol code the protocol command calls, so `himalaya jmap vacation` keeps working next to a future shared `vacation`.
- **The clap tree stops depending on cargo features; only dispatch stays gated.** A verb whose implementations are all compiled out still parses and fails with the capability error. That is what makes `json-schema` output stable across builds, and it is why `--help` must state which backends serve each command instead of silently listing commands the binary cannot run.
- **No config migration.** Capabilities are derived from the account's existing transport sections.

## Scope / non-goals

- No new protocol coverage: this moves existing operations, it does not add backends. `mailboxSettings` in io-msgraph and a shared `vacation` are separate changes that depend on this one.
- The `--help` compatibility annotation is generated from the registry, never hand-maintained, or it goes stale silently (rclone's optional-features matrix is the cautionary prior art).
- The guardrail LCD provided mechanically becomes a written rule plus review attention. That is the real risk of this change, and it is accepted deliberately.
