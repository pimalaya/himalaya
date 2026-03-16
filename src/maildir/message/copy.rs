use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use io_fs::runtimes::std::handle;
use io_maildir::{
    coroutines::copy_message::*,
    maildir::{Maildir, MaildirSubdir},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::account::MaildirAccount;

/// Copy Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct CopyMessagesCommand {
    /// Path to the source Maildir, where messages are copied from.
    #[arg(long = "source", short = 's')]
    #[arg(value_name = "PATH", default_value = "Inbox")]
    pub maildir_source_path: PathBuf,

    /// Path to the target Maildir, where messages are copied into.
    #[arg(long = "target", short = 't')]
    #[arg(value_name = "PATH")]
    pub maildir_target_path: PathBuf,
    /// Subdir of the target Maildir.
    #[arg(long = "subdir", value_name = "NAME", value_enum)]
    pub maildir_target_subdir: MaildirSubdirArg,

    /// Id(s) of message(s) to copy.
    #[arg(value_name = "ID", num_args = 1..)]
    pub ids: Vec<String>,
}

impl CopyMessagesCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir_source = match Maildir::try_from(self.maildir_source_path.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir_source_path))?,
        };

        let maildir_target = match Maildir::try_from(self.maildir_target_path.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir_target_path))?,
        };

        for id in self.ids {
            let mut arg = None;
            let mut coroutine = CopyMaildirMessage::new(
                maildir_source.clone(),
                maildir_target.clone(),
                self.maildir_target_subdir.clone().into(),
                id,
            );

            loop {
                match coroutine.resume(arg.take()) {
                    CopyMaildirMessageResult::Io(io) => arg = Some(handle(io)?),
                    CopyMaildirMessageResult::Ok => break,
                    CopyMaildirMessageResult::Err(err) => bail!(err),
                }
            }
        }

        printer.out(Message::new("Message(s) successfully copied"))
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum MaildirSubdirArg {
    Cur,
    New,
    Tmp,
}

impl From<MaildirSubdirArg> for MaildirSubdir {
    fn from(value: MaildirSubdirArg) -> Self {
        match value {
            MaildirSubdirArg::Cur => MaildirSubdir::Cur,
            MaildirSubdirArg::New => MaildirSubdir::New,
            MaildirSubdirArg::Tmp => MaildirSubdir::Tmp,
        }
    }
}
