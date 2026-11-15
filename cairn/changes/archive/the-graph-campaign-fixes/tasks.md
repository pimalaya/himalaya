---
cairn: tasks
change: the-graph-campaign-fixes
---

# Tasks

- [x] Batch Graph sub-requests four at a time, retrying 429 and 5xx after `Retry-After`, failing a role lookup other than 404
- [x] List every Graph folder by path, every page read, and refuse a mailbox value naming no folder
- [x] Scope the Graph id-taking writes to the mailbox by `parentFolderId`, and clear the categories on `flag set`
- [x] Follow every page of an unpaged Graph envelope listing, date envelopes by `sentDateTime`, quote each KQL word
- [x] Fix the `msgraph messages list` search quoting, count, next link and `--top 0`, and page the folder listings
- [x] Declare Graph `submit.copy` `partial`
- [x] Refuse an IMAP `\Noinferiors` parent, require ids on the raw local store commands, order the Maildir flags
- [x] Patch io-msgraph by path (path segments, categories tri-state, messages list count)
- [x] Fold the spec, update CHANGELOG.md
