use clap;
use error_chain::error_chain;
use log::debug;

use crate::{ctx::Ctx, imap::model::ImapConnector};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

// ===================
// Main Functions
// ===================
/// Provides the following **subcommands**:
/// - `notify`
/// - `watch`
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

pub fn matches(ctx: &Ctx) -> Result<bool> {
    if let Some(matches) = ctx.arg_matches.subcommand_matches("notify") {
        debug!("notify command matched");

        let keepalive = clap::value_t_or_exit!(matches.value_of("keepalive"), u64);
        debug!("keepalive: {}", &keepalive);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.notify(&ctx, keepalive)?;

        imap_conn.logout();
        return Ok(true);
    }

    if let Some(matches) = ctx.arg_matches.subcommand_matches("watch") {
        debug!("watch command matched");

        let keepalive = clap::value_t_or_exit!(matches.value_of("keepalive"), u64);
        debug!("keepalive: {}", &keepalive);

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        imap_conn.watch(&ctx, keepalive)?;

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}
