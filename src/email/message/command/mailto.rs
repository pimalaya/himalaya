use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use mail_builder::MessageBuilder;
use url::Url;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, printer::Printer, ui::editor,
};

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
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageMailtoCommand {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: Url::parse(url)?,
            cache: Default::default(),
            account: Default::default(),
        })
    }

    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message mailto command");

        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;

        let mut builder = MessageBuilder::new().to(self.url.path());
        let mut body = String::new();

        for (key, val) in self.url.query_pairs() {
            match key.to_lowercase().as_bytes() {
                b"cc" => builder = builder.cc(val.to_string()),
                b"bcc" => builder = builder.bcc(val.to_string()),
                b"subject" => builder = builder.subject(val),
                b"body" => body += &val,
                _ => (),
            }
        }

        match account_config.find_full_signature() {
            Ok(Some(ref signature)) => builder = builder.text_body(body + "\n\n" + signature),
            Ok(None) => builder = builder.text_body(body),
            Err(err) => {
                debug!("cannot add signature to mailto message, skipping it: {err}");
                debug!("{err:?}");
            }
        }

        let tpl = account_config
            .generate_tpl_interpreter()
            .with_show_only_headers(account_config.get_message_write_headers())
            .build()
            .from_msg_builder(builder)
            .await?;

        editor::edit_tpl_with_editor(&account_config, printer, &backend, tpl).await
    }
}
