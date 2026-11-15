---
cairn: tasks
change: the-jmap-escape-hatch-is-request
---

# Tasks

- [x] Move src/jmap/query.rs to src/jmap/request.rs, `JmapQueryCommand` and `JmapQueryOutput` to `JmapRequestCommand` and `JmapRequestOutput`, the about to "Send a raw JMAP request and print the response".
- [x] `JmapCommand::Request`, and `JmapCommand::Query` hidden, both running `JmapRequestCommand`.
- [x] Register the schema under `himalaya-jmap-request` and `himalaya-jmap-query`.
- [x] `the_raw_request_is_jmap_request`: `jmap --help` lists `request` and not `query`, both parse, both keys hold one schema; it fails on the old tree. `an_option_after_jmap_method_calls_is_the_option` runs `jmap request`.
- [x] Fold into cairn/spec/commands.md; the CHANGELOG line; the log entry.
