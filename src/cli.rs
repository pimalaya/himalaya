use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{
    account::command as account,
    completion::command as completion,
    config::{self, TomlConfig},
    man::command as man,
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
    pub command: Command,

    /// Override the default configuration file path
    ///
    /// The given path is shell-expanded then canonicalized (if
    /// applicable). If the path does not point to a valid file, the
    /// wizard will propose to assist you in the creation of the
    /// configuration file.
    #[arg(long, short, value_name = "PATH", global = true, value_parser = config::path_parser)]
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
    #[arg(
	long,
	short,
        value_name = "FORMAT",
        global = true,
        value_enum,
	default_value_t = Default::default(),
    )]
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
    #[arg(
	long,
        short = 'C',
        value_name = "MODE",
        global = true,
        value_enum,
	default_value_t = Default::default(),
    )]
    pub color: ColorFmt,
}

/// Top-level CLI commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Subcommand to manage accounts
    #[command(subcommand)]
    Account(account::Command),

    /// Generate all man pages to the given directory
    #[command(arg_required_else_help = true, alias = "mans")]
    Man(man::Command),

    /// Print completion script for the given shell to stdout
    #[command(arg_required_else_help = true, aliases = ["completions", "compl", "comp"])]
    Completion(completion::Command),
}

impl Command {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Account(cmd) => cmd.execute(printer, config).await,
            Self::Man(cmd) => cmd.execute(printer).await,
            Self::Completion(cmd) => cmd.execute(printer).await,
        }
    }
}
