---
cairn: change
id: cli-alignment-leftovers
status: active
created: 2026-10-18
---

# What himalaya still owes the family alignment

## Why

The cross-repo alignment audit of 2026-10-18 (CLI_ALIGNMENT_PLAN.md, which kept the item ids of the 2026-08-28/29 plan) found one item still open in himalaya, re-checked against the code on 2026-10-18.

- **G2**: `ImapClient::new` and `SmtpClient::new` each build their own `SecretResolver` (src/imap/client.rs:44, src/smtp/client.rs:45), so a live account whose IMAP and SMTP blocks share a credential command runs it once per client. secret-command-resolved-once left this out on 2026-08-29 because SMTP connects lazily on the first send; himalaya-tui has since threaded one resolver through its client constructors (credentials-unlock-once), which is the shape to compare against.

A2 (dropping the `[patch.crates-io]` pins, including pimalaya-cli and pimalaya-config at Cargo.toml:96-97) is already a task of v2-release and is not repeated here.

himalaya is the reference the shared layer is cut from. Once pimalaya-cli lands wizard-save-layer and shared-account-resolution, the local copies go: `ExistingConfig`, `append_or_print` and `print_saved` (src/wizard/configure.rs:103, :223, :278) and `resolve_account` (src/cli.rs:221).

## What

Close G2 and adopt the shared layer.
