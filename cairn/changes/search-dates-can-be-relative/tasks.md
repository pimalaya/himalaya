---
cairn: tasks
change: search-dates-can-be-relative
---

# Tasks

- [ ] Check whether Graph's KQL `sent>` and `sent<` accept a time of day, against the Microsoft test account
- [ ] src/email/search/filter: a date value type (day or instant); the parser reads the five forms with a now given by the caller; `date` refuses an instant; the keywords `ago`, `today`, `yesterday`
- [ ] Local zone as pimdir search reads it, UTC fallback with one warning
- [ ] Local evaluator: instants compared to the second
- [ ] Gmail and JMAP: instants in `after:`/`before:` and in the UTCDate filters, the JMAP re-check on instants
- [ ] IMAP: the `SENTON` plus `SENTSINCE`/`SENTBEFORE` split on the root `and` chain; `search-unsupported` under `or` and `not`
- [ ] Graph: native or the IMAP split, per the first task
- [ ] The `QUERY` help and the long help
- [ ] Tests listed in the proposal
- [ ] CHANGELOG; fold the delta into cairn/spec/search.md; log; status landed
