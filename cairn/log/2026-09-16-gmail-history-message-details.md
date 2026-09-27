---
cairn: log
change: gmail-history-message-details
landed: 2026-09-16
---

# Retain added-message details in Gmail history

Added `messages-added-details` to the JSON history listing so incremental consumers retain Gmail's supplied thread ids and labels without extra message fetches. Kept the existing id arrays and text output. Missing labels stay empty and absent thread ids stay null; the CLI does not infer either from other history changes.

Updated the commands capability with Gmail history retains added-message details. This is the additive output fix in issue #752.
