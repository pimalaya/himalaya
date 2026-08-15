---
cairn: delta
change: stream-0-3-migration
---

# Delta

The migration itself moves no requirement: which crate opens the socket and under what type name is not something the spec describes. What the transport now does on a stream that reports it is not ready is user-visible, and is added to the backends capability.

## ADDED Requirements

### Requirement: Network transport resilience
The network backends SHALL run over a transport that retries a stream reporting it is not ready (`EAGAIN` on Unix, `EINTR`, and the Windows spelling of an expired deadline) rather than ending the exchange on it. Each read and each write carries its own budget of one minute, so a slow but progressing transfer never runs out of it, and exhausting the budget SHALL fail with a message naming it rather than a raw errno.

Opening a connection SHALL arm a socket read deadline matching that budget, so a server going silent on an otherwise healthy connection ends the command instead of blocking forever.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
