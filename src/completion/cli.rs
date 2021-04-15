use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use error_chain::error_chain;
use std::io;

error_chain! {}

pub fn completion_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("completion")
        .about("Generates the completion script for the given shell")
        .args(&[Arg::with_name("shell")
            .possible_values(&["bash", "zsh", "fish"])
            .required(true)])]
}

pub fn completion_matches(mut app: App, matches: &ArgMatches) -> Result<bool> {
    if let Some(matches) = matches.subcommand_matches("completion") {
        let shell = match matches.value_of("shell").unwrap() {
            "fish" => Shell::Fish,
            "zsh" => Shell::Zsh,
            "bash" | _ => Shell::Bash,
        };
        app.gen_completions_to("himalaya", shell, &mut io::stdout());
        return Ok(true);
    };

    Ok(false)
}
