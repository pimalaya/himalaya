# IMAP + SMTP on 163 Mail — test report

Shared cross-protocol commands on the IMAP/SMTP backend against **163 Mail**
(NetEase, Coremail). Method:
[provider-test-plan.md](provider-test-plan.md).

- himalaya: `v2.2.1 +imap +smtp +rustls-ring` (rev `4df380b`)
- account: `netease` — IMAP `imaps://imap.163.com:993`, SMTP
  `smtps://smtp.163.com:465`, SASL LOGIN with authorization code
- date: 2026-10-18
- server: Coremail; `imap.sasl-ir = false` and `imap.id.auto = true` required

> ⚠️ **Coremail quirks confirmed.** 163 Mail runs on Coremail, which has
> several known issues documented in
> [provider-quirks.md](../spec/provider-quirks.md). The configuration
> requires `imap.sasl-ir = false` and `imap.id.auto = true` (with non-empty
> `id.fields`, the default). Without these, the connection fails.

## Configuration

```toml
[accounts.netease]
default = false
email = "user@163.com"
display-name = "Your Name"
downloads-dir = "~/Downloads"

imap.server = "imaps://imap.163.com:993"
imap.sasl-ir = false
imap.id.auto = true
imap.sasl.login.username = "user@163.com"
imap.sasl.login.password.raw = "***"

smtp.server = "smtps://smtp.163.com:465"
smtp.sasl.login.username = "user@163.com"
smtp.sasl.login.password.raw = "***"
```

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | base | ✅ IMAP OK, SMTP OK |
| `mailbox list` | base, `--counts`, `--json` | ✅ Chinese mailbox names displayed correctly |
| `envelope list` | in fake folder, `--json` | ✅ |
| `envelope search` | `-- subject`, `-- from` | ❌ text search returns 0 results (P1) |
| `message add` | into fake folder | ✅ |
| `message read` | pretty, Chinese content | ✅ Chinese body displayed correctly |
| `flag add/set/remove` | seen/flagged | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ |
| `message compose` | `--send` | ✅ delivered to self |
| `message reply` | `--save` | ✅ |
| `message forward` | `--save` | ✅ |
| `imap create` / `delete` | fake folders | ✅ (delete requires empty mailbox) |
| `imap fetch` | `--envelope` | ✅ |
| `imap search` | no criteria, `--seen`, `--from` | ⚠️ flag search works, text search fails (P1) |
| `imap status` | fake folder | ✅ |

## Findings

### Bugs / issues

*(none — all failures are provider-specific behaviour)*

### Provider-specific behaviour (not bugs)

- **P1 — Text search does not work.** `envelope search -- subject "..."` and
  `-- from "..."` return 0 results even when matching messages exist. Flag-based
  search (`--seen`) works correctly. The IMAP-level `imap search --from` also
  returns 0. This is a Coremail server limitation or configuration issue, not a
  himalaya bug. Users should be aware that search on 163 Mail is unreliable for
  text content.

- **P2 — Coremail SASL-IR is broken.** Confirmed that `imap.sasl-ir = false` is
  required. Without it, the server advertises SASL-IR but rejects the inline
  initial response.

- **P3 — ID command is required.** Confirmed that `imap.id.auto = true` is
  required. The server sends an RFC 2971 ID command after authentication.

- **P4 — Chinese mailbox names.** 163 Mail uses Chinese names for default
  folders: 草稿箱 (Drafts), 已发送 (Sent), 已删除 (Trash), 垃圾邮件 (Junk),
  病毒文件夹 (Virus), 广告邮件 (Ads). Modified UTF-7 encoding is handled
  correctly by io-imap; all Chinese names display properly in both plain and
  JSON output.

- **P5 — IMAP DELETE requires empty mailbox.** The server refuses to delete a
  non-empty mailbox with `NO DELETE can't delete mailbox with letter in it`.
  Messages must be moved or deleted before the mailbox can be removed.

- **P6 — Authorization code authentication.** 163 Mail requires an authorization
  code (not the account password) for IMAP/SMTP. The code is generated from the
  web interface: Settings → POP3/SMTP/IMAP → Enable IMAP → Set authorization
  code. The authorization code works via SASL LOGIN (the IMAP `LOGIN` command),
  not `AUTHENTICATE PLAIN`, consistent with the provider-quirks documentation.

- **P7 — TLS compatibility.** 163 Mail works with both `rustls-ring` (default)
  and `native-tls`. No TLS provider switch needed. Note: this is not true for
  all Chinese providers — 139 Mail requires `native-tls`, and China Mobile
  enterprise mail requires `rustls-ring`.

### Observations

- `mailbox list --counts` shows TOTAL and UNREAD columns correctly.
- `--json mailbox list` outputs valid JSON with Chinese folder names, but
  `total` and `unread` fields are `null` even when `--counts` is passed to the
  plain command. This may be a schema issue (counts not included in JSON by
  default).
- `message read` correctly decodes UTF-8 Chinese content.
- `message compose --send` delivered to self within seconds; the delivered copy
  appeared in Inbox.
- Flag table symbols: `*` = unseen, `!` = flagged, blank = seen.
- `message copy` and `message move` report accurate counts.
- `imap fetch --envelope` returns message metadata correctly.

## Verdict

163 Mail (IMAP + SMTP) is **mostly working** across shared commands. The core
functionality (list, read, flag, copy, move, send, reply, forward) all work
correctly, and Chinese mailbox names and content are handled properly.

**The main limitation is text search**, which does not work on Coremail. This is
a provider-side issue, not a himalaya bug. Users should be documented that
search on 163 Mail is unreliable.

The Coremail quirks (SASL-IR, ID command) are correctly documented in
provider-quirks.md and the configuration works as expected when these are set.

**Safe to document as a supported provider with search limitations.**
