//! # Parser
//!
//! Top-level clap parser and subcommand dispatcher, running each command
//! over the [`Session`] client it needs, and the account resolution and
//! first-run welcome every command starts from.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, ConfigFlags, JsonFlag, LogFlags},
        commands::{
            completion::CompletionCommand, json_schema::JsonSchemaCommand, manual::ManualCommand,
        },
    },
    footer, long_version,
    mcp::exit_on_failure,
    printer::Printer,
    welcome::{Welcome, is_wanted},
};
use pimalaya_config::toml::{ConfigMissingError, TomlConfig};

#[cfg(feature = "gmail")]
use crate::gmail::{cli::GmailCommand, client::build_gmail_client};
#[cfg(feature = "imap")]
use crate::imap::{cli::ImapCommand, client::build_imap_client};
#[cfg(feature = "jmap")]
use crate::jmap::{cli::JmapCommand, client::build_jmap_client};
#[cfg(feature = "m2dir")]
use crate::m2dir::{cli::M2dirCommand, client::build_m2dir_client};
#[cfg(feature = "maildir")]
use crate::maildir::{cli::MaildirCommand, client::build_maildir_client};
#[cfg(feature = "mbox")]
use crate::mbox::{cli::MboxCommand, client::build_mbox_client};
#[cfg(feature = "msgraph")]
use crate::msgraph::{cli::MsgraphCommand, client::build_msgraph_client};
#[cfg(feature = "pimdir")]
use crate::pimdir::{cli::PimdirCommand, client::build_pimdir_client};
#[cfg(any(feature = "jmap", feature = "gmail"))]
use crate::shared::vacation::cli::VacationClient;
#[cfg(any(backend, feature = "smtp"))]
use crate::shared::{
    attachment::cli::AttachmentCommand, envelope::cli::EnvelopeCommand, flag::cli::FlagCommand,
    mailbox::cli::MailboxCommand, vacation::cli::VacationCommand,
};
// NOTE: `EmailClient` and the `message` command host the send path, so
// they exist for any backend rather than for storage ones alone.
#[cfg(any(backend, feature = "smtp"))]
use crate::shared::{client::EmailClient, message::cli::MessageCommand};
#[cfg(feature = "sieve")]
use crate::sieve::{cli::SieveCommand, client::build_sieve_client};
#[cfg(feature = "smtp")]
use crate::smtp::{cli::SmtpCommand, client::build_smtp_client};
use crate::{
    account::cli::AccountCommand,
    backend::Backend,
    config::{AccountConfig, Config},
    error::ErrorCode,
    json_schema,
    mcp::McpCommand,
    repl::ReplCommand,
    session::Session,
};
#[cfg(any(backend, feature = "smtp"))]
use crate::{
    account::context::Account,
    capability::{self, Capability, Resolution},
};

/// The sample configuration as a raw file, for `curl` to fetch.
const SAMPLE_RAW_URL: &str =
    "https://raw.githubusercontent.com/pimalaya/himalaya/master/config.sample.toml";

/// Top-level command-line interface parser.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(after_help = footer!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    /// The subcommand to run, a bare `himalaya` showing this help.
    #[command(subcommand)]
    pub cmd: Option<Command>,
    #[command(flatten)]
    pub config: ConfigFlags,
    #[command(flatten)]
    pub account: AccountFlag,
    /// Force a specific backend for cross-protocol commands.
    ///
    /// Only the shared commands read it, the protocol-specific ones always
    /// using their own backend. `auto`, the default, picks the first
    /// configured backend the command supports; an explicit value bails when
    /// the account has no such block, or the command no such implementation.
    #[arg(short, long, global = true, default_value_t)]
    pub backend: Backend,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

