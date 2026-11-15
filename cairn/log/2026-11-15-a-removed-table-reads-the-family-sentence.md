---
cairn: log
change: a-removed-table-reads-the-family-sentence
landed: 2026-11-15
---

# A removed table reads the family sentence

Task H25 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (contract.md, H-7): a v1 `folder.alias` met serde's generic unknown-field error.

## What landed

- Cargo.toml and Cargo.lock: pimalaya-config 0.5.1, its requirement raised to "0.5.1".
- src/config.rs: `TableRemoved`, `AttachmentRemoved`, `AccountRemoved` and `FolderRemoved` through `pimalaya_config::removed_key!`, the v3 ones `the output being plain text` (`PLAIN_TEXT`), `folder` naming `mailbox.alias`; src/config/envelope.rs `EnvelopeTableRemoved`. The `removed()` helper and the `refuse_*` deserializers are gone.
- Tests: the v2 refusals assert the family sentence; `the_v1_folder_table_names_mailbox_alias`.
- Left as it is: serde's "expected one of" list still names the refused tables (L13, pimalaya-config's loader). No key is renamed or removed: the live tester's files load as before.

## Capabilities moved

- [config](../spec/config.md): *Unknown keys are refused* names the family sentence and v1's `folder`, with the scenario *A v1 folder alias*; *No colour is configured*, its scenario quoting the new sentence.
