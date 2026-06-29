use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use io_maildir::maildir::types::MaildirSubdir;

const INBOX: &str = "Inbox";

/// CLI argument carrying the name of a Maildir.
#[derive(Debug, Parser)]
pub struct MaildirNameArg {
    /// Name of the Maildir.
    #[arg(name = "maildir_name", value_name = "NAME")]
    pub inner: String,
}

/// CLI flag selecting the source Maildir by path.
#[derive(Debug, Parser)]
pub struct MaildirPathFlag {
    /// Path to the source Maildir.
    #[arg(name = "maildir_source_path", long = "maildir", short = 'm')]
    #[arg(value_name = "PATH", default_value = INBOX)]
    pub inner: PathBuf,
}

/// CLI flag selecting a Maildir by path, required with no default.
/// Used by destructive commands that must not silently fall back to
/// Inbox.
#[derive(Debug, Parser)]
pub struct RequiredMaildirPathFlag {
    /// Path to the Maildir.
    #[arg(name = "maildir_path", long = "maildir", short = 'm')]
    #[arg(value_name = "PATH")]
    pub inner: PathBuf,
}

/// CLI flag selecting the target Maildir by path.
#[derive(Debug, Parser)]
pub struct TargetMaildirPathFlag {
    /// Path to the target Maildir.
    #[arg(name = "maildir_target_path", long = "target", short = 't')]
    #[arg(value_name = "PATH")]
    pub inner: PathBuf,
}

/// CLI argument carrying a single message identifier.
#[derive(Debug, Parser)]
pub struct MessageIdArg {
    /// Identifier of the message
    #[arg(name = "message_id", value_name = "ID")]
    pub inner: String,
}

/// CLI argument carrying one or more message identifiers.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s).
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}

/// CLI value selecting a Maildir subdirectory: cur, new, or tmp.
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
