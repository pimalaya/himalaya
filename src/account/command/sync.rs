use anyhow::Result;
use clap::{ArgAction, Parser};
use email::{
    account::sync::{AccountSyncBuilder, AccountSyncProgressEvent},
    folder::sync::FolderSyncStrategy,
};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
use log::info;
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use crate::{backend::BackendBuilder, config::TomlConfig, printer::Printer};

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

#[derive(Debug, Parser)]
pub struct Command {
    /// The name of the account that needs to be synchronized
    ///
    /// The account names are taken from the table at the root level
    /// of your TOML configuration file.
    #[arg(value_name = "ACCOUNT")]
    pub account_name: String,

    /// Run the synchronization without applying changes
    ///
    /// Instead, a report will be printed to stdout containing all the
    /// changes the synchronization plan to do.
    #[arg(long, short)]
    pub dry_run: bool,

    #[arg(long, short = 'f', value_name = "FOLDER", action = ArgAction::Append, conflicts_with = "exclude_folder", conflicts_with = "all_folders")]
    pub include_folder: Vec<String>,

    #[arg(long, short = 'x', value_name = "FOLDER", action = ArgAction::Append, conflicts_with = "include_folder", conflicts_with = "all_folders")]
    pub exclude_folder: Vec<String>,

    #[arg(
        long,
        short = 'A',
        conflicts_with = "include_folder",
        conflicts_with = "exclude_folder"
    )]
    pub all_folders: bool,
}

impl Command {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing account sync command");

        let included_folders = HashSet::from_iter(self.include_folder);
        let excluded_folders = HashSet::from_iter(self.exclude_folder);

        let strategy = if !included_folders.is_empty() {
            Some(FolderSyncStrategy::Include(included_folders))
        } else if !excluded_folders.is_empty() {
            Some(FolderSyncStrategy::Exclude(excluded_folders))
        } else if self.all_folders {
            Some(FolderSyncStrategy::All)
        } else {
            None
        };

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(Some(self.account_name.as_str()), true)?;

        let backend_builder =
            BackendBuilder::new(toml_account_config, account_config.clone(), false).await?;
        let sync_builder = AccountSyncBuilder::new(backend_builder.into())
            .await?
            .with_some_folders_strategy(strategy)
            .with_dry_run(self.dry_run);

        if self.dry_run {
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
                            let patches_len =
                                patches.values().fold(0, |sum, patch| sum + patch.len());
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
                            main_progress
                                .set_message(format!("Expunging {} folders…", folders.len()));
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
}
