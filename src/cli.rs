use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand};
use pimalaya_toolbox::{
    config::TomlConfig,
    long_version,
    terminal::{
        clap::{
            args::{AccountArg, JsonFlag, LogFlags},
            commands::{CompletionCommand, ManualCommand},
            parsers::path_parser,
        },
        printer::Printer,
    },
};

use crate::{
    // account::command::AccountSubcommand,
    account::Account,
    config::Config,
    folder::command::MailboxCommand, // message::{
                                     //     attachment::command::AttachmentSubcommand, command::MessageSubcommand,
                                     //     template::command::TemplateSubcommand,
                                     // },
};

/// IMAP CLI (requires `imap` cargo feature).
///
/// This command gives you access to the IMAP CLI API, and allows
/// you to manage IMAP mailboxes: list mailboxes, read messages,
/// add flags etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "lowercase")]
pub enum ImapCommand {
    #[command(subcommand)]
    #[command(aliases = ["mboxes", "mbox"])]
    Mailboxes(MailboxCommand),
}

impl ImapCommand {
    pub fn execute(self, printer: &mut impl Printer, account: Account) -> Result<()> {
        match self {
            Self::Mailboxes(cmd) => cmd.execute(printer, account),
        }
    }
}

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
    pub account: AccountArg,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

#[derive(Debug, Subcommand)]
pub enum BackendCommand {
    #[command(subcommand)]
    Imap(ImapCommand),
}

impl BackendCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::Imap(cmd) => {
                let config = Config::from_paths_or_default(config_paths)?;
                let (_, account) = config.get_account(account_name)?;
                cmd.execute(printer, account)
            }
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum HimalayaCommand {
    // #[command(subcommand)]
    // #[command(alias = "accounts")]
    // Account(AccountSubcommand),
    #[command(subcommand)]
    #[command(visible_alias = "mailbox", aliases = ["mailboxes", "mboxes", "mbox"])]
    #[command(alias = "folders")]
    Folder(MailboxCommand),

    // #[command(subcommand)]
    // #[command(alias = "envelopes")]
    // Envelope(EnvelopeSubcommand),

    // #[command(subcommand)]
    // #[command(alias = "flags")]
    // Flag(FlagSubcommand),

    // #[command(subcommand)]
    // #[command(alias = "messages", alias = "msgs", alias = "msg")]
    // Message(MessageSubcommand),

    // #[command(subcommand)]
    // #[command(alias = "attachments")]
    // Attachment(AttachmentSubcommand),

    // #[command(subcommand)]
    // #[command(alias = "templates", alias = "tpls", alias = "tpl")]
    // Template(TemplateSubcommand),
    #[command(arg_required_else_help = true, alias = "mans")]
    Manuals(ManualCommand),
    #[command(arg_required_else_help = true)]
    Completions(CompletionCommand),
}

impl HimalayaCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
    ) -> Result<()> {
        match self {
            // Self::Account(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, config, config_paths.first()).await
            // }
            Self::Folder(cmd) => {
                let config = Config::from_paths_or_default(config_paths)?;
                let (_, account) = config.get_account(account_name)?;
                cmd.execute(printer, account)
            }
            // Self::Envelope(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, &config).await
            // }
            // Self::Flag(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, &config).await
            // }
            // Self::Message(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, &config).await
            // }
            // Self::Attachment(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, &config).await
            // }
            // Self::Template(cmd) => {
            //     let config = TomlConfig::from_paths_or_default(config_paths).await?;
            //     cmd.execute(printer, &config).await
            // }
            Self::Manuals(cmd) => cmd.execute(printer, HimalayaCli::command()),
            Self::Completions(cmd) => cmd.execute(printer, HimalayaCli::command()),
        }
    }
}
