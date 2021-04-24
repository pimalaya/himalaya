use clap::{self, App, ArgMatches, SubCommand};
use error_chain::error_chain;
use log::debug;

use crate::{
    config::model::{Account, Config},
    imap::model::ImapConnector,
};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

pub fn imap_subcmds<'s>() -> Vec<App<'s, 's>> {
    vec![SubCommand::with_name("idle").about("Spawns a blocking idle daemon")]
}

pub fn imap_matches(
    config: &Config,
    account: &Account,
    mbox: &str,
    matches: &ArgMatches,
) -> Result<bool> {
    if let Some(_) = matches.subcommand_matches("idle") {
        debug!("idle command matched");
        let mut imap_conn = ImapConnector::new(&account)?;
        imap_conn.idle(&config, &mbox)?;
        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
