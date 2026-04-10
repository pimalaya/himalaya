use std::sync::Arc;

use clap::Parser;
use color_eyre::{
    eyre::{bail, eyre},
    Result,
};
use email::{backend::feature::BackendFeatureSource, config::Config, flag::Flag};
use mml::MmlCompilerBuilder;
use pimalaya_tui::{
    himalaya::{backend::BackendBuilder, editor},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
    message::arg::{
        body::MessageRawBodyArg, header::HeaderRawArgs, reply::MessageReplyAllArg,
        yes::MessageYesArg,
    },
};

/// Reply to the message associated to the given envelope id.
///
/// This command allows you to reply to the given message using the
/// editor defined in your environment variable $EDITOR. When the
/// edition process finishes, you can choose between saving or sending
/// the final message.
///
/// When --yes is given together with --body, the editor is skipped
/// and the reply is sent immediately. This is useful for automated
/// or scripted workflows that do not have an interactive terminal.
#[derive(Debug, Parser)]
pub struct MessageReplyCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    #[command(flatten)]
    pub reply: MessageReplyAllArg,

    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[command(flatten)]
    pub yes: MessageYesArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageReplyCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing reply message command");

        let folder = &self.folder.name;
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
        .build()
        .await?;

        let id = self.envelope.id;

        if self.yes.yes && self.body.is_empty() {
            bail!("--yes requires --body to be set");
        }

        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(eyre!("cannot find message {id}"))?
            .to_reply_tpl_builder(account_config.clone())
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .with_reply_all(self.reply.all)
            .build()
            .await?;

        if self.yes.yes {
            let email = MmlCompilerBuilder::new()
                .build(tpl.as_str())?
                .compile()
                .await?
                .into_vec()?;

            backend.send_message_then_save_copy(&email).await?;
            printer.out("Message successfully sent!\n")?;
        } else {
            editor::edit_tpl_with_editor(account_config, printer, &backend, tpl).await?;
        }

        backend.add_flag(folder, &[id], Flag::Answered).await?;

        Ok(())
    }
}
