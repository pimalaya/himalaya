use clap::ValueEnum;
use io_maildir::flag::Flag;

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

impl From<FlagArg> for Flag {
    fn from(flag: FlagArg) -> Self {
        match flag {
            FlagArg::Passed => Flag::Passed,
            FlagArg::Replied => Flag::Replied,
            FlagArg::Seen => Flag::Seen,
            FlagArg::Trashed => Flag::Trashed,
            FlagArg::Draft => Flag::Draft,
            FlagArg::Flagged => Flag::Flagged,
        }
    }
}
