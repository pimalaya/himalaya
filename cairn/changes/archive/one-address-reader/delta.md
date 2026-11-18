---
cairn: delta
change: one-address-reader
---

# Delta

## ADDED Requirements

### Requirement: Addresses are read by one parser
(backends) An address header SHALL be read by mime-rs wherever Himalaya reads one from a header value; a comma or angle brackets inside a quoted display name SHALL stay in the name.

## MODIFIED Requirements

### Requirement: The read view lists unreadable address entries
(commands) Each entry SHALL be read by mime-rs as every listing reads it, its comments dropped.

## REMOVED Requirements

None.
