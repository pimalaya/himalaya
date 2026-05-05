use std::{fmt, path::PathBuf, str::FromStr};

use anyhow::{bail, Error, Result};
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
        parsers::path_parser,
    },
    long_version,
    printer::Printer,
};
use pimalaya_config::toml::TomlConfig;

#[cfg(feature = "imap")]
use crate::imap::cli::ImapCommand;
#[cfg(feature = "jmap")]
use crate::jmap::cli::JmapCommand;
#[cfg(feature = "maildir")]
use crate::maildir::cli::MaildirCommand;
#[cfg(feature = "smtp")]
use crate::smtp::cli::SmtpCommand;
use crate::{
    account::Account, config::Config, envelopes::cli::EnvelopesCommand, flags::cli::FlagsCommand,
    mailboxes::cli::MailboxesCommand, messages::cli::MessagesCommand,
};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct HimalayaCli {
    #[command(subcommand)]
    pub command: BackendCommand,

    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). If the first path does not point to a valid file,
    /// the wizard will propose to assist you in the creation of the
    /// configuration file. Other paths are merged with the first one,
    /// which allows you to separate your public config from your
    /// private(s) one(s).
    /// you can also provide multiple paths by delimiting them with a :
    /// like you would when setting $PATH in a posix shell
    #[arg(short, long = "config", global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser, value_delimiter = ':')]
    pub config_paths: Vec<PathBuf>,
    #[command(flatten)]
    pub account: AccountFlag,

    /// Force a specific backend for cross-protocol commands.
    ///
    /// Only consumed by the shared commands (`mailboxes`, `envelopes`,
    /// `flags`, `messages`); the protocol-specific subcommands
    /// (`imap`, `jmap`, `maildir`, `smtp`) ignore it and always use
    /// their own backend.
    ///
    /// Possible values: `auto` (default), `imap`, `jmap`, `maildir`,
    /// `smtp`. With `auto`, the shared command picks the first
    /// configured backend it supports; with an explicit value, it uses
    /// only that backend (and bails if the account has no matching
    /// config block, or if the operation has no implementation for it
    /// — e.g. `--backend smtp mailboxes list`).
    #[arg(short, long, global = true, default_value_t)]
    pub backend: BackendArg,

    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

/// Selects which backend a cross-protocol command should target.
///
/// `Auto` lets the command pick the first configured-and-supported
/// backend in its own priority order. The named variants pin the
/// command to that backend; the command bails if it cannot be served
/// (config missing, or the operation has no arm for that backend).
///
/// The protocol-specific subcommands (`imap`, `jmap`, `maildir`,
/// `smtp`) ignore this arg entirely.
#[derive(Clone, Copy, Debug, Default, Parser, PartialEq, Eq)]
pub enum BackendArg {
    #[default]
    Auto,
    Imap,
    Jmap,
    Maildir,
    Smtp,
}

impl BackendArg {
    /// Whether the IMAP arm of a shared command is allowed to run.
    pub fn allows_imap(self) -> bool {
        matches!(self, Self::Auto | Self::Imap)
    }

    /// Whether the JMAP arm of a shared command is allowed to run.
    pub fn allows_jmap(self) -> bool {
        matches!(self, Self::Auto | Self::Jmap)
    }

    /// Whether the Maildir arm of a shared command is allowed to run.
    pub fn allows_maildir(self) -> bool {
        matches!(self, Self::Auto | Self::Maildir)
    }

    /// Whether the SMTP arm of a shared command is allowed to run.
    pub fn allows_smtp(self) -> bool {
        matches!(self, Self::Auto | Self::Smtp)
    }
}

impl FromStr for BackendArg {
    type Err = Error;

