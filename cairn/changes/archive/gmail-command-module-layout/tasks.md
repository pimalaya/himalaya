---
cairn: tasks
change: gmail-command-module-layout
---

# Tasks

- [x] Split messages, drafts, threads, labels, attachments, history, profile and settings into a sibling module plus one file per subcommand.
- [x] Split the nine settings resources the same way.
- [x] Gather the three copies of `FormatArg` into src/gmail/format.rs.
- [x] Rename the twelve bare-verb command types to `GmailSettings<Target><Verb>Command`; give the send-as commands their `Settings` target.
- [x] Replace the local `read_message` with the shared `MessageArg` in messages send/import/insert and drafts create/update; delete src/gmail/input.rs.
- [x] Lift `LabelsTable::new` and the filter summaries out of their duplicated call sites.
- [x] Remove blank lines between struct fields and enum variants across the subtree (inline-003).
- [x] Update the schema registry paths; verify the CLI surface and all schemas are unchanged.
- [x] Build/test/fmt/clippy.
- [x] Fold into cairn/spec/commands.md; log; land.
