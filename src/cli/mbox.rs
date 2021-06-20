use clap;
use error_chain::error_chain;
use log::{debug, trace};

use crate::{ctx::Ctx, imap::model::ImapConnector, mbox::model::Mboxes};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
}

// ===================
// Main Functions
// ===================
pub fn options<'options>() -> Vec<clap::Arg<'options, 'options>> {
    vec![
        clap::Arg::with_name("mailbox")
            .short("m")
            .long("mailbox")
            .help("Selects a specific mailbox")
            .value_name("MAILBOX")
            .default_value("INBOX"),
    ]
}

pub fn subcmds<'subcmds>() -> Vec<clap::App<'subcmds, 'subcmds>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "m"])
        .about("Lists all mailboxes")]
}

pub fn matches(ctx: &Ctx) -> Result<bool> {
    if let Some(_) = ctx.arg_matches.subcommand_matches("mailboxes") {
        debug!("mailboxes command matched");

        let mut imap_conn = ImapConnector::new(&ctx.account)?;
        let names = imap_conn.list_mboxes()?;
        let mboxes = Mboxes::from(&names);
        debug!("found {} mailboxes", mboxes.0.len());
        trace!("mailboxes: {:?}", mboxes);
        ctx.output.print(mboxes);

        imap_conn.logout();
        return Ok(true);
    }

    debug!("nothing matched");
    Ok(false)
}

// ==================
// Arg functions
// ==================
pub fn target_arg<'target>() -> clap::Arg<'target, 'target> {
    clap::Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
}
