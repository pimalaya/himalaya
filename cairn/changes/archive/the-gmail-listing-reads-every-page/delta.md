---
cairn: delta
change: the-gmail-listing-reads-every-page
---

# Delta

## ADDED Requirements

### Requirement: An unpaged Gmail listing reads every page
An unpaged Gmail `envelope list` or `envelope search` (`--page-size 0`) SHALL follow every next page token, asking 500 messages a page, Gmail's largest `maxResults`, and SHALL say no page follows only once Gmail served the last one.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
