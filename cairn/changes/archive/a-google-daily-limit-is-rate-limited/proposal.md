---
cairn: change
id: a-google-daily-limit-is-rate-limited
status: landed
created: 2026-11-18
---

# A Google daily limit is rate-limited

## Why

Finding COH-4 of the family's cli-family-analysis-2026-11-15 plan, with the owner's ruling of 2026-11-18: a Google daily quota (`dailyLimitExceeded`) was coded `permission-denied`, io-gmail's `is_rate_limited` leaving it out, where Carillon, Cardamum, Calendula and Neverest code it `rate-limited`. `quota-exceeded` keeps its storage meaning (IMAP `OVERQUOTA`, Graph `ErrorQuotaExceeded`).

## What

A Gmail refusal naming the daily quota, by reason or by message (the unauthenticated daily limit aside), is `rate-limited`, the whole command and a batch item alike. Breaking: the code of that refusal changes.
