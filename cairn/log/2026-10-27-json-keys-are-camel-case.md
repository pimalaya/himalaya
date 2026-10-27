---
cairn: log
change: json-keys-are-camel-case
landed: 2026-10-27
---

# `--json` keys are camelCase

Every struct the printer serializes now carries `#[serde(rename_all = "camelCase")]`: the registered output types and the row and nested types they carry. 96 structs changed: 26 dropped a kebab-case rename, the rest left serde's snake_case default (`next_page`, `uid_validity`, `method_responses`, `server_id`, `send_as`). The 58 registered types that were not `*Output` were renamed `<Domain><Target><Verb>Output` (`Envelopes` to `EnvelopeListOutput`, `FlagsTable` to `ImapFlagListOutput`, `MaildirFlagListOutput` and so on), and the `imap thread` schema shim now titles its schema `ImapThreadOutput`.

Left alone: src/config.rs (TOML), the transparent newtypes over io-gmail, io-msgraph and io-jmap resources, and every enum governing values (`ErrorCode`, `IanaFlag`, `DeleteAction`, `PimdirQueueState`, `PartRole`, `MailboxRole`). The pimdir `submit` payload and the mbox cache are not printer output and keep their keys; `Envelope`, which the mbox cache stores, reads its three former kebab-case keys through `alias` so a cache written before keeps its threading pointers and attachment mark.

Regenerated all 97 schemas with every feature: no property key holds a hyphen, an underscore or a dot. The contract `$id` still names `v2`, following the crate version, which this change does not bump. The dated manual test reports under cairn/spec/testing/ keep the keys they observed.

Capabilities moved: **commands** (ADDED: JSON keys are camelCase; the Gmail draft and history requirements now spell their keys in camelCase), **backends** (the pimdir attachment mark is `hasAttachment`).
