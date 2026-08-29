//! # Maildir argument
//!
//! The argument naming a Maildir the commands act on.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use io_maildir::maildir::MaildirSubdir;

/// The Maildir an argument defaults to.
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
    /// Maildir folder, resolved relative to the account root. Must name
    /// an existing folder; use `.` for the root maildir itself (the
    /// INBOX in the default fs layout, where there is no `Inbox`
    /// subfolder). Defaults to `Inbox`.
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

/// A subdirectory of a Maildir.
#[derive(Clone, Debug, ValueEnum)]
pub enum MaildirSubdirArg {
    /// Where a message a client has seen lives.
    Cur,
    /// Where a freshly delivered message lives.
    New,
    /// Where a delivery is staged before its rename.
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
