---
cairn: delta
change: pimdir-follows-storage-draft-09
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir searches through the store's index (search)
The SEARCH query language Himalaya answers is search-draft-09 §8, where it was search-draft-08 §8; the rest of the requirement is unchanged here (pimdir-search-reads-local-days adds the zone).

### Requirement: A missing or outdated pimdir index is a coded error (search, the scenario)

#### Scenario: An index of an earlier draft
- GIVEN a pimdir store whose `index_meta.format_version` is `search-draft-08`
- WHEN `himalaya --json pimdir search lunch` runs
- THEN the command exits 1 with the code `index-version`, and the index is left as it is

## REMOVED Requirements

None.