/// Top-level subcommands.
///
/// NOTE: parsed once per run, so the size of its largest variant, which
/// depends on the features built, costs nothing worth a box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum Command {
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, alias = "mailboxes")]
    Mailbox(MailboxCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, alias = "envelopes")]
    Envelope(EnvelopeCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, alias = "flags")]
    Flag(FlagCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, visible_alias = "msg", alias = "messages")]
    Message(MessageCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, alias = "attachments")]
    Attachment(AttachmentCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand)]
    Vacation(VacationCommand),
    #[cfg(feature = "imap")]
    #[command(subcommand)]
    Imap(ImapCommand),
    #[cfg(feature = "jmap")]
    #[command(subcommand)]
    Jmap(JmapCommand),
    #[cfg(feature = "gmail")]
    #[command(subcommand)]
    Gmail(GmailCommand),
    #[cfg(feature = "msgraph")]
    #[command(subcommand)]
    Msgraph(MsgraphCommand),
    #[cfg(feature = "maildir")]
    #[command(subcommand)]
    Maildir(MaildirCommand),
    #[cfg(feature = "m2dir")]
    #[command(subcommand)]
    M2dir(M2dirCommand),
    #[cfg(feature = "mbox")]
    #[command(subcommand)]
    Mbox(MboxCommand),
    #[cfg(feature = "pimdir")]
    #[command(subcommand)]
    Pimdir(PimdirCommand),
    #[cfg(feature = "smtp")]
    #[command(subcommand)]
    Smtp(SmtpCommand),
    #[cfg(feature = "sieve")]
    #[command(subcommand)]
    Sieve(SieveCommand),
    Repl(ReplCommand),
    Mcp(McpCommand),
    #[command(subcommand)]
    Account(AccountCommand),
    #[command(alias = "completions")]
    Completion(CompletionCommand),
    #[command(alias = "manuals")]
    Manual(ManualCommand),
    #[command(alias = "json-schemas")]
    JsonSchema(JsonSchemaCommand),
}

/// Resolves the account a command runs against, returning the leftover
/// global config, the account name and its config.
///
/// A missing configuration fails with a [`ConfigMissingError`] naming the
/// path looked at, which is where `-c` pointed, so a mistyped path shows
/// up as itself and `main` can meet a person with the welcome.
pub fn resolve_account(
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Config, String, AccountConfig)> {
    let mut config = Config::load(config_paths)?;
    let (name, account_config) = config.take_account(account_name)?;
    Ok((config, name, account_config))
}

/// Meets a bare `himalaya`: the help when a configuration exists, the
/// missing configuration otherwise, which a person meets as the welcome.
pub fn meet_bare_invocation(config_paths: &[PathBuf]) -> Result<()> {
    // NOTE: a broken file counts as a configuration, its error left to
    // the first command reading it.
    if let Err(err) = Config::load(config_paths)
        && err.is::<ConfigMissingError>()
    {
        return Err(err);
    }

    Cli::command().print_help()?;
    Ok(())
}

/// The welcome a person meets when no configuration exists at `path`:
/// the commands creating it from `sample`, the shipped sample
/// configuration, or downloading it without one, then `envelope list`.
pub fn welcome<'a>(
    path: &'a Path,
    config_paths: &'a [PathBuf],
    sample: Option<&'a Path>,
) -> Welcome<'a> {
    Welcome {
        bin: "himalaya",
        intro: "Welcome to Himalaya, the CLI to manage emails.",
        path,
        config_paths,
        sample,
        sample_url: SAMPLE_RAW_URL,
        next: "Edit it and uncomment the options you need, then list your envelopes:",
        command: "envelope list",
    }
}

/// Runs a shared command over the session's email client, refusing it
/// first when the account does not serve what it reaches.
///
/// Checked against the kept client's resolution, or against the
/// account's blocks before a new client connects, so a refusal costs no
/// connection and drops no kept client. An account configuring no
/// transport at all is left to the client's own error, which names the
/// v1 configuration when that is the cause.
#[cfg(any(backend, feature = "smtp"))]
fn run_shared<P: Printer>(
    printer: &mut P,
    session: &mut Session,
    capabilities: Vec<Capability>,
    command: impl FnOnce(&mut P, &mut Account, &mut EmailClient) -> Result<()>,
) -> Result<()> {
    let account = session.account;
    let backend = account.backend;

    if let Some((_, client)) = &session.email {
        client.require(&capabilities)?;
    }

    let build = |config, name: String, account_config| {
        let resolution = Resolution::of(&name, &account_config, backend);
        if resolution.storage.is_some() || resolution.sender.is_some() {
            resolution.require(&capabilities)?;
        }
        EmailClient::new(config, &name, account_config, backend)
    };

    account.run(printer, &mut session.email, build, command)
}

