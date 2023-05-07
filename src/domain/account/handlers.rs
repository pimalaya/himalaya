//! Account handlers module.
//!
//! This module gathers all account actions triggered by the CLI.

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, trace, warn};
use pimalaya_email::{
    folder::sync::Strategy as SyncFoldersStrategy, AccountConfig, Backend, BackendConfig,
    BackendSyncBuilder, BackendSyncProgressEvent, EmailSender, ImapAuthConfig, SmtpAuthConfig,
};

use crate::{
    config::{
        wizard::{prompt_passwd, prompt_secret},
        DeserializedConfig,
    },
    printer::{PrintTableOpts, Printer},
    Accounts,
};

/// Configure the current selected account
pub fn configure(
    account_config: &AccountConfig,
    backend_config: &BackendConfig,
    reset: bool,
) -> Result<()> {
    info!("entering the configure account handler");

    if reset {
        #[cfg(feature = "imap-backend")]
        if let BackendConfig::Imap(imap_config) = backend_config {
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
        if let EmailSender::Smtp(smtp_config) = &account_config.email_sender {
            let reset = match &smtp_config.auth {
                SmtpAuthConfig::Passwd(passwd) => passwd.reset(),
                SmtpAuthConfig::OAuth2(oauth2) => oauth2.reset(),
            };
            if let Err(err) = reset {
                warn!("error while resetting smtp secrets, skipping it");
                warn!("{err}");
            }
        }
    }

    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = backend_config {
        match &imap_config.auth {
            ImapAuthConfig::Passwd(passwd) => {
                passwd.configure(|| prompt_passwd("Enter your IMAP password:"))
            }
            ImapAuthConfig::OAuth2(oauth2) => {
                oauth2.configure(|| prompt_secret("Enter your IMAP OAuth 2.0 client secret:"))
            }
        }?;
    }

    #[cfg(feature = "smtp-sender")]
    if let EmailSender::Smtp(smtp_config) = &account_config.email_sender {
        match &smtp_config.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                passwd.configure(|| prompt_passwd("Enter your SMTP password:"))
            }
            SmtpAuthConfig::OAuth2(oauth2) => {
                oauth2.configure(|| prompt_secret("Enter your SMTP OAuth 2.0 client secret:"))
            }
        }?;
    }

    println!("Account successfully configured!");
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
pub fn sync<P: Printer>(
    account_config: &AccountConfig,
    printer: &mut P,
    backend: &dyn Backend,
    folders_strategy: Option<SyncFoldersStrategy>,
    dry_run: bool,
) -> Result<()> {
    info!("entering the sync accounts handler");
    trace!("dry run: {dry_run}");
    trace!("folders strategy: {folders_strategy:#?}");

    let mut sync_builder = BackendSyncBuilder::new(account_config);

    if let Some(strategy) = folders_strategy {
        sync_builder = sync_builder.folders_strategy(strategy);
    }

    if dry_run {
        let report = sync_builder.dry_run(true).sync(backend)?;
        let mut hunks_count = report.folders_patch.len();

        if !report.folders_patch.is_empty() {
            printer.print_log("Folders patch:")?;
            for (hunk, _) in report.folders_patch {
                printer.print_log(format!(" - {hunk}"))?;
            }
            printer.print_log("")?;
        }

        if !report.envelopes_patch.is_empty() {
            printer.print_log("Envelopes patch:")?;
            for (hunk, _) in report.envelopes_patch {
                hunks_count += 1;
                printer.print_log(format!(" - {hunk}"))?;
            }
            printer.print_log("")?;
        }

        printer.print(format!(
            "Estimated patch length for account {} to be synchronized: {hunks_count}",
            backend.name(),
        ))?;
    } else if printer.is_json() {
        sync_builder.sync(backend)?;
        printer.print(format!(
            "Account {} successfully synchronized!",
            backend.name()
        ))?;
    } else {
        let multi = MultiProgress::new();
        let progress = multi.add(
            ProgressBar::new(0).with_style(
                ProgressStyle::with_template(
                    " {spinner:.dim} {msg:.dim}\n {wide_bar:.cyan/blue} {pos}/{len} ",
                )
                .unwrap(),
            ),
        );

        let report = sync_builder
            .on_progress(|evt| {
                use BackendSyncProgressEvent::*;
                Ok(match evt {
                    GetLocalCachedFolders => {
                        progress.set_length(4);
                        progress.set_position(0);
                        progress.set_message("Getting local cached folders…");
                    }
                    GetLocalFolders => {
                        progress.inc(1);
                        progress.set_message("Getting local maildir folders…");
                    }
                    GetRemoteCachedFolders => {
                        progress.inc(1);
                        progress.set_message("Getting remote cached folders…");
                    }
                    GetRemoteFolders => {
                        progress.inc(1);
                        progress.set_message("Getting remote folders…");
                    }
                    BuildFoldersPatch => {
                        progress.inc(1);
                        progress.set_message("Building patch…");
                    }
                    ProcessFoldersPatch(n) => {
                        progress.set_length(n as u64);
                        progress.set_position(0);
                        progress.set_message("Processing patch…");
                    }
                    ProcessFolderHunk(msg) => {
                        progress.inc(1);
                        progress.set_message(msg + "…");
                    }
                    StartEnvelopesSync(folder, n, len) => {
                        multi.println(format!("[{n:2}/{len}] {folder}")).unwrap();
                        progress.reset();
                    }
                    GetLocalCachedEnvelopes => {
                        progress.set_length(4);
                        progress.set_message("Getting local cached envelopes…");
                    }
                    GetLocalEnvelopes => {
                        progress.inc(1);
                        progress.set_message("Getting local maildir envelopes…");
                    }
                    GetRemoteCachedEnvelopes => {
                        progress.inc(1);
                        progress.set_message("Getting remote cached envelopes…");
                    }
                    GetRemoteEnvelopes => {
                        progress.inc(1);
                        progress.set_message("Getting remote envelopes…");
                    }
                    BuildEnvelopesPatch => {
                        progress.inc(1);
                        progress.set_message("Building patch…");
                    }
                    ProcessEnvelopesPatch(n) => {
                        progress.set_length(n as u64);
                        progress.set_position(0);
                        progress.set_message("Processing patch…");
                    }
                    ProcessEnvelopeHunk(msg) => {
                        progress.inc(1);
                        progress.set_message(msg + "…");
                    }
                })
            })
            .sync(backend)?;

        progress.finish_and_clear();

        let folders_patch_err = report
            .folders_patch
            .iter()
            .filter_map(|(hunk, err)| err.as_ref().map(|err| (hunk, err)))
            .collect::<Vec<_>>();
        if !folders_patch_err.is_empty() {
            printer.print_log("")?;
            printer.print_log("Errors occured while applying the folders patch:")?;
            folders_patch_err
                .iter()
                .try_for_each(|(hunk, err)| printer.print_log(format!(" - {hunk}: {err}")))?;
        }

        if let Some(err) = report.folders_cache_patch.1 {
            printer.print_log("")?;
            printer.print_log(format!(
                "Error occured while applying the folder cache patch: {err}"
            ))?;
        }

        let envelopes_patch_err = report
            .envelopes_patch
            .iter()
            .filter_map(|(hunk, err)| err.as_ref().map(|err| (hunk, err)))
            .collect::<Vec<_>>();
        if !envelopes_patch_err.is_empty() {
            printer.print_log("")?;
            printer.print_log("Errors occured while applying the envelopes patch:")?;
            for (hunk, err) in folders_patch_err {
                printer.print_log(format!(" - {hunk}: {err}"))?;
            }
        }

        if !report.envelopes_cache_patch.1.is_empty() {
            printer.print_log("")?;
            printer.print_log("Error occured while applying the envelopes cache patch:")?;
            for err in report.envelopes_cache_patch.1 {
                printer.print_log(format!(" - {err}"))?;
            }
        }

        printer.print(format!(
            "Account {} successfully synchronized!",
            backend.name()
        ))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use pimalaya_email::{AccountConfig, ImapConfig};
    use std::{collections::HashMap, fmt::Debug, io};
    use termcolor::ColorSpec;

    use crate::{
        account::{
            DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedImapAccountConfig,
        },
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
                DeserializedAccountConfig::Imap(DeserializedImapAccountConfig {
                    base: DeserializedBaseAccountConfig {
                        default: Some(true),
                        ..DeserializedBaseAccountConfig::default()
                    },
                    backend: ImapConfig::default(),
                }),
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
