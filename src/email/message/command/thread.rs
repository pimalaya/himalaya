use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::envelope::arg::ids::EnvelopeIdArg;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Read human-friendly version of messages associated to the
/// given envelope id's thread.
///
/// This command allows you to thread a message. When threading a message,
/// the "seen" flag is automatically applied to the corresponding
/// envelope. To prevent this behaviour, use the --preview flag.
#[derive(Debug, Parser)]
pub struct MessageThreadCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    /// Thread the message without applying the "seen" flag to its
    /// corresponding envelope.
    #[arg(long, short)]
    pub preview: bool,

    /// Thread only the body of the message.
    ///
    /// All headers will be removed from the message.
    #[arg(long)]
    #[arg(conflicts_with = "headers")]
    pub no_headers: bool,

    /// List of headers that should be visible at the top of the
    /// message.
    ///
    /// If a given header is not found in the message, it will not be
    /// visible. If no header is given, defaults to the one set up in
    /// your TOML configuration file.
    #[arg(long = "header", short = 'H', value_name = "NAME")]
    #[arg(conflicts_with = "no_headers")]
    pub headers: Vec<String>,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageThreadCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing thread message(s) command");

        let folder = &self.folder.name;
        let id = &self.envelope.id;

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
                    .with_peek_messages(BackendFeatureSource::Context)
                    .with_thread_envelopes(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let envelopes = backend
            .thread_envelope(folder, *id, Default::default())
            .await?;

        let ids: Vec<_> = envelopes
            .graph()
            .nodes()
            .map(|e| e.id.parse::<usize>().unwrap())
            .collect();

        let emails = if self.preview {
            backend.peek_messages(folder, &ids).await
        } else {
            backend.get_messages(folder, &ids).await
        }?;

        let mut glue = "";
        let mut bodies = String::default();

        for (i, email) in emails.to_vec().iter().enumerate() {
            bodies.push_str(glue);
            bodies.push_str(&format!("-------- Message {} --------\n\n", ids[i + 1]));

            let tpl = email
                .to_read_tpl(&account_config, |mut tpl| {
                    if self.no_headers {
                        tpl = tpl.with_hide_all_headers();
                    } else if !self.headers.is_empty() {
                        tpl = tpl.with_show_only_headers(&self.headers);
                    }

                    tpl
                })
                .await?;

            bodies.push_str(&tpl);
            glue = "\n\n";
        }

        printer.out(bodies)
    }
}
