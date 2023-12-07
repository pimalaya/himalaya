use anyhow::Result;
use clap::Parser;
use log::info;
use mail_builder::MessageBuilder;
use url::Url;

use crate::{backend::Backend, config::TomlConfig, printer::Printer, ui::editor};

/// Parse and edit a message from a mailto URL string
#[derive(Debug, Parser)]
pub struct MessageMailtoCommand {
    /// The mailto url
    #[arg()]
    pub url: Url,
}

impl MessageMailtoCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message mailto command");

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(None, false)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;

        let mut builder = MessageBuilder::new().to(self.url.path());

        for (key, val) in self.url.query_pairs() {
            match key.to_lowercase().as_bytes() {
                b"cc" => builder = builder.cc(val.to_string()),
                b"bcc" => builder = builder.bcc(val.to_string()),
                b"subject" => builder = builder.subject(val),
                b"body" => builder = builder.text_body(val),
                _ => (),
            }
        }

        let tpl = account_config
            .generate_tpl_interpreter()
            .with_show_only_headers(account_config.email_writing_headers())
            .build()
            .from_msg_builder(builder)
            .await?;

        editor::edit_tpl_with_editor(&account_config, printer, &backend, tpl).await
    }
}
