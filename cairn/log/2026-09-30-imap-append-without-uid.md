---
cairn: log
change: imap-append-without-uid
landed: 2026-09-30
---

# Accept a successful IMAP append without a recoverable UID

`message send --save` saves before it sends. A server acknowledging the append without `APPENDUID` made Himalaya search the submitted `Message-ID` for the new UID; QQ Exmail rewrites that header, so the search found nothing, Himalaya failed, and SMTP never ran although the copy was saved. Contributed in [#759].

## What landed

**The shared `add_message` returns an optional id.** IMAP returns none when an acknowledged append has neither `APPENDUID` nor a search match, logging which at debug level; a rejected `APPEND` still fails before any send. JMAP returns none when `Email/import` reports no id, where it used to return an empty string. Maildir, m2dir and pimdir keep returning an id.

**`message add` reports the save without an id**, `null` under `--json`, and save-then-send goes on.

## Capabilities moved

- backends: *Append and search gaps* gains the optional add id

## Verification

`cargo test --all-features` passes and clippy reports nothing. The contributor reproduced the failure and the fix against QQ Exmail. Whether QQ reports `UIDNOTSTICKY` or just omits `APPENDUID` is still unknown, no IMAP trace having been shared.

[#759]: https://github.com/pimalaya/himalaya/pull/759
