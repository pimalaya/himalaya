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

**One id domain.** Resolution is per verb, but the verbs that exchange ids (mailbox, envelope, message, flag, attachment) SHALL resolve to the same storage transport, whatever each could resolve to alone. On an account declaring both IMAP and JMAP, `envelope list` answering from one and `message read` from the other would hand an IMAP UID to JMAP, which does not fail cleanly: at worst it names another message. Today's `select_storage` guarantees this by picking one storage backend for all of them; the resolver states it as a rule. A verb whose ids belong to no message (sending, vacation, a sieve script) resolves on its own.

**Hidden when known, refused when not.** A caller that lists what it can do, the MCP server's `tools/list` (cairn/spec/mcp.md), lists only the verbs the static tier allows for the account: a tool guaranteed to fail costs the agent context on every turn and a call to find out. A verb only the dynamic tier can decide stays listed and fails at call time. The clap tree itself stays stable across accounts and builds (below), so `--help`, completions and `json-schema` do not vary with the account. The capability error carries a stable code, `capability-missing`, so a caller tells "not supported here" from "it broke" without reading the message.

**One meaning per verb.** A backend registers a capability only when its implementation produces the same observable result as the others. An implementation with a different observable outcome stays protocol-only. Vacation over ManageSieve is the worked example: it has no date window (RFC 5230 offers `:days`, not `fromDate`/`toDate`), and enabling it means owning or rewriting the account's single active script. It therefore does not register the shared capability and remains under `himalaya sieve`, even though the union rule would otherwise admit it.

## Capability inventory (2026-11-02)

Every operation himalaya implements today, shared or protocol-specific, across all its backends, read from the command tree and the shared adapters. It is the input of the registry: a row the registry declares is a capability, a row it does not stays protocol-specific.

