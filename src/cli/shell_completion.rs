use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use log::debug;
use std::io;

// ==============
// Functions
// ==============
/// Provided the following subcommands: `completion shell <Shell>`
pub fn subcmds<'comp>() -> Vec<App<'comp, 'comp>> {
    vec![SubCommand::with_name("completion")
        .about("Generates the completion script for the given shell")
        .args(
            &[
            Arg::with_name("shell")
            .possible_values(&["bash", "zsh", "fish"])
            .required(true)
            ]
            )
    ]
}

pub fn matches<'comp>(
    app: fn() -> clap::App<'comp, 'comp>,
    matches: &ArgMatches
    ) -> bool
{
    if let Some(matches) = matches.subcommand_matches("completion") {
        debug!("Shell-Completion command matched!");

        let shell = match matches.value_of("shell").unwrap() {
            "fish" => Shell::Fish,
            "zsh" => Shell::Zsh,
            "bash" | _ => Shell::Bash,
        };

        debug!("Selected shell: {}", shell);

        // generate the autocompletion-file for the given shell and print it out
        app().gen_completions_to("himalaya", shell, &mut io::stdout());
        return true;
    };

    debug!("Nothing matched");
    false
}
