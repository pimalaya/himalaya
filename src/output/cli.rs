use clap::Arg;

pub fn output_args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name("output")
            .long("output")
            .short("o")
            .help("Defines the output format")
            .value_name("FMT")
            .possible_values(&["plain", "json"])
            .default_value("plain"),
        Arg::with_name("log")
            .long("log")
            .short("l")
            .help("Defines the logs level")
            .value_name("LEVEL")
            .possible_values(&["error", "warn", "info", "debug", "trace"])
            .default_value("info"),
    ]
}
