---
cairn: tasks
change: protocol-commands-print-the-raw-payload
---

# Tasks

- [x] JMAP src/jmap/raw.rs (`get` on every object when given no id, `Identity` under the submission capability, `create`), thread/get.rs, identity/get.rs, submission/create.rs, submission/query.rs; tests `a_get_without_ids_asks_for_every_object`, `a_create_keeps_what_the_server_set`.
- [x] Gmail src/gmail/client.rs (`raw_list_at`, `raw_get`, test `scripted`), labels/list.rs, labels/get.rs (one id), settings/sendas/list.rs, delegates/list.rs, forwarding_addresses/list.rs, filters/list.rs; tests `a_label_prints_as_gmail_sent_it`, `the_labels_print_as_gmail_sent_them`, `the_settings_lists_print_as_gmail_sent_them`.
- [x] Graph src/msgraph/client.rs (`raw_get`, test `scripted`), mail_folders/get.rs (one id), mail_folders/list.rs (no get path), profile/get.rs; tests `a_folder_prints_as_graph_sent_it`, `the_user_prints_as_graph_sent_it`.
- [x] D26 src/shared/raw_json.rs, src/gmail/request.rs, src/msgraph/request.rs, src/jmap/request.rs (whole request object), src/jmap/query.rs (the former form, hidden), src/jmap/cli.rs, src/json_schema.rs; tests `a_request_prints_the_raw_response` (Gmail, Graph), `the_raw_request_is_jmap_request`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
