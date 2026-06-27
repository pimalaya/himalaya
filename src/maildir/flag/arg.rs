use clap::ValueEnum;
use io_maildir::flag::types::MaildirFlag;

/// CLI value selecting one of the six standard Maildir flags.
#[derive(Clone, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FlagArg {
    Passed,
    Replied,
    Seen,
    Trashed,
    Draft,
    Flagged,
}

impl From<FlagArg> for MaildirFlag {
    fn from(flag: FlagArg) -> Self {
        match flag {
            FlagArg::Passed => MaildirFlag::Passed,
            FlagArg::Replied => MaildirFlag::Replied,
            FlagArg::Seen => MaildirFlag::Seen,
            FlagArg::Trashed => MaildirFlag::Trashed,
            FlagArg::Draft => MaildirFlag::Draft,
            FlagArg::Flagged => MaildirFlag::Flagged,
        }
    }
}
