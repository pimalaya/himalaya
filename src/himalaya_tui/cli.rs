use clap;
use log::debug;
use crate::app::App;

/// Here are all subcommands related to the TUI.
pub fn himalaya_tui_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![
        clap::SubCommand::with_name("tui")
            .about("Opens himalaya with the TUI"),
    ]
}

/// This function will look which subcommands (which belong to the TUI) has
/// been added in the commandline arguments and execute the appropriate code.
pub fn himalaya_tui_matches(app: &App) -> Result<bool, String> {
    if let Some(matches) = app.arg_matches.subcommand_matches("tui") {
        debug!("TUI subcommand matched => Opening TUI");

        return Ok(true);
    }

    println!("The TUI is currently on road and will reach himalaya soon.");
    println!("(In other words: It's still under development)");

    debug!("Nothing matched");
    Ok(false)
}
