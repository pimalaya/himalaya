//! Account handlers module.
//!
//! This module gathers all account actions triggered by the CLI.

use anyhow::Result;
#[cfg(feature = "imap-backend")]
use email::backend::ImapAuthConfig;
#[cfg(feature = "smtp-sender")]
use email::sender::SmtpAuthConfig;
use email::{
    account::{
        sync::{AccountSyncBuilder, AccountSyncProgressEvent},
        AccountConfig,
    },
    backend::BackendConfig,
    sender::SenderConfig,
};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
use log::{info, trace, warn};
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};

use crate::{
    config::{
        wizard::{prompt_passwd, prompt_secret},
        DeserializedConfig,
    },
    printer::{PrintTableOpts, Printer},
    Accounts,
};

const MAIN_PROGRESS_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template(" {spinner:.dim} {msg:.dim}\n {wide_bar:.cyan/blue} \n").unwrap()
});

const SUB_PROGRESS_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template(
        "   {prefix:.bold} — {wide_msg:.dim} \n   {wide_bar:.black/black} {percent}% ",
    )
    .unwrap()
});

const SUB_PROGRESS_DONE_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template("   {prefix:.bold} \n   {wide_bar:.green} {percent}% ").unwrap()
});

/// Configure the current selected account
pub async fn configure(config: &AccountConfig, reset: bool) -> Result<()> {
    info!("entering the configure account handler");

    if reset {
        #[cfg(feature = "imap-backend")]
        if let BackendConfig::Imap(imap_config) = &config.backend {
            let reset = match &imap_config.auth {
                ImapAuthConfig::Passwd(passwd) => passwd.reset(),
                ImapAuthConfig::OAuth2(oauth2) => oauth2.reset(),
            };
            if let Err(err) = reset {
                warn!("error while resetting imap secrets, skipping it");
                warn!("{err}");
            }
        }

        #[cfg(feature = "smtp-sender")]
        if let SenderConfig::Smtp(smtp_config) = &config.sender {
            let reset = match &smtp_config.auth {
                SmtpAuthConfig::Passwd(passwd) => passwd.reset(),
                SmtpAuthConfig::OAuth2(oauth2) => oauth2.reset(),
            };
            if let Err(err) = reset {
                warn!("error while resetting smtp secrets, skipping it");
                warn!("{err}");
            }
        }

        #[cfg(feature = "pgp")]
        config.pgp.reset().await?;
    }

    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = &config.backend {
        match &imap_config.auth {
            ImapAuthConfig::Passwd(passwd) => {
                passwd.configure(|| prompt_passwd("IMAP password")).await
            }
            ImapAuthConfig::OAuth2(oauth2) => {
                oauth2
                    .configure(|| prompt_secret("IMAP OAuth 2.0 client secret"))
                    .await
            }
        }?;
    }

    #[cfg(feature = "smtp-sender")]
    if let SenderConfig::Smtp(smtp_config) = &config.sender {
        match &smtp_config.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                passwd.configure(|| prompt_passwd("SMTP password")).await
            }
            SmtpAuthConfig::OAuth2(oauth2) => {
                oauth2
                    .configure(|| prompt_secret("SMTP OAuth 2.0 client secret"))
                    .await
            }
        }?;
    }

    #[cfg(feature = "pgp")]
    config
        .pgp
        .configure(&config.email, || prompt_passwd("PGP secret key password"))
        .await?;

    println!(
        "Account successfully {}configured!",
        if reset { "re" } else { "" }
    );

    Ok(())
}

/// Lists all accounts.
pub fn list<'a, P: Printer>(
    max_width: Option<usize>,
    config: &AccountConfig,
    deserialized_config: &DeserializedConfig,
    printer: &mut P,
) -> Result<()> {
    info!("entering the list accounts handler");

    let accounts: Accounts = deserialized_config.accounts.iter().into();
    trace!("accounts: {:?}", accounts);

    printer.print_table(
        Box::new(accounts),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )?;

    info!("<< account list handler");
    Ok(())
}

