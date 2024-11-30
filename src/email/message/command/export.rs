use std::{
    fs,
    io::{stdout, Write},
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{himalaya::backend::BackendBuilder, terminal::config::TomlConfig as _};
use tracing::info;

use crate::envelope::arg::ids::EnvelopeIdArg;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Export a message.
///
/// This command allows you to export a message. When exporting a message,
/// the "seen" flag is automatically applied to the corresponding
/// envelope. To prevent this behaviour, use the --preview flag.
#[derive(Debug, Parser)]
pub struct MessageExportCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    /// Export the full raw message as one unique .eml file.
    ///
    /// The raw message represents the headers and the body as it is
    /// on the backend, unedited: not decoded nor decrypted. This is
    /// useful for debugging faulty messages, but also for
    /// saving/sending/transfering messages.
    #[arg(long, short = 'F')]
    pub full: bool,

    /// Where the message should be exported into.
    ///
    /// The destination should point to a valid directory. If `--full`
    /// is given, it can also point to a .eml file.
    #[arg(long, short, alias = "dest")]
    pub destination: Option<PathBuf>,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageExportCommand {
    pub async fn execute(self, config: &TomlConfig) -> Result<()> {
        info!("executing export message command");

        let folder = &self.folder.name;
        let id = &self.envelope.id;

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
                    .with_get_messages(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let msgs = backend.get_messages(folder, &[*id]).await?;
        let msg = msgs.first().ok_or(eyre!("cannot find message {id}"))?;

        if self.full {
            let bytes = msg.raw()?;

            match self.destination {
                Some(mut dest) if dest.is_dir() => {
                    dest.push(format!("{id}.eml"));
                    fs::write(&dest, bytes)?;
                    let dest = dest.display();
                    println!("Message {id} successfully exported at {dest}!\n");
                }
                Some(dest) => {
                    fs::write(&dest, bytes)?;
                    let dest = dest.display();
                    println!("Message {id} successfully exported at {dest}!\n");
                }
                None => {
                    stdout().write_all(bytes)?;
                }
            };
        } else {
            let msg = msg.parsed()?;

            for part in &msg.parts {
                println!("part: {:#?}", part.body);
            }
        }

        Ok(())
    }
}
