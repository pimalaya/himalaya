use clap::{ArgAction, Parser};
use color_eyre::{eyre::bail, eyre::eyre, Result};
use email::backend::context::BackendContextBuilder;
#[cfg(feature = "imap")]
use email::imap::ImapContextBuilder;
use email::{
    account::sync::AccountSyncBuilder,
    backend::BackendBuilder,
    folder::sync::config::FolderSyncStrategy,
    sync::{hash::SyncHash, SyncEvent},
};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};
use tracing::info;

use crate::{
    account::arg::name::OptionalAccountNameArg, backend::BackendKind, config::TomlConfig,
    printer::Printer,
};

static MAIN_PROGRESS_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template(" {spinner:.dim} {msg:.dim}\n {wide_bar:.cyan/blue} \n").unwrap()
});

static SUB_PROGRESS_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template(
        "   {prefix:.bold} — {wide_msg:.dim} \n   {wide_bar:.black/black} {percent}% ",
    )
    .unwrap()
});

static SUB_PROGRESS_DONE_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template("   {prefix:.bold} \n   {wide_bar:.green} {percent}% ").unwrap()
});

/// Synchronize an account.
///
/// This command allows you to synchronize all folders and emails
/// (including envelopes and messages) of a given account into a local
/// Maildir folder.
#[derive(Debug, Parser)]
pub struct AccountSyncCommand {
    #[command(flatten)]
    pub account: OptionalAccountNameArg,

    /// Run the synchronization without applying any changes.
    ///
    /// Instead, a report will be printed to stdout containing all the
    /// changes the synchronization plan to do.
    #[arg(long, short)]
    pub dry_run: bool,

    /// Synchronize only specific folders.
    ///
    /// Only the given folders will be synchronized (including
    /// associated envelopes and messages). Useful when you need to
    /// speed up the synchronization process. A good usecase is to
    /// synchronize only the INBOX in order to quickly check for new
    /// messages.
    #[arg(long, short = 'f')]
    #[arg(value_name = "FOLDER", action = ArgAction::Append)]
    #[arg(conflicts_with = "exclude_folder", conflicts_with = "all_folders")]
    pub include_folder: Vec<String>,

    /// Omit specific folders from the synchronization.
    ///
    /// The given folders will be excluded from the synchronization
    /// (including associated envelopes and messages). Useful when you
    /// have heavy folders that you do not want to take care of, or to
    /// speed up the synchronization process.
    #[arg(long, short = 'x')]
    #[arg(value_name = "FOLDER", action = ArgAction::Append)]
    #[arg(conflicts_with = "include_folder", conflicts_with = "all_folders")]
    pub exclude_folder: Vec<String>,

    /// Synchronize all exsting folders.
    #[arg(long, short = 'A')]
    #[arg(conflicts_with = "include_folder", conflicts_with = "exclude_folder")]
    pub all_folders: bool,
}

