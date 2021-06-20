use clap::Arg;

// ==============
// Functions
// ==============
/// Provides the following **options**:
/// - `-o, --output`
/// - `-l, --log-level`
pub fn options<'a>() -> Vec<Arg<'a, 'a>> {
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
    ]
}
