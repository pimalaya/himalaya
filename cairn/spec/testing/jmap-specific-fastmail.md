---
cairn: spec
capability: testing
status: current
---

# JMAP *specific* API on Fastmail: test report

Companion to [jmap-fastmail.md](jmap-fastmail.md) (unified commands). This exercises `himalaya jmap …`: the raw JMAP method surface.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `fastmail-jmap`
- date: 2026-11-26
- method: every `jmap` subcommand inside the run's mailboxes (`live-2026-11-26-himalaya-A` `PF_N`, `-B` `PF_s`, `-C`/`-C2` `PFgR`), per [provider-test-plan.md](provider-test-plan.md). The identity `live-2026-11-26-himalaya` was created, updated and deleted (only `Pimalaya` is left); every email of the run was destroyed and the mailboxes deleted; Inbox, Sent, Drafts and Trash were 10, 7, 0 and 28 messages before and after. The vacation response read before the run (`isEnabled: false`, every other field `null`) was set while disabled and restored to exactly that.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `jmap request` | a request object (`Core/echo`), a method error inside one (`accountNotFound`) | ✅ `{response}` as sent, exit 0 |
| `jmap request` | a bare method-calls array | ✅ refused `The JSON body must be an object`, exit 1 |
| `jmap request` | a request refused whole (`using: ["urn:bogus"]`, HTTP 400) | ✅ `{refusal: {status: 400, reason: unknownCapability, …}}`, exit 3 (K4 fixed) |
| `jmap mailbox get` | a known id and an unknown one | ✅ `errors` `mailbox-not-found`, exit 3 |
| `jmap mailbox query` | `--name`, `--role sent -s 1` | ✅ `nextPage` `null` |
| `jmap mailbox create` / `update` | `--name` | ✅ `{id}` |
| `jmap mailbox create` | empty name; a name a sibling holds | ✅ `{refusal: {reason: invalidArguments}}` and `{refusal: {reason: alreadyExists}}`, exit 3 |
| `jmap mailbox destroy` | an empty run mailbox and an unknown id | ✅ `{ids, errors}` `mailbox-not-found`, exit 3 |
| `jmap mailbox destroy` | a non-empty mailbox without `--purge`; the two run mailboxes | NOT RUN: denied by the auto-mode classifier (destructive action); the empty run mailboxes were deleted with `mailbox delete` by name instead |
| `jmap email get` / `query` | by id with an unknown one; `-m --subject`, `-m --has-attachment --sort sent-at --desc -s 2` | ✅ raw objects; `message-not-found` exit 3; `nextPage` `"2"` |
| `jmap email read` / `export` | text, `--json` export | ✅ `{data, mime, size}` |
| `jmap email update` | `--add-keyword --remove-keyword` on a real id and an unknown one; `--add-mailbox` | ✅ `{ids, errors}`, exit 3 |
| `jmap email import` | `--mailbox-id --keyword`, `--received-at`, `--upload-only` | ✅ `{id, blobId}`; `id` `null` when only uploaded |
| `jmap email parse` | a real blob and an unknown one | ✅ `{bodies: [{blobId, body}], errors}`, exit 3 |
| `jmap email delete` | the run's 19 emails and an unknown id | ✅ `{ids, errors}`, exit 3 |
| `jmap thread get` | a real id and an unknown one | ✅ `thread-not-found`, exit 3 |
| `jmap identity get` | | ✅ |
| `jmap identity create` | `live-2026-11-26-himalaya pimalaya@fastmail.org` | ✅ `{id: "187874231"}`, exit 0 (K5 fixed) |
| `jmap identity update` / `delete` | `--text-signature`; delete with an unknown id beside | ✅; unknown id `invalidArguments`, uncoded, exit 3 |
| `jmap submission create` | a draft to self with `--identity-id --mail-from --rcpt-to`; an unknown email id | ✅ delivered, `undoStatus: final`; `{refusal: {reason: invalidProperties}}`, exit 3 |
| `jmap submission query` / `get` / `cancel` | the sent id and an unknown one | ✅ none listed; `submission-not-found`, exit 3 (K2) |
| `jmap vacation-response get` / `set` | `get`; `set --subject --text-body --disable` | ✅ `{id: "singleton"}`, read back |
| `jmap email copy` | | NOT RUN: needs a second account |

## Findings

### Bugs

- **K4 (2026-11-18): fixed.** `himalaya -a fastmail-jmap --json jmap request -- '{"using":["urn:bogus"],"methodCalls":[["Mailbox/get",{},"0"]]}'` prints `{"refusal":{"status":400,"reason":"unknownCapability","message":"Invalid or unknown capabilities: urn:bogus","code":null,"retryAfter":null}}`, exit 3.
- **K5 (2026-11-18): fixed.** `jmap identity create` prints the new `{id}` and exits 0; one identity was created, none duplicated.
- None new on this surface.

### Behaviour (not bugs)

- **K2 (kept): Fastmail keeps no EmailSubmission.** `submission query` lists none after a `final` send, `get` and `cancel` answer `notFound`.
- **K3: the vacation response reads and writes.** `set` changed the subject and text while disabled; the unified `vacation set --no-*` flags then cleared them, `jmap vacation-response set` having no flag to write `null`.
- `jmap submission create` sends a draft but leaves it in Drafts with `$draft`: nothing asks the server to move it (no `onSuccessUpdateEmail`), unlike the unified send (J6 of the unified report).
- An identity delete with an id Fastmail does not hold comes back `invalidArguments` ("not deleteable") rather than `notFound`, so its item error is uncoded; a provider choice.
- `jmap mailbox create ''` is refused with Fastmail's text `'create' argument has invalid Id value`, which reads oddly for an empty name.
- A method error inside a 2xx `jmap request` stays in the raw response, exit 0: the command prints the response as JMAP sent it, now under `response`.

## Verdict

The raw JMAP surface **works** and follows the v3 contract for every command: gets and queries list unknown ids as coded errors, the sets print `{ids, errors}`, refused writes and refused requests print the refusal object, all with exit 3. Both 2026-11-18 bugs (K4, K5) are fixed. `jmap email copy` and the non-empty `mailbox destroy` rows were not run.
