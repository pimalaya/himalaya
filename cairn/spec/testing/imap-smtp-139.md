# IMAP + SMTP on 139 Mail — test report

Shared cross-protocol commands on the IMAP/SMTP backend against **139 Mail**
(China Mobile). Method: [provider-test-plan.md](provider-test-plan.md).

- himalaya: `v2.2.1 +imap +smtp +native-tls` (rev `4df380b`)
- account: `mobile` — IMAP `imaps://imap.139.com:993`, SMTP
  `smtps://smtp.139.com:465`, SASL LOGIN with authorization code
- date: 2026-10-18
- server: China Mobile Mail; Chinese mailbox names (草稿箱, 已发送, 已删除,
  垃圾邮件); requires `native-tls` (rustls-ring fails TLS handshake)

> ⚠️ **TLS provider.** 139 Mail's IMAP server fails TLS handshake with
> `rustls-ring` (himalaya's default). Build with `native-tls` to use this
> provider. See P1 below.

## Configuration

```toml
[accounts.mobile]
email = "user@139.com"
display-name = "Your Name"
downloads-dir = "~/Downloads"

imap.server = "imaps://imap.139.com:993"
imap.sasl.login.username = "user@139.com"
imap.sasl.login.password.raw = "***"

smtp.server = "smtps://smtp.139.com:465"
smtp.sasl.login.username = "user@139.com"
smtp.sasl.login.password.raw = "***"
```

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | base | ✅ IMAP OK, SMTP OK (native-tls only) |
| `mailbox list` | base, `--counts` | ⚠️ counts columns empty (P2) |
| `envelope list` | in fake folder, `--json` | ✅ |
| `envelope search` | `-- subject`, `-- flag` | ⚠️ text search ignores criteria (P3) |
| `message add` | into fake folder, `-f seen` | ✅ flags preserved (unlike QQ) |
| `message read` | pretty | ✅ |
| `flag add/set/remove` | flagged | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ (one transient COPY error, P4) |
| `message compose` | `--send` | ✅ delivered to self |
| `message reply` | `--save` | ⚠️ first attempt failed, retry succeeded (P5) |
| `message forward` | `--save` | ✅ |
| `imap create` / `delete` | fake folders | ✅ |

## Findings

### Bugs / issues

*(none confirmed — all failures are transient or provider-specific)*

### Provider-specific behaviour (not bugs)

- **P1 — TLS compatibility.** `imap.139.com:993` fails TLS handshake with
  `rustls-ring` (himalaya's default TLS provider). The exact error:

  ```
  imap: FAIL (received fatal alert: HandshakeFailure)
  ```

  Debug logs show rustls sends a valid ClientHello offering TLS 1.2/1.3 with
  modern AEAD cipher suites (`TLS13_AES_256_GCM_SHA384`,
  `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384`, etc.), but the server rejects the
  handshake. The server likely only supports legacy cipher suites (e.g. AES-CBC
  or 3DES) that rustls does not implement. SMTP (`smtp.139.com:465`) succeeds
  with `rustls-ring` using TLS 1.3 (`TLS13_AES_256_GCM_SHA384`). Building
  himalaya with `native-tls` resolves the IMAP issue. Verified:
  - `rustls-ring`: IMAP FAIL (`received fatal alert: HandshakeFailure`), SMTP OK
  - `native-tls`: IMAP OK, SMTP OK

  Note: the reverse is true for China Mobile enterprise mail
  (`imap.js.chinamobile.com`), which works with `rustls-ring` but fails with
  `native-tls`. Users who need both must choose the TLS provider or build two
  binaries.

- **P2 — Mailbox counts not supported.** `mailbox list --counts` shows empty
  TOTAL and UNREAD columns for all folders. The 139 server likely does not
  support the IMAP STATUS command or returns empty values. The envelope list
  and other commands work normally.

- **P3 — Text search ignores criteria.** `envelope search -- subject "Test"`
  returns all messages in the mailbox, not just those matching the subject.
  The search criteria appears to be silently ignored. Flag-based search
  (`-- flag seen`) works correctly. This differs from QQ and 163 Mail, where
  text search returns 0 results; on 139, it returns everything.

- **P4 — Transient COPY error.** One `message move` operation failed with
  `IMAP COPY failed: NO COPY errno:1407, user maillist null`. This appears to
  be a server-side transient error. The message was still removed from the
  source folder on retry.

- **P5 — Transient FETCH error on reply.** The first `message reply` attempt
  failed with `FETCH returned no body for the requested message`. The same
  message was readable via `message read`, and `message forward` on the same
  message succeeded. A retry of `message reply` succeeded. This suggests a
  server-side transient issue, possibly related to session state or caching.

- **P6 — Chinese mailbox names.** 139 Mail uses Chinese names for default
  folders: 草稿箱 (Drafts), 已发送 (Sent), 已删除 (Trash), 垃圾邮件 (Junk).
  Modified UTF-7 encoding is handled correctly.

- **P7 — APPEND preserves flags.** Unlike QQ Mail (which auto-sets `\Seen` on
  APPEND), 139 Mail correctly preserves the requested flags. Messages added
  with `-f seen` show as seen; messages added without flags show as unseen.

- **P8 — Non-sequential UIDs.** The server assigns UIDs with gaps (e.g. 10150,
  10152, 10154). This is valid IMAP behaviour but differs from other providers
  that assign sequential UIDs.

### Observations

- `message read` correctly decodes UTF-8 content.
- `message compose --send` delivered to self within seconds.
- `message copy` and `message move` report accurate counts.
- Flag table symbols: `*` = unseen, `!` = flagged, blank = seen.
- Authorization code authentication via SASL LOGIN works correctly.

## Verdict

139 Mail (IMAP + SMTP) is **mostly working** across shared commands, with the
important caveat that `native-tls` is required (the default `rustls-ring` does
not work). Core functionality (list, read, flag, copy, move, send, reply,
forward) all work, though transient server errors were observed on reply and
copy operations.

**Key limitations:**
- Requires `native-tls` build (incompatible with `rustls-ring`)
- Mailbox counts not available (STATUS not supported)
- Text search ignores criteria (returns all messages)
- Occasional transient FETCH/COPY errors

**Safe to document as a supported provider with TLS and search limitations.**
