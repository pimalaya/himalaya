---
cairn: change
id: the-capped-message-is-refused
status: landed
created: 2026-11-18
---

# The capped message is refused

## Why

Finding HIM-4 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. A message past mime-rs's own entity cap (`ENTITIES_MAX`, 10,000) was read in part: the parser leaves the multipart it stopped at unsplit, which src/shared/message/part.rs counted as one part, so 10,001 attachment parts exited 0 as one `multipart/mixed`, and a decoy text part beside such an inner multipart hid what it held from `message read`, `attachment list` and the MCP tools. cairn/spec/commands.md says such a message is refused rather than read in part. The finding asked mime-rs for a mark on a capped parse: mime-rs 0.2.0 already sets one, `MimeAnomaly::PartLimit` on the entity it stopped at (`NestingLimit` past its depth), so no library change is needed.

## What

- `measure` refuses, with `message-too-complex`, an entity carrying `PartLimit` or `NestingLimit`.
