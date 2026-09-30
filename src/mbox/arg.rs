//! # mbox argument
//!
//! The arguments naming the mbox files and messages the commands act on.

use clap::Parser;

/// The mbox a flag defaults to.
const INBOX: &str = "INBOX";

/// CLI argument carrying the name of an mbox.
#[derive(Debug, Parser)]
pub struct MboxNameArg {
    /// Name of the mbox, relative to the account root.
    #[arg(name = "mbox_name", value_name = "NAME")]
    pub inner: String,
}

/// CLI flag selecting the source mbox.
#[derive(Debug, Parser)]
pub struct MboxPathFlag {
    /// The mbox: a name relative to the account root, `INBOX` for the
    /// spool, or an absolute path to any mbox file. Defaults to `INBOX`.
    #[arg(name = "mbox_source_path", long = "mbox", short = 'm')]
    #[arg(value_name = "MBOX", default_value = INBOX)]
    pub inner: String,
}

/// CLI flag selecting an mbox, required with no default. Used by
/// destructive commands that must not silently fall back to `INBOX`.
#[derive(Debug, Parser)]
pub struct RequiredMboxPathFlag {
    /// Name of the mbox, relative to the account root.
    #[arg(name = "mbox_path", long = "mbox", short = 'm')]
    #[arg(value_name = "NAME")]
    pub inner: String,
}

/// CLI flag selecting the target mbox.
#[derive(Debug, Parser)]
pub struct TargetMboxPathFlag {
    /// The target mbox: a name relative to the account root, `INBOX`
    /// for the spool, or an absolute path to any mbox file.
    #[arg(name = "mbox_target_path", long = "target", short = 't')]
    #[arg(value_name = "MBOX")]
    pub inner: String,
}

/// CLI argument carrying one or more message identifiers.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s), as `envelope list` shows them.
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}
