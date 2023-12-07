use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    account::command::AccountSubcommand,
    completion::command::CompletionGenerateCommand,
    config::{self, TomlConfig},
    envelope::command::EnvelopeSubcommand,
    flag::command::FlagSubcommand,
    folder::command::FolderSubcommand,
    manual::command::ManualGenerateCommand,
    message::command::MessageSubcommand,
    output::{ColorFmt, OutputFmt},
    printer::Printer,
};

#[derive(Parser, Debug)]
#[command(
    name = "himalaya",
    author,
    version,
    about,
    propagate_version = true,
    infer_subcommands = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: HimalayaCommand,

    /// Override the default configuration file path
    ///
    /// The given path is shell-expanded then canonicalized (if
    /// applicable). If the path does not point to a valid file, the
    /// wizard will propose to assist you in the creation of the
    /// configuration file.
    #[arg(long, short, global = true)]
    #[arg(value_name = "PATH", value_parser = config::path_parser)]
    pub config: Option<PathBuf>,

    /// Customize the output format
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

    /// Control when to use colors
    ///
    /// The default setting is 'auto', which means himalaya will try
    /// to guess when to use colors. For example, if himalaya is
    /// printing to a terminal, then it will use colors, but if it is
    /// redirected to a file or a pipe, then it will suppress color
    /// output. himalaya will suppress color output in some other
    /// circumstances as well. For example, if the TERM environment
    /// variable is not set or set to 'dumb', then himalaya will not
    /// use colors.
    ///
    /// The possible values are:
    ///
    ///  - never: colors will never be used
    ///
    ///  - always: colors will always be used regardless of where output is sent
    ///
    ///  - ansi: like 'always', but emits ANSI escapes (even in a Windows console)
    ///
    ///  - auto: himalaya tries to be smart
    #[arg(long, short = 'C', global = true)]
    #[arg(value_name = "MODE", value_enum, default_value_t = Default::default())]
    pub color: ColorFmt,
}

#[derive(Subcommand, Debug)]
pub enum HimalayaCommand {
    /// Manage accounts
    #[command(subcommand)]
    #[command(alias = "accounts")]
    Account(AccountSubcommand),

    /// Manage folders
    #[command(subcommand)]
    #[command(alias = "folders")]
    Folder(FolderSubcommand),

    /// Manage envelopes
    #[command(subcommand)]
    #[command(alias = "envelopes")]
    Envelope(EnvelopeSubcommand),

    /// Manage flags
    #[command(subcommand)]
    #[command(alias = "flags")]
    Flag(FlagSubcommand),

    /// Manage messages
    #[command(subcommand)]
    #[command(alias = "messages", alias = "msgs", alias = "msg")]
    Message(MessageSubcommand),

    /// Generate manual pages to a directory
    #[command(arg_required_else_help = true)]
    #[command(alias = "manuals", alias = "mans")]
    Manual(ManualGenerateCommand),

    /// Print completion script for a shell to stdout
    #[command(arg_required_else_help = true)]
    #[command(alias = "completions")]
    Completion(CompletionGenerateCommand),
}

impl HimalayaCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Account(cmd) => cmd.execute(printer, config).await,
            Self::Folder(cmd) => cmd.execute(printer, config).await,
            Self::Envelope(cmd) => cmd.execute(printer, config).await,
            Self::Flag(cmd) => cmd.execute(printer, config).await,
            Self::Message(cmd) => cmd.execute(printer, config).await,
            Self::Manual(cmd) => cmd.execute(printer).await,
            Self::Completion(cmd) => cmd.execute(printer).await,
        }
    }
}
