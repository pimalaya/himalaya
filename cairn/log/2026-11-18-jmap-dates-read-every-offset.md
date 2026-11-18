---
cairn: log
change: jmap-dates-read-every-offset
landed: 2026-11-18
---

# JMAP dates read every offset

Finding HIM-6 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/jmap/backend.rs: `earliest_start`, the residual-checked `date` and `after` bounds.
- Test `a_date_east_of_utc_is_found_on_its_own_day` (a +0900 and a +1400 message), failing on 002bf012.
- CHANGELOG (Fixed).

## Capabilities moved

- [search](../spec/search.md): *Per-backend translation*, the JMAP bound.
