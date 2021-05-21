use clap;
use env_logger;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{env, path::PathBuf, process::exit};

use himalaya::{
    app::App,
    comp::cli::{comp_matches, comp_subcmds},
    config::{cli::config_args, model::Config},
    flag::cli::{flag_matches, flag_subcmds},
    himalaya_tui::cli::{himalaya_tui_matches, himalaya_tui_subcmds},
    imap::cli::{imap_matches, imap_subcmds},
    mbox::cli::{mbox_matches, mbox_source_arg, mbox_subcmds},
    msg::cli::{msg_matches, msg_subcmds},
    output::{cli::output_args, model::Output},
};

error_chain! {
    links {
        CompletionCli(himalaya::comp::cli::Error, himalaya::comp::cli::ErrorKind);
        Config(himalaya::config::model::Error, himalaya::config::model::ErrorKind);
        FlagCli(himalaya::flag::cli::Error, himalaya::flag::cli::ErrorKind);
        ImapCli(himalaya::imap::cli::Error, himalaya::imap::cli::ErrorKind);
        MboxCli(himalaya::mbox::cli::Error, himalaya::mbox::cli::ErrorKind);
        MsgCli(himalaya::msg::cli::Error, himalaya::msg::cli::ErrorKind);
    }
}

fn parse_args<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .args(&output_args())
        .args(&config_args())
        .arg(mbox_source_arg())

        // So in this part, we are "registing" all subcommands like the "tui"
        // command of "himalaya tui". Each function in between the brackets of
        // `subcommands()` includes the subcommands which suit teir category.
        .subcommands(flag_subcmds())
        .subcommands(imap_subcmds())
        .subcommands(mbox_subcmds())
        .subcommands(msg_subcmds())
        .subcommands(comp_subcmds())
        .subcommands(himalaya_tui_subcmds())
}

fn run() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    let args = parse_args();
    let arg_matches = args.get_matches();

    // Check completion before init config
    if comp_matches(parse_args, &arg_matches)? {
        return Ok(());
    }

    let output = Output::new(arg_matches.value_of("output").unwrap());
    debug!("Output: {:?}", output);

    // This part will read the config file and stores it values.
    debug!("## Init config ##");

    let custom_config: Option<PathBuf> = arg_matches.value_of("config").map(|s| s.into());
    debug!("Custom config path: {:?}", custom_config);

    let config = Config::new(custom_config)?;
    trace!("Config: {:?}", config);

    let account_name = arg_matches.value_of("account");
    debug!("Init account: {}", account_name.unwrap_or("default"));
    let account = config.find_account_by_name(account_name)?;
    trace!("Account: {:?}", account);

    let mbox = arg_matches.value_of("mailbox").unwrap();
    debug!("Mailbox: {}", mbox);

    debug!("Begin matching");
    let app = App::new(&config, &account, &output, &mbox, &arg_matches);
    let _matched =
        mbox_matches(&app)? || flag_matches(&app)? || imap_matches(&app)? || msg_matches(&app)? || himalaya_tui_matches(&app)?;

    Ok(())
}

fn main() {
    if let Err(ref errs) = run() {
        let mut errs = errs.iter();

        match errs.next() {
            None => (),
            Some(err) => {
                error!("{}", err);
                eprintln!("{}", err);

                errs.for_each(|err| {
                    error!("{}", err);
                    eprintln!(" ↳ {}", err);
                });
            }
        }

        exit(1);
    } else {
        exit(0);
    }
}
