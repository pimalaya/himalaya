use atty::Stream;
use clap;
use error_chain::error_chain;
use log::{debug, trace};
use mailparse;
use std::io::{self, BufRead};

use crate::{app::App, imap::model::ImapConnector, msg::tpl::model::Tpl};

error_chain! {
    links {
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
    }
    foreign_links {
        Clap(clap::Error);
        MailParse(mailparse::MailParseError);
    }
}

pub fn uid_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("uid")
        .help("Specifies the targetted message")
        .value_name("UID")
        .required(true)
}

fn reply_all_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("A")
        .long("all")
}

pub fn tpl_subcommand<'a>() -> clap::App<'a, 'a> {
    clap::SubCommand::with_name("template")
        .aliases(&["tpl"])
        .about("Generates a message template")
        .subcommand(
            clap::SubCommand::with_name("new")
                .aliases(&["n"])
                .about("Generates a new message template")
                .args(&tpl_args()),
        )
        .subcommand(
            clap::SubCommand::with_name("reply")
                .aliases(&["rep", "r"])
                .about("Generates a reply message template")
                .arg(uid_arg())
                .arg(reply_all_arg())
                .args(&tpl_args()),
        )
        .subcommand(
            clap::SubCommand::with_name("forward")
                .aliases(&["fwd", "fw", "f"])
                .about("Generates a forward message template")
                .arg(uid_arg())
                .args(&tpl_args()),
        )
}

pub fn tpl_args<'a>() -> Vec<clap::Arg<'a, 'a>> {
    vec![
        clap::Arg::with_name("subject")
            .help("Overrides the Subject header")
            .short("s")
            .long("subject")
            .value_name("STRING"),
        clap::Arg::with_name("from")
            .help("Overrides the From header")
            .short("f")
            .long("from")
            .value_name("ADDR"),
        clap::Arg::with_name("to")
            .help("Overrides the To header")
            .short("t")
            .long("to")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("cc")
            .help("Overrides the Cc header")
            .short("c")
            .long("cc")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("bcc")
            .help("Overrides the Bcc header")
            .short("b")
            .long("bcc")
            .value_name("ADDR")
            .multiple(true),
        clap::Arg::with_name("header")
            .help("Overrides a specific header")
            .short("h")
            .long("header")
            .value_name("KEY: VAL")
            .multiple(true),
        clap::Arg::with_name("body")
            .help("Overrides the body")
            .short("B")
            .long("body")
            .value_name("STRING"),
        clap::Arg::with_name("signature")
            .help("Overrides the signature")
            .short("S")
            .long("signature")
            .value_name("STRING"),
    ]
}

pub fn tpl_matches(app: &App, matches: &clap::ArgMatches) -> Result<bool> {
    match matches.subcommand() {
        ("new", Some(matches)) => tpl_matches_new(app, matches),
        ("reply", Some(matches)) => tpl_matches_reply(app, matches),
        ("forward", Some(matches)) => tpl_matches_forward(app, matches),

        // TODO: find a way to show the help message for template subcommand
        _ => Err("Subcommand not found".into()),
    }
}

