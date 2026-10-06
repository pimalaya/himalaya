---
cairn: tasks
change: message-read-invalid-recipients
---

# Tasks

- [x] `InvalidAddresses` in `MessageHeaders`, filled by the address parser with the entries it drops.
- [x] Group names and empty groups yield no entry.
- [x] Tests: a group with an invalid entry, a bare word, `a@localhost`, a quoted local part, an encoded-word name with no address, only valid entries, `undisclosed-recipients:;`.
- [x] Fold into the spec, log, changelog.
