use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use pimalaya_toolbox::{
    config::TomlConfig,
    long_version,
    terminal::{
        clap::{
            args::{AccountFlag, JsonFlag, LogFlags},
            parsers::path_parser,
        },
        printer::Printer,
    },
};

use crate::config::Config;
#[cfg(feature = "imap")]
use crate::imap::command::ImapCommand;
#[cfg(feature = "smtp")]
use crate::smtp::command::SmtpCommand;

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
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

#[derive(Debug, Subcommand)]
pub enum BackendCommand {
    #[cfg(feature = "imap")]
    #[command(subcommand)]
    Imap(ImapCommand),
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
    ) -> Result<()> {
        match self {
            Self::Imap(cmd) => {
                let config = Config::from_paths_or_default(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                let Some(imap_config) = account_config.imap else {
                    bail!("IMAP config is missing for this account")
                };

                cmd.execute(printer, imap_config)
            }
            Self::Smtp(cmd) => {
                let config = Config::from_paths_or_default(config_paths)?;
                let (_, account_config) = config.get_account(account_name)?;
                let Some(smtp_config) = account_config.smtp else {
                    bail!("SMTP config is missing for this account")
                };

                cmd.execute(printer, smtp_config)
            }
        }
    }
}
