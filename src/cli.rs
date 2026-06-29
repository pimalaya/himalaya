use std::{path::PathBuf, process::exit};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, ConfigFlags, JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
    },
    long_version,
    printer::Printer,
};
use pimalaya_config::toml::TomlConfig;

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
#[cfg(feature = "msgraph")]
use crate::msgraph::{cli::MsgraphCommand, client::build_msgraph_client};
#[cfg(feature = "smtp")]
use crate::smtp::{cli::SmtpCommand, client::build_smtp_client};
use crate::{
    account::cli::AccountCommand,
    backend::Backend,
    config::Config,
    shared::{
        attachment::cli::AttachmentCommand, client::EmailClient, envelope::cli::EnvelopeCommand,
        flag::cli::FlagCommand, mailbox::cli::MailboxCommand, message::cli::MessageCommand,
    },
    wizard,
};

/// Top-level command-line interface parser.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,

    #[command(flatten)]
    pub config: ConfigFlags,
    #[command(flatten)]
    pub account: AccountFlag,
    /// Force a specific backend for cross-protocol commands.
    ///
    /// Only consumed by the shared commands (`mailboxes`, `envelopes`,
    /// `flags`, `messages`); the protocol-specific subcommands ignore it
    /// and always use their own backend. With `auto` (the default) the
    /// shared command picks the first configured backend it supports;
    /// with an explicit value it uses only that backend, and bails if
    /// the account has no matching config block or the operation has no
    /// implementation for it (e.g. `--backend smtp mailboxes list`).
    #[arg(short, long, global = true, default_value_t)]
    pub backend: Backend,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    // --- Shared API
    //
    #[command(subcommand, visible_alias = "mbox", alias = "mailboxes")]
    Mailbox(MailboxCommand),
    #[command(subcommand, alias = "envelopes")]
    Envelope(EnvelopeCommand),
    #[command(subcommand, alias = "flags")]
    Flag(FlagCommand),
    #[command(subcommand, visible_alias = "msg", alias = "messages")]
    Message(MessageCommand),
    #[command(subcommand, alias = "attachments")]
    Attachment(AttachmentCommand),

    // --- Protocol-specific APIs
    //
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
    #[cfg(feature = "smtp")]
    #[command(subcommand)]
    Smtp(SmtpCommand),

    // --- Meta
    //
    #[command(subcommand)]
    Account(AccountCommand),
    Completion(CompletionCommand),
    Manual(ManualCommand),
}

/// Loads `Config` from the merged `config_paths` or, when no file
/// exists, runs the wizard to bootstrap one at the target path. Used
/// by every `build_*_client` helper to get a populated `Config` before
/// the per-backend client opens its connection.
pub fn load_or_wizard(config_paths: &[PathBuf]) -> Result<Config> {
    if let Some(config) = Config::from_paths_or_default(config_paths)? {
        return Ok(config);
    }

    match wizard::discover::run(&Config::target_path(config_paths)?)? {
        Some(config) => Ok(config),
        None => exit(0),
    }
}

impl Command {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let configs = || {
            let mut config = load_or_wizard(config_paths)?;

            let Some((_, account_config)) = config.take_account(account_name)? else {
                bail!("Cannot find account")
            };

            Ok((config, account_config))
        };

        match self {
            // --- Shared API
            //
            Self::Mailbox(cmd) => {
                let (config, account_config) = configs()?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Envelope(cmd) => {
                let (config, account_config) = configs()?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Flag(cmd) => {
                let (config, account_config) = configs()?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Message(cmd) => {
                let (config, account_config) = configs()?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Attachment(cmd) => {
                let (config, account_config) = configs()?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }

            // --- Protocol-specific APIs
            //
            #[cfg(feature = "imap")]
            Self::Imap(cmd) => {
                let (mut account, mut client) = build_imap_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "jmap")]
            Self::Jmap(cmd) => {
                let (mut account, mut client) = build_jmap_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "gmail")]
            Self::Gmail(cmd) => {
                let (mut account, mut client) = build_gmail_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "msgraph")]
            Self::Msgraph(cmd) => {
                let (mut account, mut client) = build_msgraph_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => {
                let (mut account, mut client) = build_maildir_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "m2dir")]
            Self::M2dir(cmd) => {
                let (mut account, mut client) = build_m2dir_client(config_paths, account_name)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "smtp")]
            Self::Smtp(cmd) => {
                let (_account, mut client) = build_smtp_client(config_paths, account_name)?;
                cmd.execute(printer, &mut client)
            }

            // --- Meta
            //
            Self::Account(cmd) => cmd.execute(printer, config_paths, account_name, backend),
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
        }
    }
}
