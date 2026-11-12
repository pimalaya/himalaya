---
cairn: log
change: the-page-prints-above-the-items
landed: 2026-11-12
---

# The page prints above the items

Every list prints its fields in one block above its items and nothing under them: `Account:`, what is listed, then `Page:`, `Next page:`, `Incomplete:`, `History id:` and `Cancel:`, a blank line parting the block from the items. The owner asked for it on 2026-11-12, having tried `mailbox` and `envelope` and found the block every-list-prints-as-envelopes-do printed under the items disturbing. `listing::Listing` lost its `footer`. Capability moved: commands (requirement "Text output is plain text" modified, its scenarios "An envelope list", "A mailbox list" and "A page reached by a token" with it).
