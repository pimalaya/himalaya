use clap::{Arg, ArgMatches};

const ARG_MAX_TABLE_WIDTH: &str = "max-table-width";

pub(crate) type MaxTableWidth = Option<usize>;

/// Represents the max table width argument.
pub fn max_width<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_MAX_TABLE_WIDTH)
        .help("Defines a maximum width for the table")
        .short("w")
        .long("max-width")
        .value_name("INT")
}

/// Represents the max table width argument parser.
pub fn parse_max_width<'a>(matches: &'a ArgMatches<'a>) -> Option<usize> {
    matches
        .value_of(ARG_MAX_TABLE_WIDTH)
        .and_then(|width| width.parse::<usize>().ok())
}
