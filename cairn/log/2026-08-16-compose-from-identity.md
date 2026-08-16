---
cairn: log
change: compose-from-identity
landed: 2026-08-16
---

# The account speaks for an address again

[Issue 721](https://github.com/pimalaya/himalaya/issues/721) reported `messages compose` producing a message with no `From`, and having to pass `--from` by hand before every send. The account was already resolved, and nothing on it could answer the question: v2 rebuilt the config around what each backend needs to connect, and an address is not that. v1 carried `email` and `display-name` and its template builder read them.

## What landed

`email` and `display-name` on `[accounts.<name>]`, and `display-name` at the top level as the default name for every account. The address stays account-only: it is per-mailbox by nature, and a global one would be wrong for every account but the first.

**Both spellings, both binaries.** himalaya-tui had kept the same two fields under `from` and `from-name`, because it composes and could not do without, and the two binaries share one configuration file. So each now takes the other's names as serde aliases: the CLI reads `from`/`from-name`, the TUI reads `email`/`display-name`. No existing file has to be rewritten, whichever binary wrote it.

**The composers fill the header.** `compose`, `reply` and `forward` share one builder and one `--from` flag, so the default is resolved in one place, [`Account::resolve_from`](../../src/account/context.rs): the flag wins whole, otherwise the merged account answers, and neither leaves the header out rather than guessing. An explicit `--from` also drops the configured name, an address the user spelled out being theirs entire.

**The name travels apart from the address.** `BuilderArgs` gained `from_name`, and the builder now calls `Address::new_address(name, address)` where it passed a bare `&str`. mail_builder quotes and RFC 2047-encodes the display name itself, so `Doe, Alice` comes out as `"Doe, Alice" <alice@example.org>` and this code owns no quoting rule. Formatting the pair into one string would have needed one, and an address parser to take it back apart.

**The wizard writes the address it was handed.** Its single prompt takes an email, a server URL or a folder path; `prompted_email` reads it back as an address when that is what it was, and a `scheme://` input is refused before the local-part check, its `@` being a credential rather than a mailbox.

## Left out

`signature` and `signature-delim`, the other two v1 fields the TUI carries and the composers have flags for. They are body, not address, and the issue is about the header.

The identity is not required and not validated. An account that never composes has no use for it, and Himalaya is not the last word on what an addr-spec looks like.

`-a/--account` stays required for `compose`, which the issue also questioned. The command routes through `--save` and `--send`, both of which need the resolved account's backend.

## Capabilities moved

- **config**: added the account identity requirement, with the cross-binary alias rule.
- **commands**: added the requirement that the composers default the `From` header, and that the name reaches the MIME builder apart from the address.
- **wizard**: added the requirement that a prompted address is kept as the account's `email`; the generated-account ordering requirement now names the identity between `default` and the storage backend.
