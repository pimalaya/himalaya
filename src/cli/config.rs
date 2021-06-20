use clap::Arg;

// ===================
// Main-Functions
// ===================
/// Provides the following options:
/// - `-c, --config`
/// - `-a, --account`
pub fn options<'option>() -> Vec<Arg<'option, 'option>> {
    vec![
        Arg::with_name("config")
            .long("config")
            .short("c")
            .help("Forces a specific config path")
            .value_name("PATH"),
        Arg::with_name("account")
            .long("account")
            .short("a")
            .help("Selects a specific account")
            .value_name("NAME"),
    ]
}
