---
cairn: spec
capability: packaging
status: current
---

# Packaging

Himalaya is an application, the top layer of the Pimalaya stack. It writes no protocol or storage logic of its own: it is a thin shell driving the sans-I/O io-* libraries below it, consuming their blocking `*Std` clients and orchestrating and rendering the results. The CLI plumbing (clap args, printer, logger), TOML config loading and the blocking stream runtime come from pimalaya-cli, pimalaya-config and pimalaya-stream.

### Requirement: Binary only
Himalaya SHALL build as a binary with no public library API and no lib target. A user needing the protocol or storage logic reaches for the io-* library that owns it (io-imap, io-jmap, io-gmail, io-msgraph, io-smtp, io-managesieve, io-maildir, io-mbox, io-pimdir).

### Requirement: Feature-gated backends
Every backend SHALL sit behind its own cargo feature (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`), so a build ships only the protocols it needs. A protocol command and its backend adapter compile only when its feature is on.

### Requirement: TLS provider features
The TLS providers SHALL be cargo features forwarded to pimalaya-stream and every network backend: `rustls-ring` (default), `rustls-aws`, `native-tls`.

### Requirement: Release profile
The binary manifest SHALL carry the shared release profile (`lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`) and omit the library-only manifest fields (the docs.rs metadata block, the documentation field, the no-std category).

### Requirement: Licence
Himalaya SHALL be dual-licensed MIT OR Apache-2.0, with no per-file headers.

### Requirement: The sample ships beside the binary
The Nix package SHALL install config.sample.toml as share/doc/himalaya/config.sample.toml, the release archives SHALL carry it with the share/ of the build they come from, and install.sh SHALL copy it to the same path under its prefix when the downloaded archive holds it, so the first-run welcome copies it rather than downloading it.

### Requirement: Hosts are ASCII
Himalaya SHALL build url's IDNA without a Unicode back end, `idna_adapter` being pinned to 1.0.0 in Cargo.toml. A host SHALL be written in ASCII, an internationalised one in its Punycode form (`xn--…`), and an `http` or `https` URL with a non-ASCII host (a JMAP server, an HTTP proxy) SHALL be refused when parsed. IMAP, SMTP and ManageSieve servers are URLs of schemes url keeps opaque, which never reach IDNA: their host is written in ASCII as well.

#### Scenario: An internationalised JMAP server
- GIVEN `jmap.server = "https://bücher.example/jmap"`
- WHEN the account connects
- THEN the server is refused before any connection, and the same account connects with `jmap.server = "https://xn--bcher-kva.example/jmap"`

### Requirement: Dependencies name their features
Every dependency, build and dev ones included, SHALL be declared with `default-features = false` and the features Himalaya uses, so a default turned on upstream never reaches the binary. A feature only one backend needs SHALL be turned on by that backend's cargo feature: pimalaya-cli's `prompt`, which `pimdir queue cancel` alone asks through, by `pimdir`. A dependency nothing uses SHALL be removed, and a crate the tests alone use SHALL be a dev-dependency.

### Requirement: Dependency checks pass clean
`cargo deny check` SHALL pass over the targets deny.toml lists, which are the release targets and the ones people build from source, allowing only the licences the tree carries. A duplicate the tree cannot avoid stays a visible warning rather than a skip: today getrandom 0.2 (ring 0.17), syn 2 (bounded-static-derive under io-imap and io-smtp, windows-core under chrono on Windows), and hashbrown 0.15 with foldhash 0.1 (chumsky 0.13).

### Requirement: Live provider suites
The binary SHALL carry one ignored integration suite per provider under tests/ (google.rs, microsoft.rs, fastmail.rs), run with `--ignored` and sharing tests/common.rs. Each SHALL hold an `anonymous` module that runs every backend the provider's sample offers on a made-up token and touches no account, its accounts taken from the provider sections of config.sample.toml where one fits so a provider drifting from the sample fails it, and asserting the exit code of the output contract (3 with the `account check` report, 1 with the failure object) and a message naming the refusal in Himalaya's words. It MAY hold a `credentialed` module that mints its own token from `HIMALAYA_`-prefixed variables and only reads (`account check`, `mailbox list`, `envelope list`), failing naming the variables it misses, which CI runs in the upstream repository only and once its secrets exist. A token SHALL reach the binary through a token command reading its environment, never argv nor the configuration, and no assertion SHALL print a token or secret. CI SHALL run each suite in its own job at `RUST_LOG=debug` at most.
