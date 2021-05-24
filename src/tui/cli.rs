use clap;

use log::debug;

use crate::tui::model::run_tui;
use crate::config::model::Config;

/// Here are all subcommands related to the TUI.
pub fn tui_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![
        clap::SubCommand::with_name("tui")
            .about("Opens himalaya with the TUI"),
    ]
}

/// This function will look which subcommands (which belong to the TUI) has
/// been added in the commandline arguments and execute the appropriate code.
pub fn tui_matches<'func>(arg_matches: &clap::ArgMatches<'func>, config: &Config) -> Result<(), ()> {
    if let Some(_) = arg_matches.subcommand_matches("tui") {
        debug!("TUI subcommand matched => Opening TUI");

        debug!("The TUI is currently on road and will reach himalaya soon.");
        debug!("(In other words: It's still under development)");

        match run_tui(config) {
            Ok(_) => return Ok(()),
            Err(err) => {
                println!("{}", err);
                return Err(());
            }
        }
    }

    debug!("Nothing matched");
    Err(())
}
