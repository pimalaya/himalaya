use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use log::debug;
use std::io;

pub fn subcmds<'s>() -> Vec<App<'s, 's>> {
    vec![SubCommand::with_name("completion")
        .about("Generates the completion script for the given shell")
        .args(&[Arg::with_name("shell")
            .possible_values(&["bash", "zsh", "fish"])
            .required(true)])]
}

pub fn matches<'a>(app: fn() -> App<'a, 'a>, matches: &ArgMatches) -> Result<bool> {
    if let Some(matches) = matches.subcommand_matches("completion") {
        debug!("completion command matched");
        let shell = match matches.value_of("shell").unwrap() {
            "fish" => Shell::Fish,
            "zsh" => Shell::Zsh,
            "bash" | _ => Shell::Bash,
        };
        debug!("shell: {}", shell);
        app().gen_completions_to("himalaya", shell, &mut io::stdout());
        return Ok(true);
    };

    debug!("nothing matched");
    Ok(false)
}
