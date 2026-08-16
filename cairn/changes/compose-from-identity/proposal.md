---
cairn: change
id: compose-from-identity
status: landed
created: 2026-08-16
---

# Give the account back an identity, and let the composers use it

## Why

[Issue 721](https://github.com/pimalaya/himalaya/issues/721): `messages compose` produces a message with no `From` header unless `--from` is passed by hand, every time. The account the command already resolved knows which mailbox it speaks for, and the composer never asks it.

The reason is that v2 dropped the field. v1 carried `email` (required) and `display-name` on every account, and the template builder read them, so `From` was filled before the user saw the draft. v2 rebuilt the config around what each backend needs to connect, and an address is not that: IMAP authenticates with a username, Gmail with a token, and neither is necessarily the address the user sends as. So the identity was left out entirely, and with it the only thing that could answer `From`.

himalaya-tui kept the field under different names, `from` and `from-name`, because it composes too and could not do without. The two binaries share one configuration file, so the same account today carries the address under a key one of them ignores.

## What

`email` and `display-name`, back on `[accounts.<name>]`, plus `display-name` at the top level where it acts as the default for every account. Both binaries read both spellings: the CLI takes `from`/`from-name` as aliases, the TUI takes `email`/`display-name`, so one account block answers whichever binary opens it and no existing file has to be rewritten.

**The composers fill `From` from them.** `compose`, `reply` and `forward` share one builder and one `--from` flag, so they get the default in the same place: the flag wins when passed, otherwise the resolved account supplies the address, and `display-name` names it. With neither, the header is omitted exactly as it is today.

**The name is passed apart from the address**, not formatted into it. `mail_builder` takes an address as a display name plus an addr-spec and encodes the pair itself, so a name carrying a comma, a quote or a non-ASCII character comes out right without this code owning a quoting rule.

**The wizard writes the address it was already given.** Its first prompt takes an email, a server URL or a folder path; when what it got is an email, that is the account's `email`. It does not gain a display-name prompt: the wizard discovers, it does not interview, and the key is one line to add by hand.

## What this is not

`signature` and `signature-delim` are not added. They are v1 fields the TUI also carries and the composers also have flags for, but they are body, not address, and the issue is about the header. They belong to their own change.

The identity is not made required, nor validated as an addr-spec. An account that never composes has no use for it, and a `From` the user spelled out by hand is theirs.

`-a/--account` stays required for `compose`, which the issue also questions. The composer routes through `--save` and `--send`, both of which need the resolved account's backend, and a default account is one line of config; making the command work without one is a separate argument.
