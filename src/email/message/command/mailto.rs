use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use mail_builder::MessageBuilder;
use pimalaya_tui::{
    himalaya::{backend::BackendBuilder, editor},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;
use url::Url;

use crate::{account::arg::name::AccountNameFlag, config::TomlConfig};

/// Parse and edit a message from a mailto URL string.
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

        let mut builder = MessageBuilder::new().to(self.url.path());
        let mut body = String::new();

        for (key, val) in self.url.query_pairs() {
            match key {
                key if key.eq_ignore_ascii_case("in-reply-to") => {
                    builder = builder.in_reply_to(val.to_string());
                }
                key if key.eq_ignore_ascii_case("cc") => {
                    builder = builder.cc(val.to_string());
                }
                key if key.eq_ignore_ascii_case("bcc") => {
                    builder = builder.bcc(val.to_string());
                }
                key if key.eq_ignore_ascii_case("subject") => {
                    builder = builder.subject(val.to_string());
                }
                key if key.eq_ignore_ascii_case("body") => {
                    body += &val;
                }
                _ => (),
            }
        }

        match account_config.find_full_signature() {
            Some(ref sig) => builder = builder.text_body(body + "\n\n" + sig),
            None => builder = builder.text_body(body),
        }

        let tpl = account_config
            .generate_tpl_interpreter()
            .with_show_only_headers(account_config.get_message_write_headers())
            .build()
            .from_msg_builder(builder)
            .await?
            .into();

        editor::edit_tpl_with_editor(account_config, printer, &backend, tpl).await
    }
}
