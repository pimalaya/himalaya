use anyhow::Result;
use clap::Parser;
#[cfg(feature = "sendmail")]
use email::message::send_raw::sendmail::SendRawMessageSendmail;
#[cfg(feature = "smtp")]
use email::message::send_raw::smtp::SendRawMessageSmtp;
use log::info;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    email::template::arg::TemplateRawArg,
    printer::Printer,
};

/// Send a template.
///
/// This command allows you to send a template and save a copy to the
/// sent folder. The template is compiled into a MIME message before
/// being sent. If you want to send a raw message, use the message
/// send command instead.
#[derive(Debug, Parser)]
pub struct TemplateSendCommand {
    #[command(flatten)]
    pub template: TemplateRawArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template send command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let send_message_kind = toml_account_config.send_raw_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            send_message_kind,
            |builder| {
                match send_message_kind {
                    #[cfg(feature = "smtp")]
                    Some(BackendKind::Smtp) => {
                        builder.set_send_raw_message(|ctx| {
                            ctx.smtp.as_ref().and_then(SendRawMessageSmtp::new)
                        });
                    }
                    #[cfg(feature = "sendmail")]
                    Some(BackendKind::Sendmail) => {
                        builder.set_send_raw_message(|ctx| {
                            ctx.sendmail.as_ref().and_then(SendRawMessageSendmail::new)
                        });
                    }
                    _ => (),
                };
            },
        )
        .await?;

        let tpl = if io::stdin().is_terminal() {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

        backend.send_raw_message(&msg).await?;

        printer.print("Template successfully sent!")
    }
}
