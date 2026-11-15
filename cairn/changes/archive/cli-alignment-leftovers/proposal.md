---
cairn: change
id: cli-alignment-leftovers
status: landed
created: 2026-10-18
---

# What himalaya still owes the family alignment

## Why

The cross-repo alignment audit of 2026-10-18 (CLI_ALIGNMENT_PLAN.md, which kept the item ids of the 2026-08-28/29 plan) found one item still open in himalaya, re-checked against the code on 2026-10-18.

- **G2**: `ImapClient::new` and `SmtpClient::new` each build their own `SecretResolver` (src/imap/client.rs:44, src/smtp/client.rs:45), so a live account whose IMAP and SMTP blocks share a credential command runs it once per client. secret-command-resolved-once left this out on 2026-08-29 because SMTP connects lazily on the first send; himalaya-tui has since threaded one resolver through its client constructors (credentials-unlock-once), which is the shape to compare against.

A2 (dropping the `[patch.crates-io]` pins, pimalaya-cli and pimalaya-config among them) is a task of v3-release and is not repeated here.

himalaya is the reference the shared layer is cut from. H1 and H2, the local copies of wizard-save-layer and shared-account-resolution, are settled: `ExistingConfig`, `append_or_print` and `print_saved` left with the wizard (the-wizard-leaves, 2026-11-11), and `resolve_account` (src/cli.rs:178) resolves through pimalaya-config's shared `take_account` (the-first-run-welcome, 2026-11-11).

## What

Close G2.
