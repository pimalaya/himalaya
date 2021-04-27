use clap;
use error_chain::error_chain;
use log::debug;

use crate::{app::App, imap::model::ImapConnector};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

pub fn imap_subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("idle").about("Spawns a blocking idle daemon")]
}

pub fn imap_matches(app: &App) -> Result<bool> {
    if let Some(_) = app.arg_matches.subcommand_matches("idle") {
        debug!("idle command matched");

        let mut imap_conn = ImapConnector::new(&app.account)?;
        imap_conn.idle(&app.config, &app.mbox)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
