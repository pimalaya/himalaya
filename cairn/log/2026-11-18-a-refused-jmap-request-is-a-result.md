---
cairn: log
change: a-refused-jmap-request-is-a-result
landed: 2026-11-18
---

# A refused JMAP request is a result

Finding K4 of the live provider run of 2026-11-18 (cairn/spec/testing/jmap-specific-fastmail.md).

## What landed

- src/jmap/error.rs: `unwrapped` reads the send, session and blob errors io-jmap's `JmapClientStdError` wraps transparently, so `jmap request` refused whole prints the refusal object, exit 3.
- Test `jmap::backend::tests::send::a_request_refused_whole_is_the_result`, failing on 3b9f9127.
- CHANGELOG (Fixed).

## Capabilities moved

- [commands](../spec/commands.md): *A protocol refusal is a result* (a scenario added).
