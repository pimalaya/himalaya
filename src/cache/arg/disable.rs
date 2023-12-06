use clap::Parser;

/// The disable cache flag parser
#[derive(Debug, Parser)]
pub struct DisableCacheFlag {
    /// Disable any sort of cache
    ///
    /// The action depends on commands it apply on. For example, when
    /// listing envelopes using the IMAP backend, this flag will
    /// ensure that envelopes are fetched from the IMAP server and not
    /// from the synchronized local Maildir.
    #[arg(
        long = "disable-cache",
        alias = "no-cache",
        name = "disable-cache",
        global = true
    )]
    pub disable: bool,
}
