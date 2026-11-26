# Migration guide

## From v2 to v3

v3 makes Himalaya a low-level CLI: plain text for a human to glance at, `--json` for scripts and frontends, daily mail being a frontend's job (see the interfaces listed in README.md). A v2 configuration loads as it is unless it sets a key v3 no longer knows or marks several accounts default. The [CHANGELOG](./CHANGELOG.md) lists every change; these are the ones to act on.

### Configuration

- **The wizard is gone**, with `himalaya configure` and the `wizard` cargo feature: bare `himalaya` prints the help. Start from [config.sample.toml](./config.sample.toml), which packages ship as share/doc/himalaya/config.sample.toml.
- **`-c` takes one path per flag**, no longer split on `:`: write `-c public.toml -c private.toml`.
- **`HIMALAYA_CONFIG` is no longer read**: pass `-c "$HIMALAYA_CONFIG"`.
- **Every default path is read**, each deep-merged on top of the previous one, where v2 read the first found: delete the file you do not use.
- **Several `default = true` accounts are refused**: keep it on one.
- **`-a default` names the account called `default`**: omit `-a` to take the account marked default.
- **An unknown key fails the configuration**, naming it and its line: fix or delete it. The v1 `backend` table and the rest of a v1 `[message]` table (`read`, `write`, `send.backend`) are refused too.
- **The colour, table and glyph keys are gone**: delete the `table` block, every `*.list.table` block and every `*-color` key. Rows mark a message with fixed characters (`*` unread, `R` answered, `!` flagged, `@` attachment).
- **A v1 `folder` table is refused**: move `folder.alias` to `mailbox.alias`.
- **`envelope.list.datetime-relative` is gone**: delete it.
- **Dates default to `%F %R` in the local timezone**: set `envelope.list.datetime-fmt = "%F %R%:z"` and `envelope.list.datetime-local-tz = false` to print them as v2 did.
- **himalaya-tui's keys go under `[tui]`**: move `keybinds`, `contact-command`, `contact-complete-key` and `theme` to a top-level `[tui]` table, an account's `contact-command` to `[accounts.<name>.tui]`.
- **A JMAP `server` or an HTTP proxy `url` takes an ASCII host only**: write an internationalised host in Punycode (`https://xn--bcher-kva.example` for `bücher.example`).
- **m2dir is gone**, with `himalaya m2dir`, `-b m2dir` and the `m2dir` cargo feature: move the account to `maildir.root` or `pimdir.root`, sync it from the server, then delete the `m2dir` block.

### Text output

- **The text output changed**: lists print one item per line, in columns with no header row, border, colour or truncation, under an `Account:` header and above one `Error:` line per item not served; records print `Label: value` lines. A script parsing text moves to `--json`:

  ```sh
  # id, sender and subject, one envelope per line
  himalaya envelope list --json | jq -r '.envelopes[] | [.id, .from[0].email, .subject] | @tsv'
  ```

- **`-w/--max-width` is gone**, nothing being truncated: drop it.

### JSON output

