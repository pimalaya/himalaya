---
cairn: log
change: imap-id-canned-default
landed: 2026-10-09
---

# imap.id.auto sends the canned fields by default

Follows [#774] by @dimpurr, who found that 163.com acknowledges `ID NIL` and then refuses the next command with `Unsafe Login`. `imap.id.fields` is now optional: unset, the auto-`ID` sends himalaya's canned `name`, `version`, `vendor` and `support-url`; an explicit empty map still sends `ID NIL`. `imap id` and the auto-`ID` share one list of canned keys. The sample config no longer tells 163.com users to set `imap.id.fields`, and its `imap.sasl.login` block is relabelled the IMAP `LOGIN` command (RFC 9051), which is what it sends.

Capabilities moved: **provider-quirks** (IMAP `ID` default parameters; Coremail requirement).

[#774]: https://github.com/pimalaya/himalaya/pull/774
