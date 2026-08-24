use std::{
    io::{IsTerminal, stdin},
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, JsonFlag, LogFlags},
        commands::{CompletionCommand, JsonSchemaCommand, ManualCommand},
        parsers::path_parser,
    },
    footer, long_version,
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
#[cfg(backend)]
use crate::shared::{
    attachment::cli::AttachmentCommand, envelope::cli::EnvelopeCommand, flag::cli::FlagCommand,
    mailbox::cli::MailboxCommand,
};
// `EmailClient` and the `message` command host the send path
// (`compose`/`send`), so they exist for any backend, not just storage.
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
    json_schema,
    wizard::{self, configure::ConfigureCommand, discover::CONFIG_SAMPLE_URL},
};

/// Top-level command-line interface parser.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_about = concat!(
    "CLI to manage emails.\n\n",
    "First time here? Run `himalaya` with no command: it offers to generate an ",
    "account discovered from your email address, which `himalaya configure` does ",
    "again later. Everything discovery does not cover is written by hand.",
))]
#[command(long_version = long_version!())]
#[command(after_help = footer!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    /// The subcommand to run.
    ///
    /// Omitted, a bare `himalaya` offers to generate a configuration when
    /// it finds none, since running the binary with no argument is what a
    /// newcomer does first, and shows this help otherwise.
    #[command(subcommand)]
    pub cmd: Option<Command>,

    #[command(flatten)]
    pub config: ConfigPathsArg,
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
    // --- Shared API (needs a storage backend)
    //
    #[cfg(backend)]
    #[command(subcommand, visible_alias = "mbox", alias = "mailboxes")]
    Mailbox(MailboxCommand),
    #[cfg(backend)]
    #[command(subcommand, alias = "envelopes")]
    Envelope(EnvelopeCommand),
    #[cfg(backend)]
    #[command(subcommand, alias = "flags")]
    Flag(FlagCommand),
    #[cfg(any(backend, feature = "smtp"))]
    #[command(subcommand, visible_alias = "msg", alias = "messages")]
    Message(MessageCommand),
    #[cfg(backend)]
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
    #[cfg(feature = "sieve")]
    #[command(subcommand)]
    Sieve(SieveCommand),

    // --- Meta
    //
    /// Configure an account interactively.
    #[command(visible_alias = "wizard")]
    Configure(ConfigureCommand),
    #[command(subcommand)]
    Account(AccountCommand),
    Completion(CompletionCommand),
    Manual(ManualCommand),
    JsonSchema(JsonSchemaCommand),
}

/// Path(s) to the TOML configuration file(s).
///
/// Declared here rather than taken from pimalaya-cli, so the environment
/// variable carries this product's name.
#[derive(Debug, Default, Parser)]
pub struct ConfigPathsArg {
    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). Other paths are merged with the first one, which
    /// allows you to separate your public config from your private
    /// one(s). Multiple paths can also be given at once, delimited by
    /// `:` like `$PATH` in a POSIX shell.
    #[arg(long = "config", short = 'c', global = true, env = "HIMALAYA_CONFIG")]
    #[arg(name = "config_paths", value_name = "PATH", value_parser = path_parser, value_delimiter = ':')]
    pub paths: Vec<PathBuf>,
}

/// Welcomes, then offers to generate a first configuration. Returns
/// whether the wizard ran.
///
/// Raised from the two places nothing can happen without a
/// configuration: a bare invocation, and a command that needs an
/// account. It is a hook rather than a gate, so declining it decides
/// nothing: what happens next is the caller's business, and for a
/// command that is simply carrying on.
pub fn offer_configuration(
    printer: &mut impl Printer,
    config_paths: &[PathBuf],
    path: &Path,
) -> Result<bool> {
    wizard::configure::print_welcome(path);

    if !prompt::bool("Create a configuration with a default account?", true)? {
        return Ok(false);
    }

    ConfigureCommand.execute(printer, config_paths)?;

    Ok(true)
}

/// Resolves the account a command runs against: loads the merged config
/// from `config_paths`, then takes the account named by `-a` (or the one
/// marked `default`). Returns the leftover global config, the resolved
/// account name and its config.
///
/// A missing configuration is met with the wizard rather than with an
/// error: the welcome frames what Himalaya is and offers to generate an
/// account, then the command carries on either way. Accepting is what
/// gives it a chance to work; declining leaves it to fail on the
/// configuration it still has not got. The two other failures name what
/// is missing and how to pick an account.
fn resolve_account(
    printer: &mut impl Printer,
    config_paths: &[PathBuf],
    account_name: Option<&str>,
) -> Result<(Config, String, AccountConfig)> {
    let mut config = match Config::from_paths_or_default(config_paths)? {
        Some(config) => config,
        None => {
            // NOTE: the target path is where `-c` pointed, or the default
            // location when it named none, so a mistyped path shows up as
            // itself rather than as a generic first run.
            let path = Config::target_path(config_paths)?;

            // NOTE: nobody is there to answer a prompt in a script or a
            // cron job, and a JSON consumer wants a failure it can read,
            // so both skip the offer and fail below.
            if !printer.is_json() && stdin().is_terminal() {
                offer_configuration(printer, config_paths, &path)?;
            }

            // NOTE: the wizard also prints the account instead of writing
            // it, so having run it proves nothing: the configuration is
            // looked up again, and the command fails the ordinary way
            // when nothing landed.
            match Config::from_paths_or_default(config_paths)? {
                Some(config) => config,
                None => bail!(
                    "No configuration found at {}, run `himalaya configure` to generate one or write it by hand: {CONFIG_SAMPLE_URL}",
                    path.display(),
                ),
            }
        }
    };

    // NOTE: an empty name and `default` both mean the default account,
    // which is the next block's business.
    let named = account_name.filter(|name| !name.is_empty() && *name != "default");

    if let Some(name) = named.filter(|name| !config.accounts.contains_key(*name)) {
        let mut names: Vec<&str> = config.accounts.keys().map(String::as_str).collect();
        names.sort_unstable();

        bail!(
            "Account `{name}` not found, the configuration holds: {}",
            names.join(", "),
        );
    }

    let Some((name, account_config)) = config.take_account(account_name)? else {
        bail!(
            "No default account found, name one with `-a <NAME>` or mark one with `default = true`"
        );
    };

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
            // --- Shared API (needs a storage backend)
            //
            #[cfg(backend)]
            Self::Mailbox(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(backend)]
            Self::Envelope(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(backend)]
            Self::Flag(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(any(backend, feature = "smtp"))]
            Self::Message(cmd) => {
                let (config, _name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = EmailClient::new(config, account_config, backend)?;
                cmd.execute(printer, &mut account, &mut client)
            }
            #[cfg(backend)]
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
            #[cfg(feature = "sieve")]
            Self::Sieve(cmd) => {
                let (config, name, account_config) =
                    resolve_account(printer, config_paths, account_name)?;
                let (mut account, mut client) = build_sieve_client(config, name, account_config)?;
                cmd.execute(printer, &mut account, &mut client)
            }

            // --- Meta
            //
            Self::Configure(cmd) => cmd.execute(printer, config_paths),
            Self::Account(cmd) => cmd.execute(printer, config_paths, account_name, backend),
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
            Self::JsonSchema(cmd) => cmd.execute(printer, json_schema::schemas()),
        }
    }
}