- **Keys are camelCase**: `messagesTotal`, `nextPage`, `threadId`. Values (error codes, flags, roles) keep their spelling, and provider payloads keep the provider's keys.
- **No output is a sentence**: the `{"message"}` confirmations are gone, every write printing what it acted on. Check the exit status, or read the receipt.
- **The message receipts**: `message send|add` and a composer's `--save`/`--send` print `{id, saved, sent, queued, errors}`, `message copy|move` `{count, queued}`, `message delete` `{action, count, ids, queued, errors}`, a protocol write `{id}` or `{ids}`.
- **Read `queued` before `sent`**: a send a sync engine applies later (pimdir) prints `"sent":false`.
- **`message read --json` is the envelope row plus a view** (`headers`, `text`, `html`, `parts`) the same on every backend, no longer mail-parser's model. `message parse` prints the view alone.
- **Bytes print as `{data, size, mime}`**, `data` in base64: `message read --raw`, `jmap email export`, the raw Gmail and Graph `get`s and their attachment downloads.
- **Absent values are `null`**, every key printed: a missing subject is `null` instead of `""`, an unknown `size` `null` instead of `0` (always on Graph).
- **Shapes are made one**: flags print `{raw, iana}` (maildir and mbox `flag list` too), addresses `{name, email}` (`imap fetch`, a composer's template), `inReplyTo` a list, `imap search` ids numbers, an attachment row `role` instead of `inline`.
- **A UTC instant ends with `Z`** (`imap fetch --internal-date`, `headers.date`) instead of `+00:00`: parse it as RFC 3339.
- **Listings print `nextPage`**, `null` on the last page, which `--page-token` takes back: `envelope list|search`, Graph's next link as is, Gmail's `nextPageToken`, the JMAP page number.
- **`--page-token` beside `--page` exits 2**: pass one. A token no listing printed is `page-token-invalid`.
- **`account check` prints `{"accounts":[...]}` and exits 3** when a backend fails: `.backends[]` becomes `.accounts[0].backends[]`. A cleartext row's `tls` is `null`.
- **`account list` puts the backends a build leaves out under `notBuilt`**, no longer under `backends`.
- **`maildir create|rename|delete` print the folder's path as `id`**, the id the `mailbox` commands take; `maildir list` rows gain `id`.

### Protocol output

- **Every `imap` command prints `status`**: `OK`, or `NO`/`BAD` with `text` and exit 3. `imap select` prints `firstUnseen` and `highestModSeq`; `imap status` drops `deleted`, `deletedStorage` and `highestModSeq`.
- **`imap raw` and `sieve raw` print the responses as events**, one JSON line each, then the completion; `smtp raw` prints `{code, text}`.
- **A non-UTF-8 `imap raw` response prints `{"event":"undecoded","base64":"..."}`**, the exact octets, instead of text with U+FFFD: decode `base64`.
- **Protocol listings and gets print the provider's payload**, every property kept, the cursor alone lifted into `nextPage`. `msgraph profile get` prints Graph's whole user.
- **Gmail keys are Gmail's**: `messages` (not `ids`), `emailAddress` (not `email`), `delegates`, `filter`, `forwardingAddresses` (not `response`), headers under `payload.headers`. With jq, `gmail history list` arrivals are `.history[].messagesAdded[].message.id`.
- **`gmail labels get` and `msgraph mail-folders get` take one id** and print the object: run one per id, `.labels[0]` or `.folders[0]` becoming `.`.
- **`sieve check|put` print `warnings` as a list**, `[]` when none, instead of a string or `null`.
- **`jmap email parse` prints `bodies` as `[{blobId, body}]`**, instead of a list of strings.

### Exit codes and error codes

- **A failure carries a stable `code`** when a caller can act on it, described by the `himalaya-error` schema: match on it rather than on `error`.
- **A refused command line exits 2 coded `usage-invalid`**, printed on stdout under `--json`. A bare `himalaya --json` is one, and so is a value a protocol command refuses before connecting (`imap raw -- NOOP` without a tag).
- **A server's refusal exits 3**: `{"refusal":{status, reason, message, code, retryAfter}}` on Gmail, Graph and JMAP, `{status, code, text}` on `sieve`, `{"refusals":[...]}` on `smtp send`. Exit 1 now means no answer (network, TLS, a local error).
- **A partial outcome exits 3** with one `errors` entry per item: a JMAP set or get applied in part or naming ids not found, `jmap email parse`, `envelope list|search` over a message holding no header (`message-unreadable`).
- **A failure after a send exits 3**, the copy or the reply's answered flag listed under `errors`: do not send again, `sent` says the message went out.
- **A missing mailbox is `mailbox-not-found` on every backend**, where IMAP, JMAP, Gmail and Graph printed `code: null`.
- **A Gmail daily quota is `rate-limited`**, no longer `permission-denied`: wait for the quota to reset.

### Commands that now fail

- **An unserved verb fails with `capability-missing`**: `flag add|set|remove answered|draft` on Gmail and Graph, `message add` and `--save` without `--send` on Graph, a send given `--no-save` on Graph (drop the flag).
- **Creating a mailbox that exists fails `mailbox-duplicate`**: a script creating it if missing treats that code as success.
- **An id the mailbox does not hold exits 3**, named under `errors` coded `message-not-found` while the other ids are acted on: `flag add|set|remove`, `message move|copy|delete`.
- **A search clause a backend cannot answer fails `search-unsupported`**, rather than being dropped; a query that does not parse fails `search-invalid`.
- **An IMAP keyword the mailbox's `PERMANENTFLAGS` refuses fails `capability-missing`**, rather than being lost with the session.
- **`-f/--flag` takes one value per occurrence**: `-f seen -f flagged msg.eml`.
- **A message argument must be a file or a message**: `message add|send` refuse a mistyped path rather than storing it.
- **`maildir message add -f` needs `--subdir cur`**, and `--subdir tmp` is gone: pass `cur` or `new`.
- **`maildir flag set` and `mbox flag set` need `-f`**: clear flags with `flag remove`, naming them.
- **mbox `INBOX` needs `mbox.inbox`** unless the root holds an `INBOX` file: set the spool, or pass the mailbox by name.
- **`--page 0` is refused**: pass `--page-size 0` for the whole mailbox.
- **`--no-save` needs `--send`** on `message compose|reply|forward`.
- **A JMAP, Gmail or Graph read is scoped to `-m`**: `message read|reply|forward` and `attachment list|download` fail `message-not-found` elsewhere, so pass the mailbox (`-m sent` for a sent message).
- **`message read --seen` on pimdir fails** when the flag cannot be staged, carrying the cause's code.

### Search

`envelope search` reads the pimdir query language, and its order is `--sort`. Rewrite every v2 query:

| v2 | v3 |
| --- | --- |
| `from alice` | `from:alice` |
| `to bob`, `cc carol`, `bcc dave` | `to:bob`, `cc:carol`, `bcc:dave` |
| `address eve` | `with:eve` |
| `subject "weekly report"` | `subject:"weekly report"` |
| `body invoice` | `body:invoice`, or `invoice` for the whole message |
| `flag seen`, `not flag seen` | `is:read`, `is:unread` |
| `flag flagged`, `flag answered`, `flag draft` | `is:flagged`, `is:answered`, `is:draft` |
| `flag <keyword>` | `tag:<keyword>` |
| `attachment` | `has:attachment` |
| `message-id <id>` | `mid:<id>` |
| `date 2026-11-14` | `date:2026-11-14` |
| `after 2026-11-14` | `date:2026-11-15..` (`after` was strict) |
| `before 2026-11-14` | `date:..2026-11-13` (`before` was strict) |
| `after 2 days ago`, `after 5 hours ago` | `date:2d`, `date:5h` |
| `a and b`, `a or b`, `not a` | `a b`, `a or b`, `not a` or `-a` |
| `order by date desc` | `--sort newest-first` |
| `order by date asc`, `order by date` | `--sort oldest-first` |
| `order by subject` | `--sort title` |
| `order by from` | `--sort sender` |
| `order by to`, a descending subject or sender | none |

- **Text matches whole words** on Maildir, mbox and pimdir, where v2 matched a substring: `subject:repo*` finds `report`.
- **The capability options take the predicate names**: `envelope.search.message-id` is `envelope.search.mid`, `envelope.search.sort.date` is `sort.newest-first` and `sort.oldest-first`.
- **`pimdir search --sort date|name` is `--sort newest-first|title`**.
- **A word starting with `~`, a word of two characters or more starting with `/` and a value starting with `{` are refused** (`reserved-form`): quote them to search the characters, `"~draft"`.

### Commands that act differently

- **`--inline` has no `-i` in the help**, `-i` meaning `--interactive` across the family: write `--inline`.
- **`attachment download` without an id skips the inline parts**: pass `--inline` to write them too.
- **An option after a pimdir query, a message, a Sieve script or a JMAP call array is the option**: put a word starting with a hyphen after `--` (`pimdir search -- -from:alice`).
- **The root Maildir is named `INBOX`** (`.` when a top-level `Inbox` folder exists) instead of its folder's basename: match `INBOX` or its id.
- **`maildir message save` and `mbox message save` are `message add`**, the old names still working: a JSON schema read by its name moves to `himalaya-maildir-message-add` or `himalaya-mbox-message-add`.
- **`maildir message copy|move` and `mbox message copy|move` print the target ids**, where they echoed the source ones: read a moved message by the printed id.
- **A pimdir store an earlier release wrote is refused** with `store-version` (its index `index-version`), Himalaya reading pimdir v1 alone: sync it with the neverest that wrote it until `pimdir queue list` lists nothing, then move its directory aside and let a v1 neverest sync a new store.

### pimdir queue

- **A queue row is `row` and `handle`**, where v2 printed `queueId`: `pimdir queue show|cancel` take the handle, a write's copy prints `copyRow` and `copyHandle`.
- **pimdir writes print their rows**: `pimdir flag add|set|remove` print `{messages: [{id, row, handle}]}`, `pimdir mailbox create|rename|delete` print `id` (`null` for a creation until synced).
- **`pimdir queue list` prints every row in one shape**, pending under `actions`, parked under `parked`: `send` is `kind` (`submit` or `add`), `envelope.subject` is `subject`. With jq, `select(.send)` becomes `select(.kind == "submit")`.
- **`pimdir queue show` prints the list row then `state`, `appliedAt` and `refusals`**: a new `unconfirmed` state, `row-not-found` for an unknown row, `store-reset` for another store's handle.
- **pimdir tag statuses are kebab-case**: `unknown-flags`, `ambiguous-source`.

## From v1 to v2

v2 rewrote Himalaya over I/O-free libraries, moving OAuth, the keyring, composition and session reuse out. Rewrite the configuration from [config.sample.toml](./config.sample.toml) beside the v1 one, and check it with `himalaya -c <new file> account check` before replacing it.

- **`-o json` is `--json`**, `--quiet`, `--debug` and `--trace` are `--log-level <LEVEL>`, and `-f/--folder` is `-m/--mailbox`.
- **`folders` is `mailboxes`**: create, expunge and delete mailboxes with the protocol commands (`himalaya imap create`...).
- **`envelope list` no longer searches**: use `envelope search`. `thread` moved to the protocol commands.
- **`flag` takes the flags as `-f/--flag`**, repeated, and the ids as positionals.
- **`message copy|move` take `--from` and `--to`** mailboxes, `save` is `add` (with a mandatory `--mailbox`), `write` is `compose`.
- **`export`, `edit` and `template` are gone**, and `compose|reply|forward` are no longer interactive: chain [pimalaya/mml](https://github.com/pimalaya/mml) into `message send` or `message add`.
- **`message read` no longer renders text**: pipe `message read --raw` into a reader. A `mailto:` handler is a shell wrapper around mml and `message send`.
- **`attachment download --downloads-dir` is `--dir`**.
- **Each backend is its own table**: `backend.type = "imap"` becomes `imap.server`, `imap.tls`, `imap.sasl.*`, the v1 send backend `smtp.*`, and Maildir `maildir.root`.
- **A secret is `{ raw = "..." }` or `{ command = "..." }`**: the keyring is gone, read it through a password manager command (`pass`, `secret-tool`).
- **OAuth is gone**: route a token from [pimalaya/ortie](https://github.com/pimalaya/ortie) through SASL `oauthbearer` or `xoauth2`.
- **The `template` and `pgp` tables are gone**: `template.reply.posting-style` and `template.forward.posting-style` are `message.reply.posting-style` and `message.forward.posting-style` (`top`, `bottom` or `none`).
- **`signature` is the text only**: pass a file with `--signature-file`.
- **`[folder.alias]` is `[mailbox.alias]`**, case-insensitive; an `inbox` alias is the default mailbox.
- **Notmuch and Sendmail are gone**.
