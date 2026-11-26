---
cairn: spec
capability: testing
status: current
---

# Microsoft Graph on Outlook: unified-command test report

Unified commands on the Microsoft Graph backend. Companion: [msgraph-specific.md](msgraph-specific.md) (the `msgraph …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside throwaway mail folders.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`, `--version` v3.0.0
- account: `microsoft` (Microsoft Graph, `msgraph.auth.token.command`), the Exchange Online test tenant, which holds a fake corpus kept untouched; its folders list under their English names (`Inbox`, `Sent Items`, `Deleted Items`) although the user's preferred language is French
- date: 2026-11-26
- fixtures: folders `live-2026-11-26-himalaya-A` and `live-2026-11-26-himalaya-B` (made with `mailbox create`); Graph has no MIME append, so six drafts with subjects starting `live-2026-11-26-himalaya` (senders alice, bob, carol, dave, eve; dates 2026-11-10 to 2026-11-26; one `multipart/mixed` with `note.txt`) were made with `msgraph messages create --folder drafts` and moved into `A` with `message move -f drafts`. The sends of the run (two unified, two raw) were moved from Inbox and Sent Items into `A`, then both folders were deleted with `mailbox delete`. The account listed 12 folders before and after, every count identical (Inbox 85/1307, Sent Items 0/453, Deleted Items 40/40, Drafts 0/0); `envelope search subject:live-2026-11-26-himalaya` finds nothing in Inbox, Sent Items, Drafts and Deleted Items.
- live suite: not run in this pass (the lead runs the cargo live suites).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ `ok: true`, `tls` all `null` |
| `mailbox list` | `--counts`, `--json` with and without `--counts` | ✅ `total` and `unread` `null` without `--counts` |
| `mailbox create` | new folder, existing name (text, `--json`) | ✅ existing refused `mailbox-duplicate`, Graph's 409 `ErrorFolderExists` in `sources`, exit 1 |
| `mailbox rename` / `delete` | rename and back, missing folder, delete, delete again | ✅ missing folder `mailbox-not-found`, exit 1 |
| `envelope list` | by name, `-s 2 -p 2`, `-s 2 --page-token 2`, `-r`, `--has-attachment`, empty folder (text, `--json`), no `-m`, role `inbox`, missing folder, `-p 0` | ✅ `-p 0` refused by clap, exit 2; missing folder exit 1 |
| `envelope search` | `from:alice`, `to:carol`, `with:carol`, `cc:carol`, `bcc:carol`, `subject:report`, `subject:"weekly report"`, `body:invoice`, bare `kiwis`, `has:attachment`, `mid:live-2026-11-26-m3@live.himalaya.test`, `nosuchwordxyz` | ✅ |
| `envelope search` | `date:2026-11-24`, `date:2026-11-24..`, `date:..2026-11-19`, `date:2026-11-19..2026-11-24`, `date:today`, `date:yesterday`, `date:2026-11` | ✅ compares the `Date:` header (KQL `sent`), bounds inclusive |
| `envelope search` | `from:alice has:attachment`, `from:alice or from:bob`, `not from:alice`, `-from:alice`, `from:alice (subject:alpha or has:attachment)`, `subject:report or subject:lunch`, no query | ✅ |
| `envelope search` | `--sort newest-first`, `oldest-first`, `title`, `sender`; `-s 2 -p 2`, `-s 2 --page-token 2`, `--sort oldest-first -s 2 -p 2`; `-r --has-attachment --sort sender` | ✅ each page sorted on its own, as the search spec says |
| `envelope search` | `date:5h`, `date:2d`, `date:1w`, `is:read`, `is:unread`, `is:flagged`, `is:answered`, `is:draft`, `tag:foo`, `mid:<id> from:carol`, `mid:<id> or from:alice`, `--sort relevance` | ✅ refused `search-unsupported`, exit 1 (but see M6 for `date:2d`) |
| `envelope search` | `(from:alice`, `~draft`, `"~draft"` | ✅ the first two refused `search-invalid` (`unbalanced-parenthesis`, `reserved-form`), exit 1; the quoted one searched, no match |
| `envelope search` | `subject:repo*`, `subject:quart*`, `quart*`, `subject:"report weekly"` | ❌ the prefix is dropped: nothing found, exit 0; the phrase matches out of order (M5) |
| `message add` | file | ✅ refused `capability-missing` (Graph creates drafts only) |
| `message read` | pretty, multipart, `--raw`, `--json`, malformed id `999`, id outside `-m`, an unseen message stays unseen, `--seen` | ✅ `message-not-found` exit 1 (Graph's 400 in `sources` for `999`) |
| `flag add` / `set` / `remove` | `seen` + `flagged`, `important` on two ids, `set flagged` replacing `seen` and `important`, remove two flags on two ids, malformed id beside a real one | ✅ the malformed id an `errors` entry `message-not-found`, the other flagged, exit 3 |
| `flag add` / `set` | `answered`, `draft`, `junk`, no `-f`, `-f bogus` | ✅ `capability-missing` exit 1 (`isDraft` read-only, no junk mark); clap refusals exit 2 |
| `message copy` / `move` | single, multi-id, move, malformed id beside a real one, missing `--to`, missing target folder, no `-f` | ✅ missing target `mailbox-not-found` (M4 fixed); missing `--to` `usage-invalid` exit 2; no `-f` reads Inbox, an id elsewhere `message-not-found`, exit 3 |
| `message delete` | in a folder, then in the trash | ✅ `action: moved-to-trash`, then `deleted` |
| `attachment list` / `download` | list, `--inline`, none, `-d`, `--json -d` on a taken name, `--stdout <id>`, unknown attachment id | ✅ file 0600, taken name written `note (1).txt`; unknown id uncoded (observation) |
| `message compose` | `--save` without `--send` | ✅ refused `capability-missing` |
| `message send` | `--no-save` | ✅ refused `capability-missing` before connecting |
| `message send` | `--save` of a missing folder, to self | ✅ `saved: true`, `sent: true`, `id: null`, delivered and filed in Sent Items |
| `message reply` / `forward` | printed: `-P top -Q "Quoted:"`, no `-m`, source outside `-m` | ✅ `Re:` and `Fwd:` subjects, `In-Reply-To` and `References`, quoted body; outside `-m` `message-not-found`, exit 1 |
| `message reply --send` | to self | ✅ delivered, `In-Reply-To` kept |

## Findings

### Bugs

- **M5: a prefix word and a phrase lose their meaning on Graph.** `himalaya -a microsoft --json envelope search -m live-2026-11-26-himalaya-A subject:repo*` prints `{"envelopes":[],"nextPage":null,"total":null,"errors":[]}`, exit 0, while `subject:report` finds two messages; `subject:quart*` and `quart*` likewise find nothing. `subject:"report weekly"` finds `beta weekly report`, the phrase matched as two words in any order. src/unified/email/search/query.rs `lower` keeps `PimdirQueryText::text` and drops its `prefix` and `phrase` fields, and src/msgraph/search.rs `value_kql` quotes each word apart, so `repo*` reaches KQL as the word `repo`. The search help says "A trailing `*` makes a word a prefix"; KQL serves both a prefix (`repo*`) and a phrase, so Graph can answer them, or else refuse them `search-unsupported` rather than answer wrong. The lowering is shared, so the IMAP, JMAP and Gmail paths likely lose the same fields (not checked here).
- **M4 (2026-11-18) is fixed:** a missing folder is now coded, `message move -f live-2026-11-26-himalaya-A -t live-2026-11-26-himalaya-nope <id>` printing `{"code":"mailbox-not-found",…}`, exit 1; so do `envelope list`, `mailbox rename` and `mailbox delete`.

### Gaps

- **M6: the migration guide's `date:2d` is refused on Graph.** v2's `after 2 days ago` and `after 1 week ago` were served on Graph (2026-11-18 run). MIGRATION.md maps them to `date:2d` and `date:1w`, which v3 resolves to an instant, so Graph refuses them: `envelope search -m live-2026-11-26-himalaya-A date:2d` prints ``{"code":"search-unsupported","error":"Cannot search `date:2026-11-24T14:19:00+00:00..`: Microsoft Graph search compares days alone; give a day, or `today`",…}``, exit 1. That follows the search spec (an amount names an instant), but the guide does not say that Graph users need a day (`date:2026-11-24..`). The two instant refusals are also worded apart: `date:5h` reads ``Cannot search `date.instant` over msgraph: KQL compares days``.

### Behaviour (not bugs)

- Graph files every sent message in Sent Items itself: `--save` is ignored on a send and the receipt says `saved: true` with a `null` id; `--no-save` is refused before connecting.
- Instants (`date:5h`, `date:2d`) are refused: KQL `sent` compares days, as the search spec says. A day, a month, a range of days, `today` and `yesterday` are served.
- `envelope list` orders by Graph's `receivedDateTime`, which for the moved drafts is when they were created, so the Date column reads 2026-11-26, 2026-11-10, 2026-11-23, …; received mail lists newest first as expected.
- A paged search sorts each page apart: `-s 2` gives `zeta, epsilon` then `delta, gamma`, while the unpaged search is in date order; `--sort` orders the fetched page only, as the search spec says.
- Envelope rows carry `inReplyTo: []` on Graph (backends spec: `internetMessageHeaders` is not in a listing); `message read --json` reads it from the raw message.
- Moved drafts keep `\Draft` (Graph's `isDraft` is read-only), so every fixture listed `\Draft`.
- `envelope list -r` and the `To` column show the account owner's directory name (`CLEMENT ROLAND RAYMOND DOUIN`) for its own address, not the `Microsoft` the message carries; `message read` shows the header's name.

### Observations

- `attachment download -m live-2026-11-26-himalaya-A <id> 9` prints ``{"code":null,"error":"No attachment with id 9 on message `…`",…}``, exit 1: an unknown part id is uncoded on every backend (src/unified/attachment/download.rs); the code table has no row for it.
- `mailbox delete live-2026-11-26-himalaya-B` prints ``Successfully deleted mailbox `AAMk…dHXsAAA=` ``, the folder's id rather than the name given; `--json` prints `{id, queued}`.
- The test rc gives the account no `email`, so the composers print no `From:` (`--json message compose` gives `from: null`) and Graph fills it on send.
- A search for the marker matched across the whole subject (`subject:live-2026-11-26-himalaya`), KQL splitting it into words.

## Verdict

Unified commands on Graph are **solid** for the subset Graph serves: list, search (KQL text, days and day ranges, `has:attachment`, `mid:`, the combinators, the four sorts), read, flags, copy, move, delete, attachments, send and reply all pass, the unsupported cells are refused with a code before any write, and the 2026-11-18 gap (M4) is fixed. One bug: a prefix word and a phrase are silently lowered to plain words (M5), giving an empty or loose answer with exit 0. One documentation gap: the migration guide's `date:2d` is refused on Graph (M6).
