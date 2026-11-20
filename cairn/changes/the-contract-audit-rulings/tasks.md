---
cairn: tasks
change: the-contract-audit-rulings
---

# Tasks

- [x] Cargo.toml: `[patch.crates-io] pimalaya-cli = { path = "../cli" }`, to become a release pin (0.8.0, the kit's account API breaking) before commit.
- [x] src/account/check.rs: pimalaya-cli's `AccountCheckOutput`, `tls_check` naming no TLS over a cleartext connection; tests tests/account.rs `an_account_check_names_no_tls_over_a_cleartext_connection`, tests/common.rs and tests/exit.rs reading `accounts[0]`.
- [x] src/main.rs, src/cli.rs: `clap::meet_bare_invocation`; test tests/first_run.rs `a_bare_invocation_under_json_is_refused_as_usage_invalid`.
- [x] src/config.rs, src/config/envelope.rs, src/backend.rs: the by-name refusals, the `m2dir` and `backend` fields go; tests `a_removed_backend_block_is_an_unknown_key`, `the_tables_of_earlier_versions_are_unknown_keys`, `envelope_table_is_an_unknown_key`.
- [x] src/pimdir/message/add.rs, send.rs, mailbox/create.rs, rename.rs, delete.rs, backend.rs (`PimdirStaged::link_id`, `PimdirQueuedIntent::mailbox`): `id`; tests tests/pimdir_queue.rs, `a_mailbox_at_the_top_prints_a_null_parent`.
- [x] src/shared/message/read.rs and every backend's `get_message` (flags beside the bytes), src/email/envelope.rs (`from_raw` for every build, `Envelope: Default`): the read row; tests tests/local_stores.rs `a_read_carries_its_id_and_an_unknown_one_is_not_found`, tests/message_parse.rs.
- [x] src/error.rs, src/shared/envelope/list.rs, search.rs, src/jmap/*/query.rs: `page_token_invalid` for every list, `conflicts_with`; test tests/local_stores.rs `a_bad_page_token_is_coded_and_conflicts_with_a_page_number`.
- [x] src/pimdir/backend.rs: `capability::missing` on the mailbox writes; tests/pimdir_refusals.rs.
- [x] src/mcp.rs: `NO_COPY`, `no-save` in `SEND_ARGS`; tests tests/send_outcome.rs `an_mcp_send_files_no_copy_unless_writes_are_allowed` (failed first: `"saved":true`), tests/mcp.rs `write_and_send_tools_under_their_flags`.
- [x] src/shared/message/handler.rs, add.rs: `MessageRouteOutput::new`, the template's `date` and `messageId`; tests `a_queued_add_says_it_is_queued`, `a_template_carries_the_date_and_message_id_of_the_message`.
- [x] src/shared/message/delete.rs: `ids`. src/maildir/create.rs, rename.rs, delete.rs, list.rs: the path as `id`.
- [x] src/pimdir/queue/list.rs, show.rs, src/pimdir/backend.rs (`queued_rows` of every kind, an `Outcome`, `listed_row`): A4; tests tests/pimdir_queue.rs, `a_queued_row_prints_its_list_row`, `an_unknown_row_is_not_found`, `an_undecodable_row_is_an_item_error_of_its_mailbox`.
- [x] src/imap/raw.rs, src/imap/cli.rs, src/cli.rs: `ImapCommand::check`; test tests/early_refusal.rs.
- [x] src/shared/envelope/search.rs, cli.rs: `query_parse_error_line`; test `a_refused_query_is_one_line_under_json`. Help paragraphs naming exit 3. src/shared/attachment/download.rs: the `Saved` lines.
- [x] Fold into cairn/spec/commands.md, backends.md, config.md, mcp.md; CHANGELOG; MIGRATION.md; the log entry.
- [ ] Before commit: replace the path patch with the released pimalaya-cli pin.
