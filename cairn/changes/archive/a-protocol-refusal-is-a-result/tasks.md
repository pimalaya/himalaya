---
cairn: tasks
change: a-protocol-refusal-is-a-result
---

# Tasks

- [x] src/shared/refusal.rs: `RefusalOutput`, `Refusable<T>`, `print` over per-protocol readers; tests `a_refusal_is_the_result`.
- [x] Readers: src/gmail/client.rs `refusal`, src/msgraph/client.rs `refusal`, src/jmap/error.rs `refusal` (HTTP status, request error, method error, `JmapSetRefusal` built by `set_error`); dispatch in src/gmail/cli.rs, src/msgraph/cli.rs, src/jmap/cli.rs; tests `gmail::cli::tests::a_refused_request_is_the_result`, `msgraph::cli::tests::a_refused_request_is_the_result`, `a_method_error_is_the_result`, `a_refused_create_is_the_result`.
- [x] src/jmap/set.rs `JmapSetReceipt`; the six sets; src/jmap/mailbox/update.rs refuses through `set_error`; test `a_destroy_applied_in_part_names_both_halves`.
- [x] src/sieve/outcome.rs and its dispatch in src/sieve/cli.rs; test `a_refused_command_prints_its_completion`.
- [x] src/smtp/send.rs `SmtpSendRefusalOutput`; test tests/send_outcome.rs `a_refused_smtp_send_is_the_result`.
- [x] src/json_schema.rs: `refusable!`, `sieve!`, `SmtpSendOutcome`, `JmapSetReceipt`.
- [x] Fold into cairn/spec/commands.md, CHANGELOG, MIGRATION.
