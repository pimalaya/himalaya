---
cairn: spec
capability: testing
status: current
---

# JMAP *specific* API on Fastmail: test report

Companion to [jmap-fastmail.md](jmap-fastmail.md) (unified commands). This exercises `himalaya jmap …`: the raw JMAP method surface.

- himalaya: release build of master c66b8629, default features (io-jmap 0.10.0)
- account: `fastmail-jmap`
- date: 2026-11-18
- method: every `jmap` subcommand inside the run's mailboxes (`J1` `PBh-`, `J2` `PBhV`, `J3` `PBnw`), per [provider-test-plan.md](provider-test-plan.md). The throwaway identity, mailboxes and emails were deleted.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `jmap request` | a request object, a method error inside it (`accountNotFound`) | ✅ raw response, exit 0 |
| `jmap request` | a bare method-calls array | ✅ refused `The JSON body must be an object` |
| `jmap request` | a request refused whole (`using: ["urn:bogus"]`, HTTP 400) | ❌ failure object, exit 1 (K4) |
| `jmap mailbox get` | a known id and an unknown one | ✅ `errors` `mailbox-not-found`, exit 3 |
| `jmap mailbox query` | `--name`, `--role sent -s 1` | ✅ `nextPage` `null` |
| `jmap mailbox create` / `update` | `--name` | ✅ `{id}` |
| `jmap mailbox create ''` | empty name | ✅ `{refusal: {status: null, reason: invalidArguments, …}}`, exit 3 |
| `jmap mailbox destroy` | a real id and an unknown one | ✅ `{ids, errors}`, exit 3 |
| `jmap email get` / `query` | by id, `-m --subject` | ✅ raw objects |
| `jmap email read` / `export` | | ✅ |
| `jmap email update` | `--add-keyword --remove-keyword`, a real id and an unknown one | ✅ `{ids, errors}`, exit 3 |
| `jmap email import` | `--mailbox-id --keyword` | ✅ `{id, blobId}` |
| `jmap email parse` | a real blob and an unknown one | ✅ `{bodies: [{blobId, body}], errors}`, exit 3 |
| `jmap thread get` | a real id and an unknown one | ✅ `thread-not-found`, exit 3 |
| `jmap identity get` | | ✅ |
| `jmap identity create` | name, email | ❌ created, but exit 1 (K5) |
| `jmap identity update` / `delete` | `--text-signature`; delete with an unknown id beside | ✅; unknown id `invalidProperties`, exit 3 |
| `jmap submission create` | to self, `--rcpt-to` missing | ✅ delivered, `undoStatus: final`; missing flag refused, exit 2 |
| `jmap submission query` / `cancel` | unknown id | ✅ `submission-not-found`, exit 3 (K2) |
| `jmap vacation-response get` | raw `VacationResponse/get` | ✅ Fastmail now answers (K3) |
| `jmap email copy` | | ⚪ not tested (needs a second account) |

## Findings

### Bugs

- **K4: a JMAP request refused whole by `jmap request` is no refusal result.** `himalaya -a fastmail-jmap --json jmap request -- '{"using":["urn:bogus"],"methodCalls":[["Mailbox/get",{},"0"]]}'` prints `{"code":null,"error":"JMAP send failed: HTTP 400: JMAP unknownCapability: Invalid or unknown capabilities: urn:bogus","sources":[],"backtrace":null}`, exit 1. commands.md (*A protocol refusal is a result*) asks for `{refusal: {status: 400, reason: "unknownCapability", …}}` and exit 3, the `request` commands included; src/jmap/error.rs `refusal` reads `JmapSendError::Request`, so the error `send_raw` returns (wrapped in io-jmap's `JmapClientStdError::Send`) does not reach it on this path.
- **K5: `jmap identity create` fails after the server created the identity.** `himalaya -a fastmail-jmap --json jmap identity create 'himalaya-v3-live identity' pimalaya@fastmail.org` prints `{"code":null,"error":"JMAP identity created without an id",…}`, exit 1, while `jmap identity get` then lists the new identity (`187602447`, deleted afterwards). The cause is in io-jmap 0.10.0: src/rfc8621/identity/set.rs builds `JmapIdentitySetOutput` with `created: BTreeMap::new()` and `updated: BTreeMap::new()` whatever the server answered, so src/jmap/identity/create.rs never finds its `new` entry. A caller retrying on exit 1 creates a duplicate each time, which likely explains the 32 leftover test identities found on this account on 2026-07-17. An upstream fix, then a himalaya bump.

### Behaviour (not bugs)

- **K2 (kept): Fastmail keeps no EmailSubmission.** `submission query` lists none after a `final` send; `cancel` answers `notFound`.
- **K3: the vacation response now reads.** `jmap vacation-response get` prints the raw `VacationResponse/get` object (`isEnabled: false`); no `set` was attempted.
- An identity delete with an id Fastmail cannot parse comes back `invalidProperties` rather than `notFound`, so its item error is uncoded; a provider choice.
- A method error inside a 2xx `jmap request` stays in the raw response, exit 0: the command prints the response as JMAP sent it.

## Verdict

The raw JMAP surface **works** and follows the v3 contract for every typed command: gets and queries list unknown ids as coded errors, the sets print `{ids, errors}`, refused writes print the refusal object, all with exit 3. Two bugs: a request refused whole by `jmap request` exits 1 without the refusal object (K4), and `jmap identity create` reports a failure after creating (K5, io-jmap).
