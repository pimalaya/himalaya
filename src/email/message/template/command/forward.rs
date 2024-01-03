use anyhow::{anyhow, Result};
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::{get::imap::GetMessagesImap, peek::imap::PeekMessagesImap};
#[cfg(feature = "maildir")]
use email::{flag::add::maildir::AddFlagsMaildir, message::peek::maildir::PeekMessagesMaildir};
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
    message::arg::{body::MessageRawBodyArg, header::HeaderRawArgs},
    printer::Printer,
};

/// Generate a template for forwarding a message.
///
/// The generated template is prefilled with your email in a From
/// header as well as your signature. The forwarded message is also
/// prefilled in the body of the template, prefixed by a separator.
#[derive(Debug, Parser)]
pub struct TemplateForwardCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateForwardCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template forward command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let get_messages_kind = toml_account_config.get_messages_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            get_messages_kind,
            |builder| match get_messages_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_peek_messages(|ctx| {
                        ctx.maildir.as_ref().and_then(PeekMessagesMaildir::new)
                    });
                    builder
                        .set_add_flags(|ctx| ctx.maildir.as_ref().and_then(AddFlagsMaildir::new));
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_peek_messages(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(PeekMessagesMaildir::new)
                    });
                    builder.set_add_flags(|ctx| {
                        ctx.maildir_for_sync.as_ref().and_then(AddFlagsMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder
                        .set_peek_messages(|ctx| ctx.imap.as_ref().and_then(PeekMessagesImap::new));
                    builder
                        .set_get_messages(|ctx| ctx.imap.as_ref().and_then(GetMessagesImap::new));
                }
                _ => (),
            },
        )
        .await?;

        let id = self.envelope.id;
        let tpl: String = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(anyhow!("cannot find message {id}"))?
            .to_forward_tpl_builder(&account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?
            .into();

        printer.print(tpl)
    }
}
