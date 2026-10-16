---
cairn: tasks
change: forward-carries-source-attachments
---

- [x] Carry the source's attachment parts into the forward, multipart containers skipped
- [x] Name nameless parts: subject plus `.eml` for an attached email, MIME extension otherwise
- [x] `--no-attachments` on `message forward`
- [x] Tests in `tests/message_forward.rs`
- [x] CHANGELOG
- [x] Fold into spec/commands.md, log
