//! # mbox flag argument
//!
//! The argument naming one of the six mbox flags.

use clap::ValueEnum;
use io_mbox::flag::MboxFlag;

/// One of the six flags the mbox status headers carry.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    /// `Status: R`: the message has been read.
    Seen,
    /// `Status: O`: a client saw the message arrive, it is no longer
    /// recent.
    Old,
    /// `X-Status: A`: the message has been replied to.
    Answered,
    /// `X-Status: F`: the message is marked for attention.
    Flagged,
    /// `X-Status: T`: the message is an unsent draft.
    Draft,
    /// `X-Status: D`: the message is marked for deletion.
    Deleted,
}

impl From<FlagArg> for MboxFlag {
    fn from(flag: FlagArg) -> Self {
        match flag {
            FlagArg::Seen => MboxFlag::Seen,
            FlagArg::Old => MboxFlag::Old,
            FlagArg::Answered => MboxFlag::Answered,
            FlagArg::Flagged => MboxFlag::Flagged,
            FlagArg::Draft => MboxFlag::Draft,
            FlagArg::Deleted => MboxFlag::Deleted,
        }
    }
}
