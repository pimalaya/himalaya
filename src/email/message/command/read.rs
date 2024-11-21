use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use mml::message::FilterParts;
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Read a message.
///
/// This command allows you to read a message. When reading a message,
/// the "seen" flag is automatically applied to the corresponding
/// envelope. To prevent this behaviour, use the --preview flag.
#[derive(Debug, Parser)]
pub struct MessageReadCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    /// Read the message without applying the "seen" flag to its
    /// corresponding envelope.
    #[arg(long, short)]
    pub preview: bool,

    /// Read the raw version of the given message.
    ///
    /// The raw message represents the headers and the body as it is
    /// on the backend, unedited: not decoded nor decrypted. This is
    /// useful for debugging faulty messages, but also for
    /// saving/sending/transfering messages.
    #[arg(long, short)]
    #[arg(conflicts_with = "no_headers")]
    #[arg(conflicts_with = "headers")]
    pub raw: bool,

    /// Read only body of text/html parts.
    ///
    /// This argument is useful when you need to read the HTML version
    /// of a message. Combined with --no-headers, you can write it to
    /// a .html file and open it with your favourite browser.
    #[arg(long)]
    #[arg(conflicts_with = "raw")]
    pub html: bool,

    /// Read only the body of the message.
    ///
    /// All headers will be removed from the message.
    #[arg(long)]
    #[arg(conflicts_with = "raw")]
    #[arg(conflicts_with = "headers")]
    pub no_headers: bool,

    /// List of headers that should be visible at the top of the
    /// message.
    ///
    /// If a given header is not found in the message, it will not be
    /// visible. If no header is given, defaults to the one set up in
    /// your TOML configuration file.
    #[arg(long = "header", short = 'H', value_name = "NAME")]
    #[arg(conflicts_with = "raw")]
    #[arg(conflicts_with = "no_headers")]
    pub headers: Vec<String>,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageReadCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing read message(s) command");

        let folder = &self.folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let account_config = Arc::new(account_config);

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            account_config.clone(),
            |builder| {
                builder
                    .without_features()
                    .with_get_messages(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let emails = if self.preview {
            backend.peek_messages(folder, ids).await
        } else {
            backend.get_messages(folder, ids).await
        }?;

        let mut glue = "";
        let mut bodies = String::default();

        for email in emails.to_vec() {
            bodies.push_str(glue);

            if self.raw {
                // emails do not always have valid utf8, uses "lossy" to
                // display what can be displayed
                bodies.push_str(&String::from_utf8_lossy(email.raw()?));
            } else {
                let tpl = email
                    .to_read_tpl(&account_config, |mut tpl| {
                        if self.no_headers {
                            tpl = tpl.with_hide_all_headers();
                        } else if !self.headers.is_empty() {
                            tpl = tpl.with_show_only_headers(&self.headers);
                        }

                        if self.html {
                            tpl = tpl.with_filter_parts(FilterParts::Only("text/html".into()));
                        }

                        tpl
                    })
                    .await?;
                bodies.push_str(&tpl);
            }

            glue = "\n\n";
        }

        printer.out(bodies)
    }
}
