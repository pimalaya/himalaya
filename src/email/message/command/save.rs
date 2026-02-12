use std::{
    fmt,
    io::{self, BufRead, IsTerminal},
    sync::Arc,
};

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config, envelope::SingleId};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag, message::arg::MessageRawArg,
};

/// Save the given raw message to the given folder.
///
/// This command allows you to add a raw message to the given folder.
#[derive(Debug, Parser)]
pub struct MessageSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub message: MessageRawArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing save message command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            Arc::new(account_config),
            |builder| {
                builder
                    .without_features()
                    .with_add_message(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let msg = if is_tty || is_json {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        let id = backend.add_message(folder, msg.as_bytes()).await?;

        printer.out(MessageAdded { folder, id })
    }
}

struct MessageAdded<'a> {
    folder: &'a String,
    id: SingleId,
}

impl fmt::Display for MessageAdded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.id.as_str();
        let folder = self.folder;
        writeln!(f, "Message {id} successfully saved to {folder}")
    }
}

impl Serialize for MessageAdded<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("MessageAdded", 2)?;
        state.serialize_field("folder", self.folder)?;
        state.serialize_field("id", self.id.as_str())?;
        state.end()
    }
}
