---
cairn: change
change: imap-sasl-ir-override
---

# Delta

## ADDED Requirements

### Requirement: SASL-IR capability is overridable
IMAP SHALL accept an `imap.sasl-ir` override deciding whether the RFC 4959 initial response is sent inline with `AUTHENTICATE`. `false` never inlines and waits for the server's continuation request, `true` always inlines, and unset follows the advertised `SASL-IR` capability. The override applies to every SASL mechanism, because a server that mishandles the inline form does so in its command parser rather than per mechanism.

### Requirement: Coremail advertises SASL-IR falsely
Coremail (126.com, 163.com) SHALL be treated as advertising `SASL-IR` without honouring it: it answers the inline initial response with a tagged `BAD`. Such an account needs `imap.sasl-ir = false`, and (per the `ID` quirk) usually `imap.id.auto = true` as well.

## MODIFIED Requirements

## REMOVED Requirements
