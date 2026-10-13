---
cairn: log
change: pimalaya-libs-unreleased
landed: 2026-10-13
---

# Pimalaya libraries aligned on their pushed, unreleased revisions

io-pimdir moves from `be03184` to `189db40` (0.8), io-msgraph from 0.4.3 to `2ae2637` (0.6), io-jmap from 0.4.1 to `919eab4` (0.6) and io-gmail from 0.4 to 0.5; io-http (`e181530`) and io-imap (`b1ab38e`) are patched to their pushed fixes, the same combination the Android app compiles. The `[patch.crates-io]` table holds the git revisions until they are released.

Code moved only where the libraries broke their API: the pimdir reads `count_unread` and `list_mail_page_filtered` take their new `since` floor as `None`, the JMAP `Email/query` and the Graph message listing fill the new fields with their defaults, and the JMAP submission errors `tooLarge`, `forbidden`, `overQuota` and `rateLimit` are named. No `--json` output changes shape. Tests run with the default features and with `--no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir`, as MOA builds it.

Capabilities moved: none.
