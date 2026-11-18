---
cairn: log
change: a-protocol-refusal-is-a-result
landed: 2026-11-18
---

# A protocol refusal is a result

Findings COH-5, HIM-21 and the `request` half of D26 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18.

## What landed

- src/shared/refusal.rs: the `{refusal: {status, reason, message, code, retryAfter}}` result of a `gmail`, `msgraph` or `jmap` command the server refused, read by src/gmail/client.rs, src/msgraph/client.rs and src/jmap/error.rs (HTTP status, JMAP request and method errors, `JmapSetRefusal`), printed by the three dispatchers, exit 3.
- src/sieve/outcome.rs: a typed `sieve` command answered `NO` or `BYE` prints its completion, exit 3.
- src/smtp/send.rs: a refused `smtp send` prints `{refusals}`, exit 3.
- src/jmap/set.rs: the six JMAP sets print `{ids, errors}`, exit 3 when an object was refused.
- src/json_schema.rs: each schema of those namespaces admits the refusal.
- Tests, each failing on 72c1c0ca: `gmail::cli::tests::a_refused_request_is_the_result`, `msgraph::cli::tests::a_refused_request_is_the_result`, `a_method_error_is_the_result`, `a_refused_create_is_the_result`, `a_destroy_applied_in_part_names_both_halves`, tests/send_outcome.rs `a_refused_smtp_send_is_the_result`; unit tests `a_refusal_is_the_result` and `a_refused_command_prints_its_completion`.
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [commands](../spec/commands.md): *A protocol refusal is a result* (added), *A server's refusal codes the whole command*, *The Gmail and Graph escape hatches are requests*, *Protocol listings print the provider's payload*, *A command acting on several items lists what failed*.
