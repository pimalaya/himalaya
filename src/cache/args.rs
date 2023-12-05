//! This module provides arguments related to the cache.

use clap::{Arg, ArgAction, ArgMatches};

const ARG_DISABLE_CACHE: &str = "disable-cache";

/// Represents the disable cache flag argument. This argument allows
/// the user to disable any sort of cache.
pub fn global_args() -> impl IntoIterator<Item = Arg> {
    [Arg::new(ARG_DISABLE_CACHE)
        .help("Disable any sort of cache")
        .long_help(
            "Disable any sort of cache.

The action depends on commands it apply on. For example, when listing
envelopes using the IMAP backend, this flag will ensure that envelopes
are fetched from the IMAP server and not from the synchronized local
Maildir.",
        )
        .long("disable-cache")
        .alias("no-cache")
        .global(true)
        .action(ArgAction::SetTrue)]
}

/// Represents the disable cache flag parser.
pub fn parse_disable_cache_arg(m: &ArgMatches) -> bool {
    m.get_flag(ARG_DISABLE_CACHE)
}
