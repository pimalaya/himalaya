# IMAP on iCloud — test report

Shared cross-protocol commands (and IMAP-specific spot-checks) on the
IMAP backend against **iCloud** (`imaps://imap.mail.me.com`, app-specific
password). The account is **IMAP-only** — no SMTP block is configured —
so `compose`/`send`/`reply`/`forward` were out of scope for this run.

- himalaya: 2.0.0-alpha.1 (rev fcf5b76)
- account: icloud (`imap`)
- date: 2026-07-19

All work happened inside two throwaway mailboxes (`Himalaya-Test-A`,
`Himalaya-Test-B`) created for the run and deleted at the end; real mail
(Inbox, Archive, Sent Messages) was never touched.

## Results

| Area | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base, `--counts`, `--max-width`, `--json` | pass |
| `envelope list` | base, paging, `-r`, `--has-attachment`, `-w`, `--json`, empty mbox | pass |
| `envelope list` (default) | no `-m` | pass (clear error: no `mailbox.alias.inbox`) |
| `envelope search` | `from/to/subject/body/flag/date/after` | pass |
| `envelope search` | `and/or/not`, `( … )` grouping | pass (see grouping note) |
| `envelope search` | `order by …` | **FAIL** — sorting ignored (see Bugs) |
| `message add` | inline, file, stdin, flags, `--json`, errors | pass |
| `message read` | pretty, multipart, `--raw`, `--json`, error, PEEK | pass |
| `flag add/set/remove` | multi-flag, multi-id, set-replaces, errors | pass |
| `message copy/move` | single, multi, move, no-`--to`, non-existent id | pass |
| `attachment list/download` | multipart, none, download, `--json` | pass |
| `imap` specific | `id`, `fetch`, `search`, `store` (keyword), `create/delete` | pass |
| `imap thread` | `-A references` | fail by design (no THREAD capability) |
| SMTP (`compose/send/reply/forward`) | — | not tested (no SMTP configured) |

## Findings

### Bugs / issues

- **`order by` / `imap sort` is silently ignored — iCloud's server SORT is
  a no-op (provider bug).** iCloud advertises `SORT ESORT CONTEXT=SORT`,
  so himalaya's default policy (`sort_fallback = !has(SORT)`) trusts it and
  issues a server `SORT`. The wire trace shows himalaya sending a correct
  `UID SORT (REVERSE DATE) UTF-8 ALL` and iCloud replying `* SORT 1 2 3 4 5
  6` — i.e. plain UID-ascending order, **ignoring both the key and
  REVERSE** (`-S subject` likewise came back in UID order, not
  alphabetical). himalaya relays the server order faithfully; the fault is
  iCloud's. Every sort key/direction therefore returns the same UID order.

- **The client-side sort fallback mis-ordered the `date` key (io-imap
  bug — now FIXED).** Under `imap.sort.fallback = true` himalaya sorts
  locally (SEARCH + FETCH + sort). Subject ordering was already correct,
  but **date** ordering came back by **weekday name, lexically** (`Fri,
  Mon, Sat, Thu, Tue, Wed`): `io_imap::rfc5256::sort::cmp_fetch_items`
  compared the **raw `Date:` header** (`envelope.date`, a byte string)
  rather than a parsed instant. Fixed by parsing the header through
  `chrono::DateTime::parse_from_rfc2822` (new `date_sort_key` helper;
  unparsable/absent dates sort first). Arrival was never affected — its
  `InternalDate` is already a `chrono::DateTime`. Verified against iCloud
  after the fix: `order by date asc` → chronological `1 2 3 4 5 6`, `desc`
  → `6 5 4 3 2 1`, subject unchanged. The default `envelope list` was
  always correct (it parses); this only touched the search/sort fallback.

### Provider-specific behaviour (not bugs)

- **No MOVE capability.** `message move` transparently uses the COPY +
  `\Deleted` + EXPUNGE fallback; the move succeeded and the source UID was
  gone afterwards, with an accurate `1 message successfully moved`.
- **No THREAD capability.** `imap thread` fails with `BAD Parse Error`
  (iCloud rejects the command) — expected, not a himalaya fault.
- **Custom keywords are accepted.** Raw `imap store <id> --action add -f
  keywordtest` succeeded and the keyword showed up in `fetch --flags`. (The
  *shared* `flag` command still rejects non-standard flags at the CLI enum,
  by design.)
- **Default mailbox** requires an explicit `-m` or a configured
  `mailbox.alias.inbox`; omitting it gives a clear actionable error rather
  than assuming a literal `INBOX`.
- **`date` search targets the `Date:` header** (SENTON), not arrival: a
  message appended today but dated 2026-07-15 matched `date 2026-07-15`.
- **`after`** is strict — `after 2026-07-16` excluded the message dated
  2026-07-16 (matched only later ones).
- **`envelope search` grouping requires glued parens.** `(from a or from
  b)` parses; `( from a or from b )` (a bare `(` argv token, which himalaya
  space-joins) fails with `expected … (nested filter)`. Minor: the DSL
  doesn't tolerate whitespace immediately inside parens. Grouping itself
  works (glued form excluded the non-matching message correctly).

### Observations

- Flag rendering: `*` unseen, `!` flagged, `R` answered; `\Draft`/`\Seen`
  set/remove verified via the FLAGS column. `\Draft` has no dedicated
  symbol.
- `message read` on an unseen message is non-destructive (PEEK): the
  message stayed unseen.
- Counts are accurate: `mailbox list --counts` matched, and a
  copy of a non-existent id reported `No message copied: no id matched`
  (UIDPLUS present, absent COPYUID → 0, message untouched).
- Attachment round-trip is byte-correct (list → download → file content).
- `imap id` returns the iCloud Mail server identification.

## Verdict

The IMAP backend is **functionally solid on iCloud** for the whole shared
surface — list, search (conditions/combinators/grouping), add, read,
flags, copy, the MOVE-less move fallback, and attachments all behave
correctly. **Sorting** needed one config knob plus one code fix:

1. iCloud's server SORT is a no-op, so users must set `imap.sort.fallback
   = true` to sort at all. Consider defaulting iCloud (or any host whose
   SORT can't be trusted) to the fallback, or at least documenting it.
2. The io-imap fallback date-sort bug is **fixed** (chronological
   `Date:`-header parse); date ordering now works under the fallback.

With `imap.sort.fallback = true`, iCloud is release-ready on the IMAP
shared surface. SMTP was not exercised (not configured);
compose/send/reply/forward remain untested on iCloud.
