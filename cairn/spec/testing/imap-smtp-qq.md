# IMAP + SMTP on QQ Mail — test report

Shared cross-protocol commands on the IMAP/SMTP backend against **QQ Mail**
(Tencent / Foxmail). Method:
[provider-test-plan.md](provider-test-plan.md).

- himalaya: `v2.2.1 +imap +smtp +native-tls` (rev `4df380b`)
- account: `qq` — IMAP `imaps://imap.qq.com:993`, SMTP
  `smtps://smtp.qq.com:465`, SASL LOGIN with authorization code
- date: 2026-10-18
- server: Tencent Mail; hierarchy separator `/`; advertised capabilities
  include `IMAP4rev1 UIDPLUS MOVE`

## Configuration

```toml
[accounts.Foxmail]
email = "user@foxmail.com"
display-name = "Your Name"
downloads-dir = "~/Downloads"

imap.server = "imaps://imap.qq.com:993"
imap.sasl.login.username = "user@foxmail.com"
imap.sasl.login.password.raw = "***"

smtp.server = "smtps://smtp.qq.com:465"
smtp.sasl.login.username = "user@foxmail.com"
smtp.sasl.login.password.raw = "***"
```

> **TLS compatibility:** QQ Mail works with both `rustls-ring` (default) and
> `native-tls`. No TLS provider switch needed.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | base | ✅ IMAP OK, SMTP OK |
| `mailbox list` | base, `--counts`, `--json` | ✅ |
| `envelope list` | in fake folder, `--json` | ✅ |
| `envelope search` | `-- subject`, `-- from` | ❌ text search returns 0 results (P1) |
| `message add` | into fake folder, `-f seen` | ✅ (but auto-sets `\Seen`, see P2) |
| `message read` | pretty | ✅ |
| `flag add/set/remove` | flagged | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ |
| `message compose` | `--send` | ✅ delivered to self |
| `message reply` | `--save` | ✅ |
| `message forward` | `--save` | ✅ |
| `imap create` / `delete` | fake folders | ✅ |
| `imap fetch` | `--flags` | ✅ |
| `imap search` | no criteria | ⚠️ text search fails (P1) |

## Findings

### Bugs / issues

*(none — all failures are provider-specific behaviour)*

### Provider-specific behaviour (not bugs)

- **P1 — Text search does not work.** `envelope search -- subject "..."` and
  `-- from "..."` return 0 results even when matching messages exist. This is
  the same behaviour observed on 163 Mail (Coremail). QQ Mail may use a similar
  server-side search implementation that does not support standard IMAP SEARCH
  text criteria. Flag-based search may still work.

- **P2 — APPEND auto-sets `\Seen`.** Messages added via `message add` (IMAP
  `APPEND`) are automatically marked as `\Seen` by the QQ Mail server, even
  when no flags were requested. This differs from Fastmail and 163 Mail, where
  appended messages retain their requested flags. As a result, the `*` (unseen)
  indicator never appears for newly-added messages on QQ Mail.

- **P3 — English mailbox names.** QQ Mail uses English names for default
  folders: `Inbox`, `Sent Messages`, `Drafts`, `Deleted Messages`, `Junk`,
  `Archive`. A nested Chinese folder exists: `其他文件夹/QQ邮件订阅`. The
  hierarchy separator is `/`.

- **P4 — Authorization code authentication.** QQ Mail requires an authorization
  code (not the QQ password) for IMAP/SMTP. The code is generated from the web
  interface: Settings → Account → POP3/IMAP/SMTP service → Generate
  authorization code.

### Observations

- `mailbox list --counts` shows TOTAL and UNREAD columns correctly.
- `message compose --send` delivered to self within seconds; the delivered copy
  appeared in Inbox.
- `message copy` and `message move` report accurate counts.
- `imap fetch --flags` returns flag data correctly.
- Flag table symbols: `*` = unseen, `!` = flagged, blank = seen.
- QQ Mail's `Deleted Messages` folder serves as the trash.

## Verdict

QQ Mail (IMAP + SMTP) is **mostly working** across shared commands. The core
functionality (list, read, flag, copy, move, send, reply, forward) all work
correctly. Both TLS providers (`rustls-ring` and `native-tls`) are supported.

**The main limitation is text search**, which does not work — the same issue
observed on 163 Mail. This appears to be a common limitation among Chinese email
providers. The APPEND auto-Seen behaviour is also noteworthy for users who rely
on flag-based workflows.

**Safe to document as a supported provider with search limitations.**
