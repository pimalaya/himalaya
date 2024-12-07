use std::{
    env::temp_dir,
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

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Export the message associated to the given envelope id.
///
/// This command allows you to export a message. A message can be
/// fully exported in one single file, or exported in multiple files
/// (one per MIME part found in the message). This is useful, for
/// example, to read a HTML message.
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

    /// Try to open the exported message, when applicable.
    ///
    /// This argument only works with full message export, or when
    /// HTML or plain text is present in the export.
    #[arg(long, short = 'O')]
    pub open: bool,

    /// Where the message should be exported to.
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
                    println!("Message {id} successfully exported at {dest}!");
                }
                Some(dest) => {
                    fs::write(&dest, bytes)?;
                    let dest = dest.display();
                    println!("Message {id} successfully exported at {dest}!");
                }
                None => {
                    stdout().write_all(bytes)?;
                }
            };
        } else {
            let dest = match self.destination {
                Some(dest) if dest.is_dir() => {
                    let dest = msg.download_parts(&dest)?;
                    let d = dest.display();
                    println!("Message {id} successfully exported in {d}!");
                    dest
                }
                Some(dest) if dest.is_file() => {
                    let dest = dest.parent().unwrap_or(&dest);
                    let dest = msg.download_parts(&dest)?;
                    let d = dest.display();
                    println!("Message {id} successfully exported in {d}!");
                    dest
                }
                Some(dest) => {
                    return Err(eyre!("Destination {} does not exist!", dest.display()));
                }
                None => {
                    let dest = temp_dir();
                    let dest = msg.download_parts(&dest)?;
                    let d = dest.display();
                    println!("Message {id} successfully exported in {d}!");
                    dest
                }
            };

            if self.open {
                let index_html = dest.join("index.html");
                if index_html.exists() {
                    return Ok(open::that(index_html)?);
                }

                let plain_txt = dest.join("plain.txt");
                if plain_txt.exists() {
                    return Ok(open::that(plain_txt)?);
                }

                println!("--open was passed but nothing to open, ignoring");
            }
        }

        Ok(())
    }
}
