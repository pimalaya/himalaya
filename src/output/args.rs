//! Module related to output CLI.
//!
//! This module provides arguments related to output.

use clap::Arg;

pub(crate) const ARG_COLOR: &str = "color";
pub(crate) const ARG_OUTPUT: &str = "output";

/// Output arguments.
pub fn global_args() -> impl IntoIterator<Item = Arg> {
    [
        Arg::new(ARG_OUTPUT)
            .help("Define the output format")
            .long("output")
            .short('o')
            .global(true)
            .value_name("format")
            .value_parser(["plain", "json"])
            .default_value("plain"),
        Arg::new(ARG_COLOR)
            .help("Control when to use colors")
            .long_help(
                "Control when to use colors.

The default setting is 'auto', which means himalaya will try to guess
when to use colors. For example, if himalaya is printing to a
terminal, then it will use colors, but if it is redirected to a file
or a pipe, then it will suppress color output. himalaya will suppress
color output in some other circumstances as well. For example, if the
TERM environment variable is not set or set to 'dumb', then himalaya
will not use colors.

The possible values for this flag are:

never    Colors will never be used.
auto     The default. himalaya tries to be smart.
always   Colors will always be used regardless of where output is sent.
ansi     Like 'always', but emits ANSI escapes (even in a Windows console).",
            )
            .long("color")
            .short('C')
            .global(true)
            .value_parser(["never", "auto", "always", "ansi"])
            .default_value("auto")
            .value_name("mode"),
    ]
}