/// Runs a `vacation` command over the client of the backend keeping the
/// auto-reply: the first configured one `--backend` allows that serves it,
/// whatever backend stores the mail.
///
/// The account is read to resolve it, nothing connecting, so a refusal
/// costs no connection; the client is the session's protocol one, kept
/// for the commands after it.
#[cfg(any(backend, feature = "smtp"))]
#[cfg_attr(not(any(feature = "jmap", feature = "gmail")), allow(unused_variables))]
fn run_vacation<P: Printer>(
    printer: &mut P,
    session: &mut Session,
    command: VacationCommand,
) -> Result<()> {
    let account = session.account;
    let (_, name, account_config) = resolve_account(account.config_paths, account.name)?;
    let resolution = Resolution::of(&name, &account_config, account.backend);
    resolution.require(&[Capability::Vacation])?;

    match resolution.transport(Capability::Vacation) {
        #[cfg(feature = "jmap")]
        Some(Backend::Jmap) => account.run(
            printer,
            &mut session.jmap,
            build_jmap_client,
            |printer, account, client| {
                command.execute(printer, account, VacationClient::Jmap(client))
            },
        ),
        #[cfg(feature = "gmail")]
        Some(Backend::Gmail) => account.run(
            printer,
            &mut session.gmail,
            build_gmail_client,
            |printer, account, client| {
                command.execute(printer, account, VacationClient::Gmail(client))
            },
        ),
        // NOTE: the resolver let through a backend serving it, every one of
        // which has its arm above.
        _ => Err(capability::missing(format!(
            "Account `{name}` cannot `vacation`: this build keeps no client for it"
        ))),
    }
}