/// Synchronizes the account defined using argument `-a|--account`. If
/// no account given, synchronizes the default one.
pub async fn sync<P: Printer>(
    printer: &mut P,
    sync_builder: AccountSyncBuilder,
    dry_run: bool,
) -> Result<()> {
    info!("entering the sync accounts handler");
    trace!("dry run: {dry_run}");

    if dry_run {
        let report = sync_builder.sync().await?;
        let mut hunks_count = report.folders_patch.len();

        if !report.folders_patch.is_empty() {
            printer.print_log("Folders patch:")?;
            for (hunk, _) in report.folders_patch {
                printer.print_log(format!(" - {hunk}"))?;
            }
            printer.print_log("")?;
        }

        if !report.emails_patch.is_empty() {
            printer.print_log("Envelopes patch:")?;
            for (hunk, _) in report.emails_patch {
                hunks_count += 1;
                printer.print_log(format!(" - {hunk}"))?;
            }
            printer.print_log("")?;
        }

        printer.print(format!(
            "Estimated patch length for account to be synchronized: {hunks_count}",
        ))?;
    } else if printer.is_json() {
        sync_builder.sync().await?;
        printer.print("Account successfully synchronized!")?;
    } else {
        let multi = MultiProgress::new();
        let sub_progresses = Mutex::new(HashMap::new());
        let main_progress = multi.add(
            ProgressBar::new(100)
                .with_style(MAIN_PROGRESS_STYLE.clone())
                .with_message("Synchronizing folders…"),
        );

        // Force the progress bar to show
        main_progress.set_position(0);

        let report = sync_builder
            .with_on_progress(move |evt| {
                use AccountSyncProgressEvent::*;
                Ok(match evt {
                    ApplyFolderPatches(..) => {
                        main_progress.inc(3);
                    }
                    ApplyEnvelopePatches(patches) => {
                        let mut envelopes_progresses = sub_progresses.lock().unwrap();
                        let patches_len = patches.values().fold(0, |sum, patch| sum + patch.len());
                        main_progress.set_length((110 * patches_len / 100) as u64);
                        main_progress.set_position((5 * patches_len / 100) as u64);
                        main_progress.set_message("Synchronizing envelopes…");

                        for (folder, patch) in patches {
                            let progress = ProgressBar::new(patch.len() as u64)
                                .with_style(SUB_PROGRESS_STYLE.clone())
                                .with_prefix(folder.clone())
                                .with_finish(ProgressFinish::AndClear);
                            let progress = multi.add(progress);
                            envelopes_progresses.insert(folder, progress.clone());
                        }
                    }
                    ApplyEnvelopeHunk(hunk) => {
                        main_progress.inc(1);
                        let mut progresses = sub_progresses.lock().unwrap();
                        if let Some(progress) = progresses.get_mut(hunk.folder()) {
                            progress.inc(1);
                            if progress.position() == (progress.length().unwrap() - 1) {
                                progress.set_style(SUB_PROGRESS_DONE_STYLE.clone())
                            } else {
                                progress.set_message(format!("{hunk}…"));
                            }
                        }
                    }
                    ApplyEnvelopeCachePatch(_patch) => {
                        main_progress.set_length(100);
                        main_progress.set_position(95);
                        main_progress.set_message("Saving cache database…");
                    }
                    ExpungeFolders(folders) => {
                        let mut progresses = sub_progresses.lock().unwrap();
                        for progress in progresses.values() {
                            progress.finish_and_clear()
                        }
                        progresses.clear();

                        main_progress.set_position(100);
                        main_progress.set_message(format!("Expunging {} folders…", folders.len()));
                    }
                    _ => (),
                })
            })
            .sync()
            .await?;

        let folders_patch_err = report
            .folders_patch
            .iter()
            .filter_map(|(hunk, err)| err.as_ref().map(|err| (hunk, err)))
            .collect::<Vec<_>>();
        if !folders_patch_err.is_empty() {
            printer.print_log("")?;
            printer.print_log("Errors occurred while applying the folders patch:")?;
            folders_patch_err
                .iter()
                .try_for_each(|(hunk, err)| printer.print_log(format!(" - {hunk}: {err}")))?;
        }

        if let Some(err) = report.folders_cache_patch.1 {
            printer.print_log("")?;
            printer.print_log(format!(
                "Error occurred while applying the folder cache patch: {err}"
            ))?;
        }

        let envelopes_patch_err = report
            .emails_patch
            .iter()
            .filter_map(|(hunk, err)| err.as_ref().map(|err| (hunk, err)))
            .collect::<Vec<_>>();
        if !envelopes_patch_err.is_empty() {
            printer.print_log("")?;
            printer.print_log("Errors occurred while applying the envelopes patch:")?;
            for (hunk, err) in folders_patch_err {
                printer.print_log(format!(" - {hunk}: {err}"))?;
            }
        }

        if let Some(err) = report.emails_cache_patch.1 {
            printer.print_log("")?;
            printer.print_log(format!(
                "Error occurred while applying the envelopes cache patch: {err}"
            ))?;
        }

        printer.print("Account successfully synchronized!")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use email::{account::AccountConfig, backend::ImapConfig};
    use std::{collections::HashMap, fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::{
        account::DeserializedAccountConfig,
        printer::{Print, PrintTable, WriteColor},
    };

    use super::*;

    #[test]
    fn it_should_match_cmds_accounts() {
        #[derive(Debug, Default, Clone)]
        struct StringWriter {
            content: String,
        }

        impl io::Write for StringWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.content
                    .push_str(&String::from_utf8(buf.to_vec()).unwrap());
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                self.content = String::default();
                Ok(())
            }
        }

        impl termcolor::WriteColor for StringWriter {
            fn supports_color(&self) -> bool {
                false
            }

            fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
                io::Result::Ok(())
            }

            fn reset(&mut self) -> io::Result<()> {
                io::Result::Ok(())
            }
        }

        impl WriteColor for StringWriter {}

        #[derive(Debug, Default)]
        struct PrinterServiceTest {
            pub writer: StringWriter,
        }

        impl Printer for PrinterServiceTest {
            fn print_table<T: Debug + PrintTable + erased_serde::Serialize + ?Sized>(
                &mut self,
                data: Box<T>,
                opts: PrintTableOpts,
            ) -> Result<()> {
                data.print_table(&mut self.writer, opts)?;
                Ok(())
            }
            fn print_log<T: Debug + Print>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn print<T: Debug + Print + serde::Serialize>(&mut self, _data: T) -> Result<()> {
                unimplemented!()
            }
            fn is_json(&self) -> bool {
                unimplemented!()
            }
        }

        let mut printer = PrinterServiceTest::default();
        let config = AccountConfig::default();
        let deserialized_config = DeserializedConfig {
            accounts: HashMap::from_iter([(
                "account-1".into(),
                DeserializedAccountConfig {
                    default: Some(true),
                    backend: BackendConfig::Imap(ImapConfig::default()),
                    ..DeserializedAccountConfig::default()
                },
            )]),
            ..DeserializedConfig::default()
        };

        assert!(list(None, &config, &deserialized_config, &mut printer).is_ok());
        assert_eq!(
            concat![
                "\n",
                "NAME      │BACKEND │DEFAULT \n",
                "account-1 │imap    │yes     \n",
                "\n"
            ],
            printer.writer.content
        );
    }
}
