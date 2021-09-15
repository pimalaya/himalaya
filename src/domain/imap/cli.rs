use anyhow::Result;
use clap;
use log::debug;

use crate::domain::{config::entity::Config, imap::service::ImapServiceInterface};

pub fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![
        clap::SubCommand::with_name("notify")
            .about("Notifies when new messages arrive in the given mailbox")
            .aliases(&["idle"])
            .arg(
                clap::Arg::with_name("keepalive")
                    .help("Specifies the keepalive duration")
                    .short("k")
                    .long("keepalive")
                    .value_name("SECS")
                    .default_value("500"),
            ),
        clap::SubCommand::with_name("watch")
            .about("Watches IMAP server changes")
            .arg(
                clap::Arg::with_name("keepalive")
                    .help("Specifies the keepalive duration")
                    .short("k")
                    .long("keepalive")
                    .value_name("SECS")
                    .default_value("500"),
            ),
    ]
}

pub fn matches<ImapService: ImapServiceInterface>(
    arg_matches: &clap::ArgMatches,
    config: &Config,
    imap: &mut ImapService,
) -> Result<bool> {
    if let Some(matches) = arg_matches.subcommand_matches("notify") {
        debug!("notify command matched");

        let keepalive = clap::value_t_or_exit!(matches.value_of("keepalive"), u64);
        debug!("keepalive: {}", &keepalive);

        imap.notify(&config, keepalive)?;
        imap.logout()?;
        return Ok(true);
    }

    if let Some(matches) = arg_matches.subcommand_matches("watch") {
        debug!("watch command matched");

        let keepalive = clap::value_t_or_exit!(matches.value_of("keepalive"), u64);
        debug!("keepalive: {}", &keepalive);

        imap.watch(keepalive)?;
        imap.logout()?;
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