impl Command {
    /// Runs the subcommand over the session's client for it, built on
    /// first use against the account `-a` names, or the default one.
    pub fn execute(self, printer: &mut impl Printer, session: &mut Session) -> Result<()> {
        let account = session.account;

        match self {
            #[cfg(any(backend, feature = "smtp"))]
            Self::Mailbox(cmd) => run_shared(
                printer,
                session,
                cmd.capabilities(),
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(any(backend, feature = "smtp"))]
            Self::Envelope(cmd) => run_shared(
                printer,
                session,
                cmd.capabilities()?,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(any(backend, feature = "smtp"))]
            Self::Flag(cmd) => run_shared(
                printer,
                session,
                cmd.capabilities(),
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            // NOTE: `message parse` reads a raw message it is given, so it
            // resolves no account and reads no configuration at all.
            #[cfg(any(backend, feature = "smtp"))]
            Self::Message(MessageCommand::Parse(cmd)) => cmd.execute(printer),
            #[cfg(any(backend, feature = "smtp"))]
            Self::Message(cmd) => run_shared(
                printer,
                session,
                cmd.capabilities(),
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(any(backend, feature = "smtp"))]
            Self::Attachment(cmd) => run_shared(
                printer,
                session,
                vec![Capability::AttachmentList],
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(any(backend, feature = "smtp"))]
            Self::Vacation(cmd) => run_vacation(printer, session, cmd),

            #[cfg(feature = "imap")]
            Self::Imap(cmd) => account.run(
                printer,
                &mut session.imap,
                build_imap_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "jmap")]
            Self::Jmap(cmd) => account.run(
                printer,
                &mut session.jmap,
                build_jmap_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "gmail")]
            Self::Gmail(cmd) => account.run(
                printer,
                &mut session.gmail,
                build_gmail_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "msgraph")]
            Self::Msgraph(cmd) => account.run(
                printer,
                &mut session.msgraph,
                build_msgraph_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => account.run(
                printer,
                &mut session.maildir,
                build_maildir_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "m2dir")]
            Self::M2dir(cmd) => account.run(
                printer,
                &mut session.m2dir,
                build_m2dir_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "mbox")]
            Self::Mbox(cmd) => account.run(
                printer,
                &mut session.mbox,
                build_mbox_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "pimdir")]
            Self::Pimdir(cmd) => account.run(
                printer,
                &mut session.pimdir,
                build_pimdir_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),
            #[cfg(feature = "smtp")]
            Self::Smtp(cmd) => account.run(
                printer,
                &mut session.smtp,
                build_smtp_client,
                |printer, _account, client| cmd.execute(printer, client),
            ),
            #[cfg(feature = "sieve")]
            Self::Sieve(cmd) => account.run(
                printer,
                &mut session.sieve,
                build_sieve_client,
                |printer, account, client| cmd.execute(printer, account, client),
            ),

            Self::Repl(cmd) => cmd.execute(printer, session),
            Self::Mcp(cmd) => {
                // stdout carries protocol messages only, so a failure goes
                // to stderr, a person missing a configuration aside.
                match cmd.execute(printer, session) {
                    Err(err) if err.is::<ConfigMissingError>() && is_wanted(&*printer) => Err(err),
                    result => {
                        exit_on_failure::<ErrorCode, _>(result);
                        Ok(())
                    }
                }
            }
            Self::Account(cmd) => {
                cmd.execute(printer, account.config_paths, account.name, account.backend)
            }
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
            Self::JsonSchema(cmd) => cmd.execute(printer, json_schema::registry()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf, process};

    use clap::Parser;
    use pimalaya_config::toml::{AccountError, ConfigMissingError};
    use tempfile::TempDir;

    use crate::cli::{Cli, resolve_account};

    /// A directory that is never created, so the paths below it keep the
    /// spelling they were given instead of being canonicalized.
    fn absent_dir() -> PathBuf {
        env::temp_dir().join(format!("himalaya-config-paths-{}", process::id()))
    }

    fn paths(args: &[&str]) -> Vec<PathBuf> {
        Cli::try_parse_from([&["himalaya"], args].concat())
            .expect("parse")
            .config
            .paths
    }

    /// Writes `toml` as the only configuration file of a fresh directory.
    fn config(toml: &str) -> (TempDir, Vec<PathBuf>) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, toml).unwrap();
        (dir, vec![path])
    }

    #[test]
    fn a_config_path_is_never_split() {
        // NOTE: an absolute Windows path holds a colon of its own, so splitting
        // on it would read `C:\dir\config.toml` as two paths and find neither.
        let path = absent_dir().join("config.toml");

        assert_eq!(
            paths(&["-c", path.to_str().unwrap(), "account", "list"]),
            [path]
        );
    }

    /// A value is one path whatever it holds, `$PATH`'s delimiters
    /// included; several paths take one `-c` each.
    #[test]
    fn a_delimited_config_value_stays_one_path() {
        let dir = absent_dir();
        let given = dir.join("one.toml:two.toml;three.toml");
        let args = ["-c", given.to_str().unwrap(), "account", "list"];
        assert_eq!(paths(&args), [given]);

        let (one, two) = (dir.join("one.toml"), dir.join("two.toml"));
        let (one_str, two_str) = (one.to_str().unwrap(), two.to_str().unwrap());
        let args = ["-c", one_str, "-c", two_str, "account", "list"];
        assert_eq!(paths(&args), [one, two]);
    }

    /// `main` meets a person with the welcome by downcasting this error,
    /// so it has to come typed, naming the path `-c` gave.
    #[test]
    fn a_missing_configuration_is_typed_for_the_welcome() {
        let paths = [absent_dir().join("config.toml")];
        let err = resolve_account(&paths, None).unwrap_err();
        let missing = err.downcast_ref::<ConfigMissingError>().unwrap();
        assert_eq!(missing.path, paths[0]);
    }

    #[test]
    fn an_unknown_account_lists_the_held_ones() {
        let (_dir, paths) = config("[accounts.work]\n[accounts.perso]\n");
        let err = resolve_account(&paths, Some("nope")).unwrap_err();

        match err.downcast_ref::<AccountError>() {
            Some(AccountError::Missing { name, names }) => {
                assert_eq!(name, "nope");
                assert_eq!(names, &["perso", "work"]);
            }
            _ => panic!("{err:?}"),
        }

        let message = err.to_string();
        assert!(message.ends_with("holds: perso, work"), "{message}");
    }

    /// The config layers make the case easy to reach, an rc file marking
    /// one account the default while the platform file marks another.
    #[test]
    fn several_default_accounts_are_refused_naming_them() {
        let account = |name: &str| format!("[accounts.{name}]\ndefault = true\n");
        let (_dir, paths) = config(&(account("work") + &account("perso")));
        let err = resolve_account(&paths, None).unwrap_err();

        match err.downcast_ref::<AccountError>() {
            Some(AccountError::SeveralDefaults(names)) => assert_eq!(names, &["perso", "work"]),
            _ => panic!("{err:?}"),
        }

        let (_, name, _) = resolve_account(&paths, Some("work")).unwrap();
        assert_eq!(name, "work");
    }
}
