---
cairn: delta
change: a-google-daily-limit-is-rate-limited
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: The JSON contract is described and versioned
(commands) `rate-limited` SHALL also code a Gmail 403 `dailyLimitExceeded`, or one whose message names a daily limit or a quota per day, the unauthenticated one aside; `permission-denied` an HTTP 403 that is no rate limit nor daily quota.

## REMOVED Requirements

None.
