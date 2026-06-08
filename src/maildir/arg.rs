use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use io_maildir::maildir::types::MaildirSubdir;

const INBOX: &str = "Inbox";

#[derive(Debug, Parser)]
pub struct MaildirNameArg {
    /// Name of the Maildir.
    #[arg(name = "maildir_name", value_name = "NAME")]
    pub inner: String,
}

#[derive(Debug, Parser)]
pub struct MaildirPathFlag {
    /// Path to the source Maildir.
    #[arg(name = "maildir_source_path", long = "maildir", short = 'm')]
    #[arg(value_name = "PATH", default_value = INBOX)]
    pub inner: PathBuf,
}

#[derive(Debug, Parser)]
pub struct TargetMaildirPathFlag {
    /// Path to the target Maildir.
    #[arg(name = "maildir_target_path", long = "target", short = 't')]
    #[arg(value_name = "PATH")]
    pub inner: PathBuf,
}

#[derive(Debug, Parser)]
pub struct MessageIdArg {
    /// Identifier of the message
    #[arg(name = "message_id", value_name = "ID")]
    pub inner: String,
}

#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s).
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
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
