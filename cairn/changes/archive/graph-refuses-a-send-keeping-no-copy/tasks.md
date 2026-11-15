---
cairn: tasks
change: graph-refuses-a-send-keeping-no-copy
---

# Tasks

- [x] src/capability.rs: `FILES_SENT_COPY` and `Resolution::require_no_copy`; the Graph `submit.copy` detail.
- [x] src/cli.rs `run_shared` checks it beside the capabilities, on the kept client's resolution or before a new client connects; src/shared/message/cli.rs `MessageCommand::keeps_no_copy`.
- [x] tests/capability.rs `graph_refuses_a_send_keeping_no_copy`: `message send --no-save` and `message compose --send --no-save` on Graph refused coded `capability-missing` before the token command runs; on Gmail the send goes on to connect.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
