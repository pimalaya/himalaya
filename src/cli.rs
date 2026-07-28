use std::{path::PathBuf, process::exit};

use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, ConfigFlags, JsonFlag, LogFlags},
        commands::{CompletionCommand, JsonSchemaCommand, ManualCommand},
    },
    long_version,
    printer::Printer,
    prompt,
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
    config::{AccountConfig, Config},
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
    /// The subcommand to run. Omitted (bare `himalaya`), it runs the
    /// first-run wizard, which discovers an account and offers to save it
    /// to a config file (or prints it on stdout when redirected). With
    /// `--account` but no subcommand it shows this help instead.
    #[command(subcommand)]
    pub cmd: Option<Command>,

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
    JsonSchema(JsonSchemaCommand),
}

/// Resolves the account a command runs against: loads the merged config
/// from `config_paths`, then takes the account named by `-a` (or the one
/// marked `default`). Returns the leftover global config, the resolved
/// account name and its config.
///
/// When no config file exists, proposes the first-run wizard (which
/// discovers an account and offers to save it to a config file) then
/// exits. A config that exists but lacks the requested account is a hard
/// error: `take_account` bails on a missing named account, and a missing
/// default surfaces here.
fn resolve_account(
    printer: &mut impl Printer,
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Config, String, AccountConfig)> {
    let Some(mut config) = Config::from_paths_or_default(config_paths)? else {
        if prompt::bool(
            "No configuration found. Assist you in generating one?",
            true,
        )? {
            wizard::discover::run(printer)?;
        }
        exit(0);
    };

    let (name, account_config) = config.take_account(account_name)?.ok_or_else(|| {
        anyhow!("No default account found; select one with `-a <NAME>` or mark one with `default = true`")
    })?;

    Ok((config, name, account_config))
}

impl Command {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        match self {
            // --- Shared API
            //
            Self::Mailbox(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Envelope(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Flag(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Message(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            Self::Attachment(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }

            // --- Protocol-specific APIs
            //
            #[cfg(feature = "imap")]
            Self::Imap(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_imap_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "jmap")]
            Self::Jmap(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_jmap_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "gmail")]
            Self::Gmail(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_gmail_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "msgraph")]
            Self::Msgraph(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_msgraph_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_maildir_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "m2dir")]
            Self::M2dir(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_m2dir_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(feature = "smtp")]
            Self::Smtp(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (_account, mut client) = build_smtp_client(config, name, account_config)?;
                cmd.execute(printer, &mut client)
            }

            // --- Meta
            //
            Self::Account(cmd) => cmd.execute(printer, config_paths, account_name, backend),
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
            Self::JsonSchema(cmd) => cmd.execute(printer, crate::json_schema::schemas()),
        }
    }
}