fn tpl_matches_new(app: &App, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("new command matched");
    let mut tpl = Tpl::new(&app);

    if let Some(from) = matches.value_of("from") {
        debug!("overriden from: {:?}", from);
        tpl.header("From", from);
    };

    if let Some(subject) = matches.value_of("subject") {
        debug!("overriden subject: {:?}", subject);
        tpl.header("Subject", subject);
    };

    let addrs = matches.values_of("to").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden to: {:?}", addrs);
        tpl.header("To", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("cc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden cc: {:?}", addrs);
        tpl.header("Cc", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("bcc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden bcc: {:?}", addrs);
        tpl.header("Bcc", addrs.collect::<Vec<_>>().join(", "));
    }

    for header in matches.values_of("header").unwrap_or_default() {
        let mut header = header.split(":");
        let key = header.next().unwrap_or_default();
        let val = header.next().unwrap_or_default().trim_start();
        debug!("overriden header: {}={}", key, val);
        tpl.header(key, val);
    }

    if atty::isnt(Stream::Stdin) {
        let body = io::stdin()
            .lock()
            .lines()
            .filter_map(|ln| ln.ok())
            .map(|ln| ln.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        debug!("overriden body from stdin: {:?}", body);
        tpl.body(body);
    } else if let Some(body) = matches.value_of("body") {
        debug!("overriden body: {:?}", body);
        tpl.body(body);
    };

    if let Some(signature) = matches.value_of("signature") {
        debug!("overriden signature: {:?}", signature);
        tpl.signature(signature);
    };

    trace!("tpl: {:?}", tpl);
    app.output.print(tpl);

    Ok(true)
}

fn tpl_matches_reply(app: &App, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("reply command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);

    let mut imap_conn = ImapConnector::new(&app.account)?;
    let msg = &imap_conn.read_msg(&app.mbox, &uid)?;
    let msg = mailparse::parse_mail(&msg)?;
    let mut tpl = if matches.is_present("reply-all") {
        Tpl::reply(&app, &msg)
    } else {
        Tpl::reply_all(&app, &msg)
    };
    if let Some(from) = matches.value_of("from") {
        debug!("overriden from: {:?}", from);
        tpl.header("From", from);
    };

    if let Some(subject) = matches.value_of("subject") {
        debug!("overriden subject: {:?}", subject);
        tpl.header("Subject", subject);
    };

    let addrs = matches.values_of("to").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden to: {:?}", addrs);
        tpl.header("To", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("cc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden cc: {:?}", addrs);
        tpl.header("Cc", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("bcc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden bcc: {:?}", addrs);
        tpl.header("Bcc", addrs.collect::<Vec<_>>().join(", "));
    }

    for header in matches.values_of("header").unwrap_or_default() {
        let mut header = header.split(":");
        let key = header.next().unwrap_or_default();
        let val = header.next().unwrap_or_default().trim_start();
        debug!("overriden header: {}={}", key, val);
        tpl.header(key, val);
    }

    if let Some(body) = matches.value_of("body") {
        debug!("overriden body: {:?}", body);
        tpl.body(body);
    };

    if let Some(signature) = matches.value_of("signature") {
        debug!("overriden signature: {:?}", signature);
        tpl.signature(signature);
    };

    trace!("tpl: {:?}", tpl);
    app.output.print(tpl);

    Ok(true)
}

fn tpl_matches_forward(app: &App, matches: &clap::ArgMatches) -> Result<bool> {
    debug!("forward command matched");

    let uid = matches.value_of("uid").unwrap();
    debug!("uid: {}", uid);

    let mut imap_conn = ImapConnector::new(&app.account)?;
    let msg = &imap_conn.read_msg(&app.mbox, &uid)?;
    let msg = mailparse::parse_mail(&msg)?;
    let mut tpl = Tpl::forward(&app, &msg);

    if let Some(from) = matches.value_of("from") {
        debug!("overriden from: {:?}", from);
        tpl.header("From", from);
    };

    if let Some(subject) = matches.value_of("subject") {
        debug!("overriden subject: {:?}", subject);
        tpl.header("Subject", subject);
    };

    let addrs = matches.values_of("to").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden to: {:?}", addrs);
        tpl.header("To", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("cc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden cc: {:?}", addrs);
        tpl.header("Cc", addrs.collect::<Vec<_>>().join(", "));
    }

    let addrs = matches.values_of("bcc").unwrap_or_default();
    if addrs.len() > 0 {
        debug!("overriden bcc: {:?}", addrs);
        tpl.header("Bcc", addrs.collect::<Vec<_>>().join(", "));
    }

    for header in matches.values_of("header").unwrap_or_default() {
        let mut header = header.split(":");
        let key = header.next().unwrap_or_default();
        let val = header.next().unwrap_or_default().trim_start();
        debug!("overriden header: {}={}", key, val);
        tpl.header(key, val);
    }

    if let Some(body) = matches.value_of("body") {
        debug!("overriden body: {:?}", body);
        tpl.body(body);
    };

    if let Some(signature) = matches.value_of("signature") {
        debug!("overriden signature: {:?}", signature);
        tpl.signature(signature);
    };

    trace!("tpl: {:?}", tpl);
    app.output.print(tpl);

    Ok(true)
}
