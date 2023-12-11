use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use std::{fs, path::PathBuf};
use uuid::Uuid;

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag, printer::Printer,
};

/// Download all attachments for the given message.
///
/// This command allows you to download all attachments found for the
/// given message to your downloads directory.
#[derive(Debug, Parser)]
pub struct AttachmentDownloadCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl AttachmentDownloadCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing attachment download command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let ids = &self.envelopes.ids;
        let emails = backend.get_messages(&folder, ids).await?;

        let mut emails_count = 0;
        let mut attachments_count = 0;

        let mut ids = ids.iter();
        for email in emails.to_vec() {
            let id = ids.next().unwrap();
            let attachments = email.attachments()?;

            if attachments.is_empty() {
                printer.print_log(format!("No attachment found for message {id}!"))?;
                continue;
            } else {
                emails_count += 1;
            }

            printer.print_log(format!(
                "{} attachment(s) found for message {id}!",
                attachments.len()
            ))?;

            for attachment in attachments {
                let filename: PathBuf = attachment
                    .filename
                    .unwrap_or_else(|| Uuid::new_v4().to_string())
                    .into();
                let filepath = account_config.get_download_file_path(&filename)?;
                printer.print_log(format!("Downloading {:?}…", filepath))?;
                fs::write(&filepath, &attachment.body)
                    .with_context(|| format!("cannot save attachment at {filepath:?}"))?;
                attachments_count += 1;
            }
        }

        match attachments_count {
            0 => printer.print("No attachment found!"),
            1 => printer.print("Downloaded 1 attachment!"),
            n => printer.print(format!(
                "Downloaded {} attachment(s) from {} messages(s)!",
                n, emails_count,
            )),
        }
    }
}
