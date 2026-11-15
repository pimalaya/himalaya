---
cairn: delta
change: an-unknown-size-is-null
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Graph envelopes
An unpaged Graph `envelope list` or `envelope search` (`--page-size 0`) SHALL follow every paging link, asking 1000 messages a page, Graph's largest `$top`. A Graph envelope's date SHALL be its `sentDateTime`, the `Date:` header, else its `receivedDateTime`. Its `size` SHALL be `null`, Graph's message carrying no size of the raw message, as a JMAP or pimdir envelope whose size the server or the store does not give.

## REMOVED Requirements

None.
