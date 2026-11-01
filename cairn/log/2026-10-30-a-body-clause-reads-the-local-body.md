---
cairn: log
change: a-body-clause-reads-the-local-body
landed: 2026-10-30
---

# A pimdir body clause reads the local body

From the pimdir stable-release review of 2026-10-30: both pimdir search paths handed the shared matcher an empty body, so `body` never matched and `not body` matched everything. When the filter holds a body clause anywhere in its tree, the scan now reads each candidate's local body from the blob store and matches it as Maildir, m2dir and mbox do; a message whose body is not local, or whose blob is gone, still matches no body clause. A filter without one reads no body. This is phase 0 of the approved pimdir search plan (change pimdir-search); the full SEARCH.md search waits for io-pimdir's query-only index open.

Test: `a_body_clause_reads_the_local_body`, over two applied adds and one bodiless message, on both the paged and the whole-mailbox paths. It failed on the old code with `left: []`, `right: ["a"]`.

Capabilities moved: **search** (Per-backend translation).
