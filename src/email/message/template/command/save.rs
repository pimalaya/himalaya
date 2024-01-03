use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add_raw::imap::AddRawMessageImap;
#[cfg(feature = "maildir")]
use email::message::add_raw_with_flags::maildir::AddRawMessageWithFlagsMaildir;
use log::info;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    email::template::arg::TemplateRawArg,
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Save a template to a folder.
///
/// This command allows you to save a template to the given
/// folder. The template is compiled into a MIME message before being
/// saved to the folder. If you want to save a raw message, use the
/// message save command instead.
#[derive(Debug, Parser)]
pub struct TemplateSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub template: TemplateRawArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template save command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_raw_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_message_kind,
            |builder| match add_message_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_add_raw_message_with_flags(|ctx| {
                        ctx.maildir
                            .as_ref()
                            .and_then(AddRawMessageWithFlagsMaildir::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_raw_message_with_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(AddRawMessageWithFlagsMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_raw_message(|ctx| {
                        ctx.imap.as_ref().and_then(AddRawMessageImap::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let tpl = if is_tty || is_json {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;
        backend.add_raw_message(folder, &msg).await?;

        printer.print(format!("Template successfully saved to {folder}!"))
    }
}