    fn from_str(backend: &str) -> Result<Self, Self::Err> {
        match backend {
            "auto" => Ok(Self::Auto),
            "imap" => Ok(Self::Imap),
            "jmap" => Ok(Self::Jmap),
            "maildir" => Ok(Self::Maildir),
            "smtp" => Ok(Self::Smtp),
            backend => bail!("Invalid backend {backend}"),
        }
    }
}
impl fmt::Display for BackendArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Imap => write!(f, "imap"),
            Self::Jmap => write!(f, "jmap"),
            Self::Maildir => write!(f, "maildir"),
            Self::Smtp => write!(f, "smtp"),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum BackendCommand {
    Manuals(ManualCommand),
    Completions(CompletionCommand),

    #[command(subcommand)]
    Mailboxes(MailboxesCommand),
    #[command(subcommand)]
    Envelopes(EnvelopesCommand),
    #[command(subcommand)]
    Flags(FlagsCommand),
    #[command(subcommand)]
    Messages(MessagesCommand),
    #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
    #[command(subcommand)]
    Attachments(crate::attachments::cli::AttachmentsCommand),

    #[cfg(feature = "imap")]
    #[command(subcommand)]
    Imap(ImapCommand),
    #[cfg(feature = "jmap")]
    #[command(subcommand)]
    Jmap(JmapCommand),
    #[cfg(feature = "maildir")]
    #[command(subcommand)]
    Maildir(MaildirCommand),
    #[cfg(feature = "smtp")]
    #[command(subcommand)]
    Smtp(SmtpCommand),
}

impl BackendCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: BackendArg,
    ) -> Result<()> {
        match self {
            Self::Manuals(cmd) => cmd.execute(printer, HimalayaCli::command()),
            Self::Completions(cmd) => cmd.execute(printer, HimalayaCli::command()),

            Self::Mailboxes(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                cmd.execute(printer, config, account_config, backend)
            }
            Self::Envelopes(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                cmd.execute(printer, config, account_config, backend)
            }
            Self::Flags(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                cmd.execute(printer, config, account_config, backend)
            }
            Self::Messages(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                cmd.execute(printer, config, account_config, backend)
            }
            #[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
            Self::Attachments(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                cmd.execute(printer, config, account_config, backend)
            }

            #[cfg(feature = "imap")]
            Self::Imap(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (account_name, mut account_config) = config.get_account(account_name)?;

                let Some(imap_config) = account_config.imap.take() else {
                    bail!("IMAP config is missing for account `{account_name}`")
                };

                let account = Account::new(config, account_config, imap_config)?;

                cmd.execute(printer, account)
            }
            #[cfg(feature = "jmap")]
            Self::Jmap(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (account_name, mut account_config) = config.get_account(account_name)?;

                let Some(jmap_config) = account_config.jmap.take() else {
                    bail!("JMAP config is missing for account `{account_name}`")
                };

                let account = Account::new(config, account_config, jmap_config)?;

                cmd.execute(printer, account)
            }
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (account_name, mut account_config) = config.get_account(account_name)?;

                let Some(maildir_config) = account_config.maildir.take() else {
                    bail!("Maildir config is missing for account `{account_name}`")
                };

                let account = Account::new(config, account_config, maildir_config)?;

                cmd.execute(printer, account)
            }
            #[cfg(feature = "smtp")]
            Self::Smtp(cmd) => {
                let config = load_or_wizard(config_paths)?;
                let (account_name, mut account_config) = config.get_account(account_name)?;

                let Some(smtp_config) = account_config.smtp.take() else {
                    bail!("SMTP config is missing for account `{account_name}`")
                };

                let account = Account::new(config, account_config, smtp_config)?;

                cmd.execute(printer, account)
            }
        }
    }
}

/// Loads `Config` from `paths`, or runs the wizard if no config file
/// is found. Centralises the `Result<Option<Config>>` → `Config`
/// adaptation so call sites stay readable.
fn load_or_wizard(paths: &[PathBuf]) -> Result<Config> {
    match Config::from_paths_or_default(paths)? {
        Some(config) => Ok(config),
        None => crate::wizard::run_or_exit(&Config::target_path(paths)?),
    }
}
