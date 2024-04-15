use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use mail_builder::MessageBuilder;
use tracing::info;
use url::Url;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig, printer::Printer,
    ui::editor,
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

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageMailtoCommand {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: Url::parse(url)?,
            #[cfg(feature = "account-sync")]
            cache: Default::default(),
            account: Default::default(),
        })
    }

    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing mailto message command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_message_kind();
        let send_message_kind = toml_account_config.send_message_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            add_message_kind.into_iter().chain(send_message_kind),
            |builder| {
                builder.set_add_message(BackendFeatureSource::Context);
                builder.set_send_message(BackendFeatureSource::Context);
            },
        )
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
