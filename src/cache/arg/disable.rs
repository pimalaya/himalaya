use clap::Parser;

/// The disable cache flag parser.
#[derive(Debug, Default, Parser)]
pub struct CacheDisableFlag {
    /// Disable any sort of cache.
    ///
    /// The action depends on commands it apply on. For example, when
    /// listing envelopes using the IMAP backend, this flag will
    /// ensure that envelopes are fetched from the IMAP server rather
    /// than the synchronized local Maildir.
    #[arg(long = "disable-cache", alias = "no-cache", global = true)]
    #[arg(name = "cache_disable")]
    pub disable: bool,
}
