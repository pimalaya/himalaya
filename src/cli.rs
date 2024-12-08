use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::Result;
use pimalaya_tui::{
    long_version,
    terminal::{
        cli::{
            arg::path_parser,
            printer::{OutputFmt, Printer},
        },
        config::TomlConfig as _,
    },
};

use crate::{
    account::command::AccountSubcommand,
    completion::command::CompletionGenerateCommand,
    config::TomlConfig,
    envelope::command::EnvelopeSubcommand,
    flag::command::FlagSubcommand,
    folder::command::FolderSubcommand,
    manual::command::ManualGenerateCommand,
    message::{
        attachment::command::AttachmentSubcommand, command::MessageSubcommand,
        template::command::TemplateSubcommand,
    },
};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<HimalayaCommand>,

    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). If the first path does not point to a valid file,
    /// the wizard will propose to assist you in the creation of the
    /// configuration file. Other paths are merged with the first one,
    /// which allows you to separate your public config from your
    /// private(s) one(s).
    #[arg(short, long = "config", global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser)]
    pub config_paths: Vec<PathBuf>,

    /// Customize the output format.
    ///
    /// The output format determine how to display commands output to
    /// the terminal.
    ///
    /// The possible values are:
    ///
    ///  - json: output will be in a form of a JSON-compatible object
    ///
    ///  - plain: output will be in a form of either a plain text or
    ///    table, depending on the command
    #[arg(long, short, global = true)]
    #[arg(value_name = "FORMAT", value_enum, default_value_t = Default::default())]
    pub output: OutputFmt,

    /// Enable logs with spantrace.
    ///
    /// This is the same as running the command with `RUST_LOG=debug`
    /// environment variable.
    #[arg(long, global = true, conflicts_with = "trace")]
    pub debug: bool,

    /// Enable verbose logs with backtrace.
    ///
    /// This is the same as running the command with `RUST_LOG=trace`
    /// and `RUST_BACKTRACE=1` environment variables.
    #[arg(long, global = true, conflicts_with = "debug")]
    pub trace: bool,
}

#[derive(Subcommand, Debug)]
pub enum HimalayaCommand {
    #[command(subcommand)]
    #[command(alias = "accounts")]
    Account(AccountSubcommand),

    #[command(subcommand)]
    #[command(visible_alias = "mailbox", aliases = ["mailboxes", "mboxes", "mbox"])]
    #[command(alias = "folders")]
    Folder(FolderSubcommand),

    #[command(subcommand)]
    #[command(alias = "envelopes")]
    Envelope(EnvelopeSubcommand),

    #[command(subcommand)]
    #[command(alias = "flags")]
    Flag(FlagSubcommand),

    #[command(subcommand)]
    #[command(alias = "messages", alias = "msgs", alias = "msg")]
    Message(MessageSubcommand),

    #[command(subcommand)]
    #[command(alias = "attachments")]
    Attachment(AttachmentSubcommand),

    #[command(subcommand)]
    #[command(alias = "templates", alias = "tpls", alias = "tpl")]
    Template(TemplateSubcommand),

    #[command(arg_required_else_help = true)]
    #[command(alias = "manuals", alias = "mans")]
    Manual(ManualGenerateCommand),

    #[command(arg_required_else_help = true)]
    #[command(alias = "completions")]
    Completion(CompletionGenerateCommand),
}

impl HimalayaCommand {
    pub async fn execute(self, printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        match self {
            Self::Account(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, config, config_paths.first()).await
            }
            Self::Folder(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Envelope(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Flag(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Message(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Attachment(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Template(cmd) => {
                let config = TomlConfig::from_paths_or_default(config_paths).await?;
                cmd.execute(printer, &config).await
            }
            Self::Manual(cmd) => cmd.execute(printer).await,
            Self::Completion(cmd) => cmd.execute().await,
        }
    }
}
