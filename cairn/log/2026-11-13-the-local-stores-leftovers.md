---
cairn: log
change: the-local-stores-campaign
landed: 2026-11-13
---

# The local stores leftovers

The residuals the re-verification of the local stores campaign found on 2026-11-13 (reports VMD and VMB) landed. On Maildir, a role alias spelled with a trailing slash or `./` marks the role, resolved through the Maildir spelling (`EmailClient::alias_mailbox_id`); `maildir create` and `rename` refuse a blank level or one holding a control character; on Maildir++ a rename or delete takes the `.a.*` children of `a` along, a rename landing one on an existing folder refused (io-maildir); a folder named with `-m` that cannot be searched fails with the permission error naming the path instead of `missing cur/ subdirectory` (io-maildir); `maildir message move` within one Maildir, without `--subdir`, changes nothing. On mbox, a directory spool is left out of `mailbox list` (io-mbox); every message command fails on a missing mbox with `mailbox-not-found` and refuses a file that is no mbox before taking a lock, so `envelope list -m README.txt` no longer reads as empty and a save into a symlink to a protected file no longer fails on the dotlock; a flag write keeps an existing `X-Mozilla-Keys` in step with the keywords (io-mbox), the cache version raised to 3 so older indexes are rescanned; the `.sbd` refusal names the Thunderbird folder suffix; the raw `mbox` id-taking commands name every unknown id under `message-not-found`, and the raw flag commands print each id once. A single message read whose id IMAP or mbox does not hold, and an IMAP mailbox not found, are coded.

Capabilities moved: backends ("mbox backend", "An mbox message is its content id", "mbox flags interoperate", "A Maildir is named relative to the root", "Mailbox role"), commands (the code table, "A command given message ids says how many it found", "The raw Maildir folder commands guard the store").

Library fixes ride on the same path patches: io-maildir (log maildirpp-children: `MaildirRenameError::TargetExists` added) and io-mbox (log thunderbird-keys-and-directory-spool: `MboxEntry::mozilla_keys`, `MboxFlags::to_mozilla_keys`).

Left open, as before: usage errors under `--json` (owner decision pending) and the empty next page after an exactly full one (F13).
