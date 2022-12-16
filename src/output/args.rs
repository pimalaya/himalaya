//! Module related to output CLI.
//!
//! This module provides arguments related to output.

use clap::Arg;

/// Output arguments.
pub fn args() -> Vec<Arg> {
    vec![
        Arg::new("output")
            .help("Defines the output format")
            .long("output")
            .short('o')
            .value_name("FMT")
            .value_parser(["plain", "json"])
            .default_value("plain"),
        Arg::new("log-level")
            .help("Defines the logs level")
            .long("log-level")
            .alias("log")
            .short('l')
            .value_name("LEVEL")
            .value_parser(["error", "warn", "info", "debug", "trace"])
            .default_value("info"),
    ]
}
