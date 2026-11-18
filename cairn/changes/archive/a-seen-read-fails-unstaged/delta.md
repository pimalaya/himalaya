---
cairn: delta
change: a-seen-read-fails-unstaged
---

# Delta

## ADDED Requirements

### Requirement: A read asked to mark the message seen fails unmarked
`message read --seen` SHALL fail on every backend when the `\Seen` it asks for cannot be set: IMAP fetching without `PEEK`, Maildir and mbox renaming or rewriting the message, JMAP, Gmail and Graph sending the flag change, and pimdir staging it in its queue. The failure SHALL carry its cause's code (`unsupported` for a source of the pimdir store declaring `mail.flags.seen` `none`, `store-busy`, ...), and SHALL NOT be downgraded to a warning: a caller asking for the flag learns it is not set.

#### Scenario: A pimdir source that keeps no seen flag
- GIVEN a pimdir account whose source declares `mail.flags.seen` `none`
- WHEN `himalaya --json message read --seen <id>` runs
- THEN it fails coded `unsupported`, exit 1, and stages nothing
