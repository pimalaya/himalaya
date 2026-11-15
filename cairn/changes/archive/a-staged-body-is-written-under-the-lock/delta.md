---
cairn: delta
change: a-staged-body-is-written-under-the-lock
---

## ADDED Requirements

### Requirement: A pimdir body is written under the staging lock
A pimdir write carrying a body (`add_message`, `send_message`) SHALL open its producer, which holds the store's staging lock (pimdir STORAGE §8), before it writes the body, and SHALL write the body through that producer's blob directory, enqueueing the action that pins it before the producer drops. No body SHALL reach the blob tree while a collector holds the lock: the write waits for it.

#### Scenario: A collector running beside a staged add
- GIVEN a pimdir store whose collector holds `objects.lock` exclusively
- WHEN a message is added to a mailbox
- THEN no body is written until the collector releases the lock, and then the body lands and its `add` is queued
