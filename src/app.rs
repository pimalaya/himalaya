use clap::{self, Arg};
use error_chain::error_chain;
use std::env;

use crate::{
    flag::cli::{flag_matches, flag_subcmds},
    imap::cli::{imap_matches, imap_subcmds},
    mbox::cli::{mbox_arg, mbox_matches, mbox_subcmds},
    msg::cli::{msg_matches, msg_subcmds},
    output::cli::output_args,
};

error_chain! {
    links {
        FlagCli(crate::flag::cli::Error, crate::flag::cli::ErrorKind);
        ImapCli(crate::imap::cli::Error, crate::imap::cli::ErrorKind);
        MboxCli(crate::mbox::cli::Error, crate::mbox::cli::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
    }
}

pub struct App<'a>(pub clap::App<'a, 'a>);

impl<'a> App<'a> {
    pub fn new() -> Self {
        let app = clap::App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .args(&output_args())
            .arg(
                Arg::with_name("account")
                    .long("account")
                    .short("a")
                    .help("Selects a specific account")
                    .value_name("STRING"),
            )
            .arg(mbox_arg())
            .subcommands(flag_subcmds())
            .subcommands(imap_subcmds())
            .subcommands(mbox_subcmds())
            .subcommands(msg_subcmds());

        Self(app)
    }

    pub fn run(self) -> Result<()> {
        let matches = self.0.get_matches();

        loop {
            if mbox_matches(&matches)? {
                break;
            }

            if flag_matches(&matches)? {
                break;
            }

            if imap_matches(&matches)? {
                break;
            }

            msg_matches(&matches)?;
            break;
        }

        Ok(())
    }
}
