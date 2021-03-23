use clap::{self, Arg, SubCommand};
use error_chain::error_chain;
use std::env;

use crate::{
    flag::cli::{flag_matches, flag_subcmds},
    mbox::cli::{mbox_arg, mbox_matches, mbox_subcmds},
    msg::cli::{msg_matches, msg_subcmds},
};

error_chain! {
    links {
        MboxCli(crate::mbox::cli::Error, crate::mbox::cli::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
        FlagCli(crate::flag::cli::Error, crate::flag::cli::ErrorKind);
    }
}

pub struct App<'a>(pub clap::App<'a, 'a>);

impl<'a> App<'a> {
    pub fn new() -> Self {
        let app = clap::App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .arg(
                Arg::with_name("output")
                    .long("output")
                    .short("o")
                    .help("Defines the output format")
                    .value_name("STRING")
                    .possible_values(&["plain", "json"])
                    .default_value("plain"),
            )
            .arg(
                Arg::with_name("account")
                    .long("account")
                    .short("a")
                    .help("Selects a specific account")
                    .value_name("STRING"),
            )
            .arg(mbox_arg())
            .subcommand(SubCommand::with_name("idle").about("Spawns a blocking idle daemon"));

        let app = app.subcommands(mbox_subcmds());
        let app = app.subcommands(flag_subcmds());
        let app = app.subcommands(msg_subcmds());

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

            msg_matches(&matches)?;
            break;
        }

        // if let Some(matches) = matches.subcommand_matches("idle") {
        //     let config = Config::new_from_file()?;
        //     let account = config.find_account_by_name(account)?;
        //     let mut imap_conn = ImapConnector::new(&account)?;
        //     let mbox = matches.value_of("mailbox").unwrap();
        //     imap_conn.idle(&config, &mbox)?;
        // }

        Ok(())
    }
}
