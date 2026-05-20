// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;

use anyhow::{bail, Result};
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
use crate::imap::{cli::ImapCommand, client::build_imap_client};
#[cfg(feature = "jmap")]
use crate::jmap::{cli::JmapCommand, client::build_jmap_client};
#[cfg(feature = "maildir")]
use crate::maildir::{cli::MaildirCommand, client::build_maildir_client};
#[cfg(feature = "smtp")]
use crate::smtp::{cli::SmtpCommand, client::build_smtp_client};
use crate::{
    account::cli::AccountCommand,
    backend::Backend,
    config::Config,
    shared::{
        attachments::cli::AttachmentCommand, client::EmailClient, envelopes::cli::EnvelopeCommand,
        flags::cli::FlagCommand, mailboxes::cli::MailboxCommand, messages::cli::MessageCommand,
    },
    wizard,
};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct HimalayaCli {
    #[command(subcommand)]
    pub command: HimalayaCommand,

    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). If the first path does not point to a valid file,
    /// the wizard will propose to assist you in the creation of the
    /// configuration file. Other paths are merged with the first one,
    /// which allows you to separate your public config from your
    /// private(s) one(s). Multiple paths can also be provided by
    /// delimiting them with `:` (like `$PATH` in a POSIX shell).
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
    pub backend: Backend,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

#[derive(Debug, Subcommand)]
pub enum HimalayaCommand {
    // --- Shared API
    //
    #[command(subcommand, visible_alias = "mbox", alias = "mboxes")]
    Mailboxes(MailboxCommand),
    #[command(subcommand)]
    Envelopes(EnvelopeCommand),
    #[command(subcommand)]
    Flags(FlagCommand),
    #[command(subcommand, visible_alias = "msg", alias = "msgs")]
    Messages(MessageCommand),
    #[command(subcommand)]
    Attachments(AttachmentCommand),

    // --- Protocol-specific APIs
    //
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

    // --- Meta
    //
    #[command(subcommand)]
    Account(AccountCommand),
    Completions(CompletionCommand),
    Manuals(ManualCommand),
}

/// Loads `Config` from the merged `config_paths` or, when no file
/// exists, runs the wizard to bootstrap one at the target path. Used
/// by every `build_*_client` helper to get a populated `Config` before
/// the per-backend client opens its connection.
pub fn load_or_wizard(config_paths: &[PathBuf]) -> Result<Config> {
    match Config::from_paths_or_default(config_paths)? {
        Some(config) => Ok(config),
        None => wizard::discover::run_or_exit(&Config::target_path(config_paths)?),
    }
}

impl HimalayaCommand {
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
            Self::Mailboxes(cmd) => {
                let (config, account_config) = configs()?;
                let client = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, client)
            }
            Self::Envelopes(cmd) => {
                let (config, account_config) = configs()?;
                let client = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, client)
            }
            Self::Flags(cmd) => {
                let (config, account_config) = configs()?;
                let client = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, client)
            }
            Self::Messages(cmd) => {
                let (config, account_config) = configs()?;
                let client = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, client)
            }
            Self::Attachments(cmd) => {
                let (config, account_config) = configs()?;
                let client = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, client)
            }

            // --- Protocol-specific APIs
            //
            #[cfg(feature = "imap")]
            Self::Imap(cmd) => {
                let client = build_imap_client(config_paths, account_name)?;
                cmd.execute(printer, client)
            }
            #[cfg(feature = "jmap")]
            Self::Jmap(cmd) => {
                let client = build_jmap_client(config_paths, account_name)?;
                cmd.execute(printer, client)
            }
            #[cfg(feature = "maildir")]
            Self::Maildir(cmd) => {
                let client = build_maildir_client(config_paths, account_name)?;
                cmd.execute(printer, client)
            }
            #[cfg(feature = "smtp")]
            Self::Smtp(cmd) => {
                let client = build_smtp_client(config_paths, account_name)?;
                cmd.execute(printer, client)
            }

            // --- Meta
            //
            Self::Account(cmd) => cmd.execute(printer, config_paths, account_name, backend),
            Self::Completions(cmd) => cmd.execute(printer, HimalayaCli::command()),
            Self::Manuals(cmd) => cmd.execute(printer, HimalayaCli::command()),
        }
    }
}
