# Migration guide

## From v1 to v2

### Context

The past years with Himalaya CLI v1 bring us the following conclusion:

- The CLI shines the best for scripting.
- The CLI is not convenient to use as your daily mail client. A client at the top of the CLI is definitely required.
- The backend abstraction brings no value to the CLI.
- Every commands re-create a whole IMAP session: TCP connection, TLS negociations, IMAP greeting, authentication, capability.

The v2 learns from that conclusion:

- By ditching the backend abstraction, the CLI API becomes more low-level. Each protocol has its own CLI API.
- As a direct consequence, the CLI becomes less user-friendly:
  - No more wizard to configure your TOML configuration.
  - Less interaction: no prompt, no confirmation, less colors.
  - No more message composition: the v2 only manipulates MIME messages.
- To compensate:
  - Composition moves to [pimalaya/mml](https://github.com/pimalaya/mml), which now contains the lib and the CLI: compile MML message, interpret MIME message, and manage templates.
  - UX efforts moves to a new, dedicated project: Himalaya TUI :tada: (which is in active development at the moment). Both Himalaya CLI and Himalaya TUI are complementary: one is focused on scripting or quick checkup, while is the other one tends to be a mail client for daily usage.
  - IMAP and SMTP sessions can be re-used thanks to a new project [pimalaya/sirup](https://github.com/pimalaya/sirup).

The v2 also removes away some complexity:

- Secrets do not support keyring natively anymore due to many issues with it. Instead use [pimalaya/mimosa](https://github.com/pimalaya/mimosa) or equivalent.
- OAuth is not supported natively anymore. Instead use [pimalaya/ortie](https://github.com/pimalaya/ortie) or equivalent.

### I/O-free

Additionally, Pimalaya has been working for the past year on an adaptation of the [Sans I/O](https://sans-io.readthedocs.io/) pattern for its libraries. It makes libraries not tied up to any sort of I/O: sync vs async, tokio vs async-std vs smol, rustls vs native-tls etc. The concept has been implemented into recent libraries, and has been tested in other CLIs like [pimalaya/ortie](https://github.com/pimalaya/ortie), [pimalaya/cardamum](https://github.com/pimalaya/cardamum) or [pimalaya/calendula](https://github.com/pimalaya/calendula). Himalaya CLI v2 implements these changes. As a direct consequence, it supports out of the box TLS via `native-tls` or via `rustls` (supporting both `aws-lc` and `ring` crypto providers)

### Config changes

It does not make sense to list all changes, since the whole API changed drastically. Better to directly consult the new [config.sample.toml](./config.sample.toml). I would recommend to copy the sample in a new location (e.g., `~/.config/himalaya/config.v2.toml`), adjust options according to your previous configuration, then test it with the argument `-c|--config ~/.config/himalaya/config.v2.toml`.

At global and account levels, only `downloads-dir` remains. All `*.table.preset` are combined into a `table-preset` option. A new option `table-arrangement` has been added with possible values `dynamic`, `dynamic-full-width` and `disabled`.

At account level only, `default` remains as well. Protocols configuration goes into a dedicated option `imap`, `maildir`, `smtp` etc.

At IMAP level (same for SMTP):

- Host + port:

  ```toml
  # v1
  backend.type = "imap"
  backend.host = "localhost"
  backend.port = 993

  # v2
  imap.url = "imaps://localhost:993"
  ```

- Encryption:
  
  ```toml
  # v1
  backend.encryption.type = "none"

  # v2
  imap.url = "imap://host[:port]"
  ```

  ```toml
  backend.encryption.type = "start-tls"

  # v2
  imap.url = "imap://host[:port]"
  imap.starttls = true
  ```

  ```toml
  # v1
  backend.encryption.type = "tls"

  # v2
  imap.url = "imaps://host[:port]"
  ```

- Authentication

  ```toml
  # v1
  backend.auth.type = "password"
  backend.auth.raw = "***"

  # v2
  # authentication becomes closer to SASL
  # more mechanisms will be added in the future
  #
  # SASL PLAIN:
  imap.sasl.plain.authcid = "login"
  imap.sasl.plain.passwd.raw = "***"
  #
  # SASL LOGIN:
  imap.sasl.login.username = "login"
  imap.sasl.login.password.raw = "***"
  #
  # SASL ANONYMOUS:
  imap.sasl.anonymous.message = "anon"
  ```

All the rest is removed, either definitely or moved to dedicated sub-projects.

### CLI changes

Since each protocol has its own CLI, all commands need to be prefixed by the protocol name:

```
# v1
himalaya envelope list

# v2
himalaya imap envelope list
```

#### Folders

List of corresponding commands for IMAP mailboxes:

| v1 | v2 |
|---|---|
| `himalaya folder add` | `himalaya imap mailbox create` |
| `himalaya folder list` | `himalaya imap mailbox list --all` |
| `himalaya folder expunge` | `himalaya imap mailbox expunge --select` |
| `himalaya folder purge` | `himalaya imap mailbox create --select` |
| `himalaya folder delete` | `himalaya imap mailbox delete` |

New commands has been added:

- `himalaya imap mailbox close`: close the current, selected mailbox
- `himalaya imap mailbox rename`: rename the given mailbox
- `himalaya imap mailbox select`: select the given mailbox
- `himalaya imap mailbox status`: get the status of the given mailbox
- `himalaya imap mailbox subscribe`: subscribe to the given mailbox
- `himalaya imap mailbox unselect`: unselect a current, selected mailbox
- `himalaya imap mailbox unsubscribe`: unsubscribe from the given mailbox

Also, some commands don't select by default anymore. It requires the `--select` command. The reason behind is that thanks to [pimalaya/sirup](https://github.com/pimalaya/sirup) it is now possible to re-use IMAP and SMTP sessions. In this case, selection is managed by the user itself.

Finally, the `mailbox list` shows by default subscribed mailboxes. To simulate v1 behaviour you need to pass `-A|--all` flag to see all mailboxes.

#### Flags

List of corresponding commands for IMAP flags:

| v1 | v2 |
|---|---|
| `himalaya flag add -f INBOX 1 2 3 5 seen custom` | `himalaya imap flag add -m INBOX 1:3,5 \\Seen custom` |
| `himalaya flag set -f INBOX 1 2 3 5 seen custom` | `himalaya imap flag set -m INBOX 1:3,5 \\Seen custom` |
| `himalaya flag remove -f INBOX 1 2 3 5 seen custom` | `himalaya imap flag remove -m INBOX 1:3,5 \\Seen custom` |

New command has been added:

- `himalaya imap flags list`: list available IMAP flags for the given mailbox

#### Envelopes

TODO

#### Messages

TODO