impl AccountSyncCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing sync account command");

        let account = self.account.name.as_deref();
        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, true)?;
        let account_name = account_config.name.as_str();

        match toml_account_config.sync_kind() {
            Some(BackendKind::Imap) | Some(BackendKind::ImapCache) => {
                let imap_config = toml_account_config
                    .imap
                    .as_ref()
                    .map(Clone::clone)
                    .map(Arc::new)
                    .ok_or_else(|| eyre!("imap config not found"))?;
                let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config)
                    .with_prebuilt_credentials()
                    .await?;
                let imap = BackendBuilder::new(account_config.clone(), imap_ctx);
                self.sync(printer, account_name, imap).await
            }
            Some(backend) => bail!("backend {backend:?} not supported for synchronization"),
            None => bail!("no backend configured for synchronization"),
        }
    }

    async fn sync(
        self,
        printer: &mut impl Printer,
        account_name: &str,
        right: BackendBuilder<impl BackendContextBuilder + SyncHash + 'static>,
    ) -> Result<()> {
        let included_folders = BTreeSet::from_iter(self.include_folder);
        let excluded_folders = BTreeSet::from_iter(self.exclude_folder);

        let folder_filters = if !included_folders.is_empty() {
            Some(FolderSyncStrategy::Include(included_folders))
        } else if !excluded_folders.is_empty() {
            Some(FolderSyncStrategy::Exclude(excluded_folders))
        } else if self.all_folders {
            Some(FolderSyncStrategy::All)
        } else {
            None
        };

        let sync_builder =
            AccountSyncBuilder::try_new(right)?.with_some_folder_filters(folder_filters);

        if self.dry_run {
            let report = sync_builder.with_dry_run(true).sync().await?;
            let mut hunks_count = report.folder.patch.len();

            if !report.folder.patch.is_empty() {
                printer.print_log("Folders patch:")?;
                for (hunk, _) in report.folder.patch {
                    printer.print_log(format!(" - {hunk}"))?;
                }
                printer.print_log("")?;
            }

            if !report.email.patch.is_empty() {
                printer.print_log("Envelopes patch:")?;
                for (hunk, _) in report.email.patch {
                    hunks_count += 1;
                    printer.print_log(format!(" - {hunk}"))?;
                }
                printer.print_log("")?;
            }

            printer.print(format!(
                "Estimated patch length for account {account_name} to be synchronized: {hunks_count}"
            ))?;
        } else if printer.is_json() {
            sync_builder.sync().await?;
            printer.print(format!("Account {account_name} successfully synchronized!"))?;
        } else {
            let multi = MultiProgress::new();
            let sub_progresses = Mutex::new(HashMap::new());
            let main_progress = multi.add(
                ProgressBar::new(100)
                    .with_style(MAIN_PROGRESS_STYLE.clone())
                    .with_message("Listing folders…"),
            );

            main_progress.tick();

            let report = sync_builder
                .with_handler(move |evt| {
                    match evt {
                        SyncEvent::ListedAllFolders => {
                            main_progress.set_message("Synchronizing folders…");
                        }
                        SyncEvent::ProcessedAllFolderHunks => {
                            main_progress.set_message("Listing envelopes…");
                        }
                        SyncEvent::GeneratedEmailPatch(patches) => {
                            let patches_len = patches.values().flatten().count();
                            main_progress.set_length(patches_len as u64);
                            main_progress.set_position(0);
                            main_progress.set_message("Synchronizing emails…");

                            let mut envelopes_progresses = sub_progresses.lock().unwrap();
                            for (folder, patch) in patches {
                                let progress = ProgressBar::new(patch.len() as u64)
                                    .with_style(SUB_PROGRESS_STYLE.clone())
                                    .with_prefix(folder.clone())
                                    .with_finish(ProgressFinish::AndClear);
                                let progress = multi.add(progress);
                                envelopes_progresses.insert(folder, progress.clone());
                            }
                        }
                        SyncEvent::ProcessedEmailHunk(hunk) => {
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
                        SyncEvent::ProcessedAllEmailHunks => {
                            let mut progresses = sub_progresses.lock().unwrap();
                            for progress in progresses.values() {
                                progress.finish_and_clear()
                            }
                            progresses.clear();

                            main_progress.set_length(100);
                            main_progress.set_position(100);
                            main_progress.set_message("Expunging folders…");
                        }
                        SyncEvent::ExpungedAllFolders => {
                            main_progress.finish_and_clear();
                        }
                        _ => {
                            main_progress.tick();
                        }
                    };

                    async { Ok(()) }
                })
                .sync()
                .await?;

            let folders_patch_err = report
                .folder
                .patch
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

            let envelopes_patch_err = report
                .email
                .patch
                .iter()
                .filter_map(|(hunk, err)| err.as_ref().map(|err| (hunk, err)))
                .collect::<Vec<_>>();
            if !envelopes_patch_err.is_empty() {
                printer.print_log("")?;
                printer.print_log("Errors occurred while applying the envelopes patch:")?;
                for (hunk, err) in envelopes_patch_err {
                    printer.print_log(format!(" - {hunk}: {err}"))?;
                }
            }

            printer.print(format!("Account {account_name} successfully synchronized!"))?;
        }

        Ok(())
    }
}
