use clap::{self, App, ArgMatches, SubCommand};
use error_chain::error_chain;

use crate::{config::Config, imap::model::ImapConnector};

error_chain! {
    links {
        Config(crate::config::Error, crate::config::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

pub fn imap_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("idle").about("Spawns a blocking idle daemon")]
}

pub fn imap_matches(matches: &ArgMatches) -> Result<bool> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(matches.value_of("account"))?;
    let mut imap_conn = ImapConnector::new(&account)?;
    let mbox = matches.value_of("mailbox").unwrap();

    if let Some(matches) = matches.subcommand_matches("idle") {
        imap_conn.idle(&config, &mbox)?;
        imap_conn.logout();
        return Ok(true);
    }

    Ok(false)
}