**Names and model come from pimdir.** pimdir STORAGE Annex B already names the write actions (`mail.message.move`, `mail.flags.seen`, `mail.submit`, `collection.create`), with its grammar (domain, object, verb; an intent's option as one more word), its rule that a name never takes a second meaning, its `x-` prefix for an application's own, and its support levels: `full`, `partial` with a `detail` saying what is missing, `none` with a `detail` saying why. The registry reuses the grammar, the rules and the levels, without the domain word: pimdir needs it because one store holds mail, contacts and calendars, where every capability himalaya declares is mail, and a name reaching a caller does so inside `himalaya-error`, whose schema `$id` names the binary already. So himalaya's `message.move` is pimdir's `mail.message.move`, and its `mailbox.*` is pimdir's `collection.*`, the container of any kind, which himalaya's commands call a mailbox. The objects follow the commands (`mailbox`, `envelope`, `message`, `attachment`), `flags` keeping pimdir's plural. A pimdir account's capabilities are the store's declared rows (`load_capabilities`), which the pimdir adapter maps by one rule, `mail.` dropped and `collection.` anchored on a mail collection read as `mailbox.`: the only place the two spellings meet.

**pimdir is the write half, not the whole API.** Annex B names what a source pushes from a replica: it has no reads (a store answers them locally, always), no account features (vacation, identities, filters), and of collections only `create`. So the union is Annex B plus the read names and feature names below. The new names that are writes a replica would push too (renaming and deleting a mailbox, `mail.flags.important`, `mail.flags.junk`, the draft verbs) are proposed to pimdir as Annex B additions, the mailbox ones as `collection.rename` and `collection.delete` in pimdir's own spelling, so the two converge on one vocabulary instead of drifting.

Cells say what himalaya's code does, not what the protocol could: **F** full, **P** partial (detail in the notes), **D** decided by the server (dynamic tier), **none** the protocol cannot (the detail says why), **-** not implemented. **Name**: ✓ in pimdir Annex B, + new. **Today**: `shared` already a shared command, `proto` only under a protocol namespace. Maildir and m2dir share a column (m2dir only differs where noted); pimdir means the pimdir backend, its writes staged in the store's queue.

### Mailboxes

| Capability | Name | IMAP | JMAP | Gmail | Graph | Maildir | mbox | pimdir | Today |
|---|---|---|---|---|---|---|---|---|---|
| `mailbox.list` | + | F | F | F | F | F | F | F | shared |
| `mailbox.counts` | + | F | F | F | F | F | F | F | shared (`--counts`) |
| `mailbox.role` | + | D | F | F | F | - | - | F | shared (pimdir, JMAP) |
| `mailbox.create` | ✓ (`collection.create`) | F | F | F | F | F | F | F | proto |
| `mailbox.rename` | + | F | F | F | F | F (m2dir -) | F | - | proto |
| `mailbox.delete` | + | F | F | F | P | F | F | - | proto |
| `mailbox.move` | + | F | F | F | F | - | - | - | proto |

- `mailbox.role`: IMAP reads roles only with SPECIAL-USE (cairn/changes/imap-special-use-aliases); Maildir and mbox have no roles, only the config aliases.
- `mailbox.delete`: Graph deletes the folder and everything in it, where IMAP servers may refuse a mailbox with children; the detail says which.
- `mailbox.move` (settled 2026-11-02): JMAP reparents through `jmap mailbox update --parent-id`, Graph through `mail-folders move`. IMAP and Gmail hold the hierarchy in the name, so `imap rename` to a name under another parent (RFC 3501 §6.3.5) and a Gmail label renamed with another `/` prefix move it. From the user's side it is one thing, a mailbox given a new place; underneath, a rename where the path is the name and a reparent where the parent is an id. Whether the shared verb is `mailbox move` or a path-taking `mailbox rename` is the shared command's design, the capability being the same.

### Reading mail

| Capability | Name | IMAP | JMAP | Gmail | Graph | Maildir | mbox | pimdir | Today |
|---|---|---|---|---|---|---|---|---|---|
| `envelope.list` | + | F | F | F | F | F | F | F | shared |
| `envelope.search` | + | F | F | P | P | F | F | F | shared |
| `envelope.count` | + | - | - | - | - | - | - | F | proto (pimdir) |
| `envelope.sort` | + | D | - | - | - | - | - | F | proto (IMAP, pimdir) |
| `thread` | + | D | F | F | - | - | - | F | proto |
| `message.read` | + | F | F | F | F | F | F | F | shared |
| `attachment.list` | + | F | F | F | F | F | F | F | shared |
| `flags.allowed` | + | F | - | - | - | F | F | - | proto |

- `envelope.search` (settled 2026-11-02, from src/gmail/search.rs and src/msgraph/search.rs): both translate every clause of the shared language (`and`, `or`, `not`, `date`, `after`, `from`, `to`, `subject`, `body`, `flag`) but refuse some flag clauses and sort only the fetched page.
  - Gmail (`q`): `flag answered` and any keyword flag are refused, Gmail having no operator for them (seen, flagged, draft, important and junk map to `is:` and `in:spam`). `body` becomes a bare word, which Gmail matches over the whole message, headers included. Sorting is client-side over the page, `messages.list` answering newest first.
  - Graph (KQL `$search`): every flag clause is refused, KQL having no flag property and Graph taking no `$filter` beside `$search`. Double quotes are dropped from patterns. Graph refuses `$orderby` and `$skip` beside `$search`, so a page is reached through the paging links, comes in relevance order and is sorted locally.
  - So `partial`, the detail naming the refused clauses and the page-local sort. Every refusal is already an error, never a widened result. The search spec plans the pimdir SEARCH language as the shared query before v3 (cairn/spec/search.md), which will redo this row.
- `envelope.sort`: IMAP SORT is an extension (RFC 5256), with `sort.fallback` sorting client-side.
- `thread`: IMAP THREAD is an extension (RFC 5256); Gmail and JMAP have thread objects; pimdir answers from its index.
- `flags.allowed` is the flags a mailbox accepts (IMAP `PERMANENTFLAGS`). `pimdir flag list` is another thing, the flags one message carries, which the envelope already holds: one name, one meaning, so it does not register this.

### Writing mail

| Capability | Name | IMAP | JMAP | Gmail | Graph | Maildir | mbox | pimdir | Today |
|---|---|---|---|---|---|---|---|---|---|
| `message.add` | ✓ | F | F | F | - | F | F | F | shared |
| `message.copy` | ✓ | F | P | P | F | F | F | F | shared |
| `message.move` | ✓ | D | F | F | F | F | F | F | shared |
| `message.remove` | ✓ | F | F | F | F | F | F | F | shared (via `message delete`) |
| `message.trash` | + | P | F | F | F | P | P | F | shared (`message delete`) |
| `flags.seen` | ✓ | F | F | F | F | F | F | F | shared |
| `flags.flagged` | ✓ | F | F | F | F | F | F | F | shared |
| `flags.answered` | ✓ | F | F | none | none | F | F | F | shared |
| `flags.draft` | ✓ | F | F | none | none | F | F | F | shared |
| `flags.keywords` | ✓ | D | F | P | P | P | F | F | shared |
| `flags.important` | + | D | F | F | F | P | P | F | shared |
| `flags.junk` | + | D | F | F | - | P | P | F | shared |

- `message.add`: Graph's `messages create` makes a draft, not a stored message, so the shared `message add` refuses it; Gmail's add is `insert` (no filters, no spam scan), `import` being another meaning that stays proto.
- `message.copy`: on JMAP (a mailbox added to `mailboxIds`) and Gmail (a label added) the "copy" is the same message in two mailboxes, flags and deletion shared, where IMAP, Graph and the local backends make an independent copy. That is a different observable result, so the registry declares them `partial` with that detail, or splits the name. `jmap email copy` (`Email/copy`) copies across accounts, a third meaning, and stays proto.
- `message.move`: IMAP MOVE is an extension (RFC 6851); without it the copy, flag and expunge fallback gives the same result, so `full` either way once the fallback is accounted for.
- `message.trash` needs the trash role: native on Gmail, Graph, JMAP and pimdir; on IMAP it needs SPECIAL-USE or an alias, locally an alias.
- `flags.answered` on Gmail and Graph is `none`: neither has a field for it. **Today the shared `flag add answered` there silently does nothing and reports every id found** (src/gmail/backend.rs, src/msgraph/backend.rs, "leaves nothing to patch"). That is a success with a second meaning; under the registry it fails with `capability-missing`.
- `flags.draft` (settled 2026-11-02): `none` on both. Graph's `isDraft` is read-only. Gmail applies `DRAFT` itself, and its labels guide lists `DRAFT` and `SENT` as labels that cannot be applied manually (<https://developers.google.com/workspace/gmail/api/guides/labels>); the adapter still maps `\Draft` onto `DRAFT` (src/gmail/backend.rs `label_of`), so a shared `flag add draft` reaches Gmail and fails there rather than up front. A draft is made by the draft verbs instead. The same guide says no label can be applied to a draft message, which the `flags.keywords` detail carries for Gmail.
- `flags.keywords`: Gmail takes existing label ids only and none on a draft message, Graph maps them to categories, Maildir to the dovecot-keywords sidecar; IMAP accepts them when `PERMANENTFLAGS` holds `\*`.
- `flags.important` (RFC 8457 `$Important`) and `flags.junk` (`$Junk`) are mapped by the Gmail and Graph adapters already and have no Annex B name yet; locally and on IMAP they are keywords, hence `partial` and `PERMANENTFLAGS`-bound.

### Sending

| Capability | Name | SMTP | JMAP | Gmail | Graph | pimdir | Today |
|---|---|---|---|---|---|---|---|
| `submit` | ✓ | F | F | F | F | F | shared |
| `submit.copy` | ✓ | P | F | F | F | F | shared (`save-copy`) |
| `draft.save` | + | - | - | F | F | - | proto |
| `draft.send` | + | - | F | F | F | - | proto |
| `submit.cancel` | + | - | F | - | - | F | proto |

- `submit.copy` over SMTP is himalaya filing the copy into the storage backend after the send, two operations rather than one: `partial`, as pimdir's own rule for a performer without server-side filing says. Gmail and Graph file every sent message whether asked or not, which the detail says.
- `draft.*`: IMAP plus SMTP could do it as add with `\Draft`, read, submit and remove, not atomically; left `-` until a shared verb exists.
- `submit.cancel`: JMAP cancels a pending `EmailSubmission`; pimdir cancels a queued `submit` (`pimdir queue cancel`), the same result for a send not yet gone.

### Account features

| Capability | Name | JMAP | Gmail | Graph | Sieve | Today |
|---|---|---|---|---|---|---|
| `vacation` | + | F | F | - | none | proto |
| `identity.list` | + | F | F | - | - | proto |

- `vacation`: Graph's `mailboxSettings` is not in io-msgraph yet; Sieve is `none` with the detail the proposal gives (no date window, owns the active script).
- `identity.list`: JMAP identities and Gmail send-as aliases; the config's aliases cover every backend client-side, which is a different source of truth.

### Staying protocol-specific

No second backend gives the same result, or the operation is the protocol itself: IMAP `id`, `select`, `unselect`, `close`, `status`, `fetch`, `expunge` (removing what `\Deleted` marks is IMAP's own model, every other backend removing at once), `subscribe`, `unsubscribe`, `raw`; SMTP `raw`; every Sieve script verb (one protocol, so nothing to unify; a shared filter verb would need Gmail filters and Sieve to mean one thing, which they do not); JMAP `query`, `email parse`, `email export`, identity writes, submission reads; Gmail `profile`, `history`, the settings (IMAP and POP access, language, auto-forwarding, filters, forwarding addresses, delegates, send-as writes), `untrash`, batch verbs; Graph `profile`, `mail-folders copy`, `messages update`, attachment writes; pimdir `queue`, `performer`, `attachment search`, `tag`; the local backends' `flag list`.

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
