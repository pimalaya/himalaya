---
cairn: spec
capability: provider-quirks
status: current
---

# Provider quirks

Provider-specific behaviour that the generic backends have to accommodate. Each quirk is a fact about a real provider, not a Himalaya design choice.

### Requirement: Bearer-only proprietary APIs
The Gmail and Microsoft Graph REST APIs SHALL be authenticated with a single OAuth 2.0 bearer token only; neither accepts an app password. Their account config carries one token field, and the wizard offers only the API-token credential path for them.

### Requirement: SASL OAuth carries a username
The IMAP and SMTP SASL OAuth mechanisms SHALL carry the login, not just the token: `XOAUTH2` encodes `user=<login>` and `OAUTHBEARER` encodes the login as the GS2 authorization identity. The wizard therefore prompts for a login on those mechanisms. JMAP over HTTP `Authorization: Bearer` is token-only and prompts no login.

### Requirement: JMAP download host may differ
A JMAP provider MAY serve blob downloads from a different host than its API endpoint (Fastmail serves downloads off a user-content host). The JMAP client SHALL open a fresh authenticated connection to the download host rather than reuse the API socket, which the API server would answer with a redirect.

### Requirement: IMAP special-use is inbox-only for now
IMAP SHALL mark only the reserved `INBOX` with a mailbox role. Reading the Sent/Drafts/Trash/Junk/Archive roles needs LIST `RETURN (SPECIAL-USE)` (RFC 6154), which io-imap cannot yet issue because upstream imap-codec has no LIST-EXTENDED support. The other IMAP roles are set through `mailbox.alias.<role>` until then.

### Requirement: RFC 2971 ID after auth
IMAP SHALL support sending an RFC 2971 `ID` command right after authentication, configured by `imap.id.{auto, fields}`, because some providers require it before serving other commands. Unset `fields` sends himalaya's canned `name`, `version`, `vendor` and `support-url`; an empty map sends `ID NIL`.

### Requirement: SASL-IR capability is overridable
IMAP SHALL accept an `imap.sasl-ir` override deciding whether the RFC 4959 initial response is sent inline with `AUTHENTICATE`. `false` never inlines and waits for the server's continuation request, `true` always inlines, and unset follows the advertised `SASL-IR` capability. The override applies to every SASL mechanism, because a server that mishandles the inline form does so in its command parser rather than per mechanism.

### Requirement: Coremail advertises SASL-IR falsely
Coremail (126.com, 163.com) SHALL be treated as advertising `SASL-IR` without honouring it: it answers the inline initial response with a tagged `BAD`. Such an account needs `imap.sasl-ir = false`, and (per the `ID` quirk) usually `imap.id.auto = true` as well, without an empty `imap.id.fields`: 163.com acknowledges `ID NIL` and then refuses the next command with `Unsafe Login`. 163.com also answers `AUTHENTICATE PLAIN` with `NO ... Not support mechanism`, advertising only `AUTH=XOAUTH2`, so an app password goes through `imap.sasl.login` (the IMAP `LOGIN` command) there.
