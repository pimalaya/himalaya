use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{
    himalaya::{backend::BackendBuilder, editor},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;
use url::Url;

use crate::{account::arg::name::AccountNameFlag, config::TomlConfig};

/// Parse and edit a message from the given mailto URL string.
///
/// This command allows you to edit a message from the mailto format
/// using the editor defined in your environment variable
/// $EDITOR. When the edition process finishes, you can choose between
/// saving or sending the final message.
#[derive(Debug, Parser)]
pub struct MessageMailtoCommand {
    /// The mailto url.
    #[arg()]
    pub url: Url,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageMailtoCommand {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: Url::parse(url)?,
            account: Default::default(),
        })
    }

    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing mailto message command");

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
                    .with_add_message(BackendFeatureSource::Context)
                    .with_send_message(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let mut msg = Vec::<u8>::new();
        let mut body = Vec::<u8>::new();

        msg.extend(b"Content-Type: text/plain; charset=utf-8\r\n");

        for (key, val) in self.url.query_pairs() {
            if key.eq_ignore_ascii_case("body") {
                body.extend(val.as_bytes());
            } else {
                msg.extend(key.as_bytes());
                msg.extend(b": ");
                msg.extend(val.as_bytes());
                msg.extend(b"\r\n");
            }
        }

        msg.extend(b"\r\n");
        msg.extend(body);

        if let Some(sig) = account_config.find_full_signature() {
            msg.extend(b"\r\n");
            msg.extend(sig.as_bytes());
        }

        let tpl = account_config
            .generate_tpl_interpreter()
            .with_show_only_headers(account_config.get_message_write_headers())
            .build()
            .from_bytes(msg)
            .await?
            .into();

        editor::edit_tpl_with_editor(account_config, printer, &backend, tpl).await
    }
}
