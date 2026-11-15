---
cairn: delta
change: a-wrong-weekday-keeps-its-date
---

# Delta

## ADDED Requirements

### Requirement: A wrong weekday keeps its date
An envelope SHALL read a `Date:` whose day-of-week disagrees with its date (`Mon, 15 Nov 2026`, a Sunday) as the date it names, on every backend: IMAP from its `ENVELOPE` and Gmail from its metadata SHALL parse it again without the day-of-week when chrono refuses it, as Maildir, mbox and the read view read it.

#### Scenario: A Gmail message dated with a wrong weekday
- GIVEN a Gmail message whose `Date:` is `Mon, 15 Nov 2026 11:00:00 +0000`
- WHEN `himalaya --json envelope list` lists it
- THEN its `date` is `2026-11-15T11:00:00Z`

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
