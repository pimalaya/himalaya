//! Module related to output CLI.
//!
//! This module provides arguments related to output.

use clap::Arg;

/// Output arguments.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name("output")
            .long("output")
            .short("o")
            .help("Defines the output format")
            .value_name("FMT")
            .possible_values(&["plain", "json"])
            .default_value("plain"),
        Arg::with_name("log-level")
            .long("log-level")
            .alias("log")
            .short("l")
            .help("Defines the logs level")
            .value_name("LEVEL")
            .possible_values(&["error", "warn", "info", "debug", "trace"])
            .default_value("info"),
        Arg::with_name("color")
            .long("color")
            .help(
                "
This flag controls when to use colors. The default setting is 'auto', which
means himalaya will try to guess when to use colors. For example, if himalaya is
printing to a terminal, then it will use colors, but if it is redirected to a
file or a pipe, then it will suppress color output. himalaya will suppress color
output in some other circumstances as well. For example, if the TERM
environment variable is not set or set to 'dumb', then himalaya will not use
colors.

The possible values for this flag are:

never    Colors will never be used.
auto     The default. himalaya tries to be smart.
always   Colors will always be used regardless of where output is sent.
ansi     Like 'always', but emits ANSI escapes (even in a Windows console).
",
            )
            .possible_values(&["never", "auto", "always", "ansi"])
            .default_value("auto")
            .value_name("WHEN"),
    ]
}
