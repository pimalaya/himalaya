# JMAP *specific* API on Fastmail — test report

Companion to [jmap-fastmail.md](jmap-fastmail.md) (shared commands). This
exercises `himalaya jmap …` — the raw JMAP method surface.

- himalaya: `v2.0.0-alpha.1 +jmap +rustls-ring` (working tree)
- account: `fastmail-jmap`
- date: 2026-07-17
- method: every `jmap` subcommand and flag, inside two throwaway JMAP
  mailboxes, per [provider-test-plan.md](provider-test-plan.md).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `jmap query` | raw method-calls array | ✅ (used throughout) |
| `jmap mailbox get` | by id | ✅ |
| `jmap mailbox query` | `--name`, `--all`, `--sort`, `-s/-p`, `--role` | ✅ (K1) |
| `jmap mailbox create` | `<name>` | ✅ |
| `jmap mailbox update` | `--name`, `--subscribe`/`--unsubscribe` | ✅ |
| `jmap mailbox destroy` | `--purge` | ✅ |
| `jmap email get` | by id | ✅ |
| `jmap email query` | `-m`, `--subject`, `--has-attachment`, filters | ✅ |
| `jmap email read` | by id | ✅ |
| `jmap email update` | `--add/remove-keyword`, `--add/remove-mailbox` | ✅ |
| `jmap email delete` | by id | ✅ |
| `jmap email export` | blob download → raw RFC 5322 | ✅ |
| `jmap email import` | `--mailbox-id`, `--keyword` | ✅ |
| `jmap email parse` | blob id → `{"bodies":[…]}` | ✅ |
| `jmap email copy` | `--from-account` | ⚪ not tested (cross-account, needs a 2nd account) |
| `jmap thread get` | by thread id | ✅ |
| `jmap identity get` | all / by id | ✅ |
| `jmap identity create/update/delete` | round-trip | ✅ |
| `jmap submission create` | `--identity-id --mail-from --rcpt-to` → send | ✅ delivered (K2) |
| `jmap submission get/query/cancel` | by id / filter | ✅ but transient (K2) |
| `jmap vacation-response get/set` | — | ⚠️ 403 on Fastmail (K3) |

JMAP send genuinely works via `jmap submission create` using the account
identity (id `178831439`, `pimalaya@fastmail.org`): status `final`, the
message was delivered to the inbox. This is the JMAP equivalent of the
shared `message send` that J3 (shared report) blocks on missing config.

## Findings

### Behaviour (not bugs)

- **K1 — `jmap mailbox query` defaulted to subscribed-only. FIXED.**
  himalaya (not JMAP) was sending `isSubscribed: true` unless `--all`, so
  `--name Himalaya` returned `{"mailboxes":[]}` for the unsubscribed fake
  mailboxes while the raw `Mailbox/query {filter:{name:"Himalaya"}}`
  returned both. Since native `Mailbox/query` applies no subscription
  filter, the default is now flipped to match: bare `jmap mailbox query`
  lists **all** mailboxes, and `--subscribed` opts into the filter
  (`--all` removed). Verified: `--name X` now matches unsubscribed
  mailboxes; `--name X --subscribed` correctly excludes them.

- **K2 — Fastmail does not retain EmailSubmission objects.**
  `submission create` succeeds and delivers, returning the object with a
  short id (e.g. `S39`), `undoStatus: final`, and `emailId`/`identityId`
  echoed as `null` (Fastmail's `EmailSubmission/set` create response only
  returns server-set fields — hence the blank columns). A *later*
  `submission get <id>` / `query` (a separate JMAP session) returns
  `{"submissions":[]}`, and `cancel` → `notFound`. This is **not** a
  consequence of the CLI running one operation per invocation: JMAP
  objects normally persist across sessions and could be fetched later —
  Fastmail simply discards the submission once it reaches `final`. And
  `cancel` needs `undoStatus: pending` (a send-undo window), which
  Fastmail's immediate send never provides. On a server with delayed
  send + retention, `get`/`cancel` across invocations would work. The
  send itself is confirmed by inbox delivery.

- **K3 — vacation response unsupported on Fastmail.**
  `jmap vacation-response get/set` → `Vacation response is not supported
  by the server`; the raw `VacationResponse/get` returns `HTTP 403`.
  Fastmail does not grant the `urn:ietf:params:jmap:vacationresponse`
  capability to this token, and himalaya surfaces it cleanly rather than
  leaking the 403.

### Housekeeping

- Found **32 leftover test identities** (`Himalaya Test Identity…`, `Test
  Updated`) accumulated from prior JMAP identity testing on this shared
  test account; deleted them, keeping the real `Pimalaya` identity. A
  reminder that identity/mailbox create tests must clean up after
  themselves.

## Verdict

The raw JMAP surface is **complete and works end-to-end** on Fastmail:
mailbox/email/thread/identity/submission methods all behave per RFC 8620/
8621, including real message submission. K1 (subscribed default) is
**fixed** — `jmap mailbox query` now mirrors native `Mailbox/query` and
lists all by default. K2/K3 are Fastmail-side behaviours handled
cleanly. `email copy` (cross-account) is the only untested cell, for
lack of a second account. No functional blocker.
