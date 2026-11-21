//! # Himalaya
//!
//! A CLI to manage emails, and the crate architecture (header-001): the
//! behavioural contract lives in cairn/spec/, the manual provider test
//! reports in cairn/spec/testing/, the history in cairn/log/, the unified
//! conventions in the [Pimalaya ARCHITECTURE][pimalaya].
//!
//! [pimalaya]: https://github.com/pimalaya/.github/blob/master/ARCHITECTURE.md
//!
//! ## Place in the stack
//!
//! The top of the Pimalaya stack: no protocol or storage logic of its own,
//! a thin shell over the blocking `*Std` clients of the io-* libraries. The
//! library exists for himalaya-tui alone, which pins it by git revision: no
//! semver promise beyond the CLI grammar and its `--json` output.
//!
//! ## Backends and plumbing
//!
//! The network backends are io-imap, io-jmap, io-gmail, io-msgraph,
//! io-smtp and io-managesieve, the local ones io-maildir, io-mbox and
//! io-pimdir.
//!
//! The CLI plumbing, TOML loading and blocking stream runtime come from
//! pimalaya-cli, pimalaya-config and pimalaya-stream. Every backend sits
//! behind its own cargo feature, so a build ships only the protocols it
//! needs.
//!
//! ## Command families
//!
//! The command tree ([`cli`], `Command`) splits in three. The unified API
//! ([`unified`]) is the cross-protocol surface, the union of what the
//! backends serve, each verb meaning one thing wherever it runs. The crate
//! root holds the code every family shares.
//!
//! The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir, mbox,
//! pimdir, smtp, sieve) each expose one backend's full surface, including
//! what the unified API cannot model. The meta commands cover accounts,
//! sessions (repl, mcp), completions, man pages and JSON Schemas.
//!
//! ## Unified commands and backend selection
//!
//! The unified commands run over a [`unified::client`] `EmailClient`: the
//! first configured storage backend `--backend` allows, local before
//! network, plus SMTP for those that cannot send. Its resolver picks them
//! and refuses a verb the backend does not serve before connecting.
//!
//! The [`unified::capability`] tables also write each unified command's
//! `--help` footer and filter the MCP tool list, so all three agree.
//!
//! Each unified method matches the active backend and calls its
//! per-protocol adapter, which converts io-* results into the CLI's own
//! [`unified::email`] types. The active [`account`] context is threaded as
//! a sibling argument through every `execute` chain.
//!
//! ## Protocol-specific commands
//!
//! Each protocol module builds its client via a `build_<proto>_client`
//! helper and a `<Proto>Client` wrapper that derefs onto the io-* `*Std`
//! client, ignoring `--backend`. Subcommands are clap-derived structs
//! with an `execute` method the module's command enum dispatches to.
//!
//! The imap command mirrors IMAP's flat command list, gmail and msgraph
//! track their REST resource domains one-to-one, and the filesystem
//! backends expose only what maps to their on-disk layout, leaving MIME
//! rendering to the unified commands.
//!
//! ## Sessions
//!
//! Every command runs over a [`session`] `Session`, which builds a client on
//! first use, keeps it, and drops it when a command fails on it. The
//! [`repl`] and the [`mcp`] server keep one session across their calls, so
//! each client connects once.
//!
//! Both run on pimalaya-cli's shared core, which claims stdin, so a command
//! reading its input there is refused under `stdin-taken`.
//!
//! ## Configuration and output
//!
//! pimalaya-config loads every existing canonical path (the rc file last),
//! or the `-c` paths, later ones deep-merged on top, and picks the `-a`
//! account or the default one. The schema ([`config`]) is multi-account:
//! a top-level block plus account blocks with per-backend sub-blocks.
//!
//! Himalaya never writes a configuration on its own. A person running it
//! without one meets the welcome ([`cli::welcome`]), which creates it from
//! the shipped sample or downloads it; a script or a JSON caller gets the
//! error instead.
//!
//! Data and errors go to stdout through the printer (`--json` switching to
//! JSON), logs, prompts and the MCP server's failures to stderr. Text is
//! plain, with no border, colour or terminal sizing: lists in pimalaya-cli's
//! listing frame, records as aligned `Label: value` lines.
//!
//! Each command's doc comment is its `--help`, so `himalaya <command>
//! --help` is the canonical usage reference.

#![forbid(unsafe_code)]

pub mod account;
#[cfg(any(backend, feature = "smtp"))]
pub mod base64;
pub mod cli;
pub mod config;
#[cfg(any(backend, feature = "smtp", feature = "sieve"))]
pub mod crlf;
pub mod error;
#[cfg(feature = "gmail")]
pub mod gmail;
#[cfg(all(test, any(feature = "gmail", feature = "jmap", feature = "msgraph")))]
pub mod http_script;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "jmap")]
pub mod jmap;
pub mod json_schema;
pub mod listing;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "mbox")]
pub mod mbox;
pub mod mcp;
#[cfg(feature = "msgraph")]
pub mod msgraph;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(feature = "pimdir")]
pub mod pimdir;
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
pub mod proxy;
#[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
pub mod raw;
#[cfg(any(feature = "gmail", feature = "jmap", feature = "msgraph"))]
pub mod raw_json;
#[cfg(any(
    feature = "gmail",
    feature = "msgraph",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "sieve"
))]
pub mod receipt;
#[cfg(any(feature = "gmail", feature = "jmap", feature = "msgraph"))]
pub mod refusal;
pub mod repl;
#[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
pub mod sasl;
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "sieve"
))]
pub mod server;
pub mod session;
#[cfg(feature = "sieve")]
pub mod sieve;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(any(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "sieve"
))]
pub mod tls;
pub mod unified;
