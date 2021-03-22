use clap::{self, Arg, SubCommand};
use error_chain::error_chain;
use std::{env, fs};

use crate::{
    config::{self, Config},
    flag::cli::{flags_matches, flags_subcommand},
    imap::{self, ImapConnector},
    input,
    msg::{self, Attachments, Msg, Msgs, ReadableMsg},
    output::{self, print},
    smtp,
};

error_chain! {
    links {
        Config(config::Error, config::ErrorKind);
        Imap(imap::Error, imap::ErrorKind);
        Input(input::Error, input::ErrorKind);
        Message(msg::Error, msg::ErrorKind);
        Output(output::Error, output::ErrorKind);
        Smtp(smtp::Error, smtp::ErrorKind);
    }
}

pub struct App<'a>(pub clap::App<'a, 'a>);

impl<'a> App<'a> {
    fn mailbox_arg() -> Arg<'a, 'a> {
        Arg::with_name("mailbox")
            .short("m")
            .long("mailbox")
            .help("Mailbox name")
            .value_name("STRING")
            .default_value("INBOX")
    }

    fn uid_arg() -> Arg<'a, 'a> {
        Arg::with_name("uid")
            .help("Message UID")
            .value_name("UID")
            .required(true)
    }

    fn reply_all_arg() -> Arg<'a, 'a> {
        Arg::with_name("reply-all")
            .help("Includes all recipients")
            .short("a")
            .long("all")
    }

    fn page_size_arg() -> Arg<'a, 'a> {
        Arg::with_name("size")
            .help("Page size")
            .short("s")
            .long("size")
            .value_name("INT")
            .default_value("10")
    }

    fn page_arg() -> Arg<'a, 'a> {
        Arg::with_name("page")
            .help("Page number")
            .short("p")
            .long("page")
            .value_name("INT")
            .default_value("0")
    }

    pub fn new() -> Self {
        Self(clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .help("Formats the output")
                .value_name("STRING")
                .possible_values(&["text", "json"])
                .default_value("text"),
        )
        .arg(
            Arg::with_name("account")
                .long("account")
                .short("a")
                .help("Selects a specific account")
                .value_name("STRING"),
        )
        .arg(
            Arg::with_name("mailbox")
                .short("m")
                .long("mailbox")
                .help("Selects a specific mailbox")
                .value_name("STRING")
                .default_value("INBOX")
        )
        .subcommand(
            SubCommand::with_name("mailboxes")
                .aliases(&["mailbox", "mboxes", "mbox", "mb", "m"])
                .about("Lists all available mailboxes"),
        )
        .subcommand(flags_subcommand())
        .subcommand(
            SubCommand::with_name("list")
                .aliases(&["lst", "l"])
                .about("Lists emails sorted by arrival date")
                .arg(Self::mailbox_arg())
                .arg(Self::page_size_arg())
                .arg(Self::page_arg()),
        )
        .subcommand(
            SubCommand::with_name("search")
                .aliases(&["query", "q", "s"])
                .about("Lists emails matching the given IMAP query")
                .arg(Self::mailbox_arg())
                .arg(Self::page_size_arg())
                .arg(Self::page_arg())
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                        .value_name("QUERY")
                        .multiple(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("read")
                .aliases(&["r"])
                .about("Reads text bodies of an email")
                .arg(Self::uid_arg())
                .arg(Self::mailbox_arg())
                .arg(
                    Arg::with_name("mime-type")
                        .help("MIME type to use")
                        .short("t")
                        .long("mime-type")
                        .value_name("STRING")
                        .possible_values(&["plain", "html"])
                        .default_value("plain"),
                ),
        )
        .subcommand(
            SubCommand::with_name("attachments")
                .aliases(&["attach", "att", "a"])
                .about("Downloads all attachments from an email")
                .arg(Self::uid_arg())
                .arg(Self::mailbox_arg()),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a new email"))
        .subcommand(
            SubCommand::with_name("reply")
                .aliases(&["rep", "re"])
                .about("Answers to an email")
                .arg(Self::uid_arg())
                .arg(Self::mailbox_arg())
                .arg(Self::reply_all_arg()),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "f"])
                .about("Forwards an email")
                .arg(Self::uid_arg())
                .arg(Self::mailbox_arg()),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("Sends a raw message")
                .arg(Arg::with_name("message").raw(true)),
        )
        .subcommand(
            SubCommand::with_name("save")
                .about("Saves a raw message in the given mailbox")
                .arg(Self::mailbox_arg())
                .arg(Arg::with_name("message").raw(true)),
        )
        .subcommand(
            SubCommand::with_name("template")
                .aliases(&["tpl", "t"])
                .about("Generates a message template")
                .subcommand(
                    SubCommand::with_name("new")
                        .aliases(&["n"])
                        .about("Generates a new message template")
                        .arg(Self::mailbox_arg()),
                )
                .subcommand(
                    SubCommand::with_name("reply")
                        .aliases(&["rep", "r"])
                        .about("Generates a reply message template")
                        .arg(Self::uid_arg())
                        .arg(Self::mailbox_arg())
                        .arg(Self::reply_all_arg()),
                )
                .subcommand(
                    SubCommand::with_name("forward")
                        .aliases(&["fwd", "fw", "f"])
                        .about("Generates a forward message template")
                        .arg(Self::uid_arg())
                        .arg(Self::mailbox_arg()),
                ),
        )
        .subcommand(
            SubCommand::with_name("idle")
                .about("Starts the idle mode")
                .arg(Self::mailbox_arg()),
        ))
    }

    pub fn run(self) -> Result<()> {
        let matches = self.0.get_matches();

        let output_type = matches.value_of("output").unwrap().to_owned();
        let account = matches.value_of("account");
        let mbox = matches.value_of("mailbox").unwrap();

        if let Some(_) = matches.subcommand_matches("mailboxes") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mboxes = imap_conn.list_mboxes()?;
            print(&output_type, mboxes)?;

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("flags") {
            flags_matches(account, &mbox, &matches)
                .chain_err(|| "Could not handle flags arg matches")?;
        }

        if let Some(matches) = matches.subcommand_matches("list") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let page_size: u32 = matches.value_of("size").unwrap().parse().unwrap();
            let page: u32 = matches.value_of("page").unwrap().parse().unwrap();

            let msgs = imap_conn.list_msgs(&mbox, &page_size, &page)?;
            let msgs = Msgs::from(&msgs);

            print(&output_type, msgs)?;

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("search") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;
            let mbox = matches.value_of("mailbox").unwrap();
            let page_size: usize = matches.value_of("size").unwrap().parse().unwrap();
            let page: usize = matches.value_of("page").unwrap().parse().unwrap();
            let query = matches
                .values_of("query")
                .unwrap_or_default()
                .fold((false, vec![]), |(escape, mut cmds), cmd| {
                    match (cmd, escape) {
                        // Next command is an arg and needs to be escaped
                        ("subject", _) | ("body", _) | ("text", _) => {
                            cmds.push(cmd.to_string());
                            (true, cmds)
                        }
                        // Escaped arg commands
                        (_, true) => {
                            cmds.push(format!("\"{}\"", cmd));
                            (false, cmds)
                        }
                        // Regular commands
                        (_, false) => {
                            cmds.push(cmd.to_string());
                            (false, cmds)
                        }
                    }
                })
                .1
                .join(" ");

            let msgs = imap_conn.search_msgs(&mbox, &query, &page_size, &page)?;
            let msgs = Msgs::from(&msgs);

            print(&output_type, msgs)?;

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("read") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let uid = matches.value_of("uid").unwrap();
            let mime = format!("text/{}", matches.value_of("mime-type").unwrap());

            let msg = imap_conn.read_msg(&mbox, &uid)?;
            let msg = ReadableMsg::from_bytes(&mime, &msg)?;

            print(&output_type, msg)?;
            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("attachments") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let uid = matches.value_of("uid").unwrap();

            let msg = imap_conn.read_msg(&mbox, &uid)?;
            let attachments = Attachments::from_bytes(&msg)?;

            match output_type.as_str() {
                "text" => {
                    println!(
                        "{} attachment(s) found for message {}",
                        attachments.0.len(),
                        uid
                    );

                    attachments.0.iter().for_each(|attachment| {
                        let filepath = config.downloads_filepath(&account, &attachment.filename);
                        println!("Downloading {}…", &attachment.filename);
                        fs::write(filepath, &attachment.raw).unwrap()
                    });

                    println!("Done!");
                }
                "json" => {
                    attachments.0.iter().for_each(|attachment| {
                        let filepath = config.downloads_filepath(&account, &attachment.filename);
                        fs::write(filepath, &attachment.raw).unwrap()
                    });

                    print!("{{}}");
                }
                _ => (),
            }

            imap_conn.logout();
        }

        if let Some(_) = matches.subcommand_matches("write") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;
            let tpl = Msg::build_new_tpl(&config, &account)?;
            let content = input::open_editor_with_tpl(tpl.to_string().as_bytes())?;
            let mut msg = Msg::from(content);

            loop {
                match input::post_edit_choice() {
                    Ok(choice) => match choice {
                        input::Choice::Send => {
                            println!("Sending…");
                            let msg = msg.to_sendable_msg()?;
                            smtp::send(&account, &msg)?;
                            imap_conn.append_msg("Sent", &msg.formatted())?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Draft => {
                            println!("Saving to draft…");
                            imap_conn.append_msg("Drafts", &msg.to_vec()?)?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Edit => {
                            let content = input::open_editor_with_draft()?;
                            msg = Msg::from(content);
                        }
                        input::Choice::Quit => break,
                    },
                    Err(err) => eprintln!("{}", err),
                }
            }

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("template") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            if let Some(_) = matches.subcommand_matches("new") {
                let tpl = Msg::build_new_tpl(&config, &account)?;
                print(&output_type, &tpl)?;
            }

            if let Some(matches) = matches.subcommand_matches("reply") {
                let uid = matches.value_of("uid").unwrap();
                let mbox = matches.value_of("mailbox").unwrap();

                let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
                let tpl = if matches.is_present("reply-all") {
                    msg.build_reply_all_tpl(&config, &account)?
                } else {
                    msg.build_reply_tpl(&config, &account)?
                };

                print(&output_type, &tpl)?;
            }

            if let Some(matches) = matches.subcommand_matches("forward") {
                let uid = matches.value_of("uid").unwrap();
                let mbox = matches.value_of("mailbox").unwrap();

                let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
                let tpl = msg.build_forward_tpl(&config, &account)?;

                print(&output_type, &tpl)?;
            }
        }

        if let Some(matches) = matches.subcommand_matches("reply") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let uid = matches.value_of("uid").unwrap();

            let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
            let tpl = if matches.is_present("reply-all") {
                msg.build_reply_all_tpl(&config, &account)?
            } else {
                msg.build_reply_tpl(&config, &account)?
            };

            let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
            let mut msg = Msg::from(content);

            loop {
                match input::post_edit_choice() {
                    Ok(choice) => match choice {
                        input::Choice::Send => {
                            println!("Sending…");
                            smtp::send(&account, &msg.to_sendable_msg()?)?;
                            imap_conn.append_msg("Sent", &msg.to_vec()?)?;
                            imap_conn.add_flags(mbox, uid, "\\Answered")?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Draft => {
                            println!("Saving to draft…");
                            imap_conn.append_msg("Drafts", &msg.to_vec()?)?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Edit => {
                            let content = input::open_editor_with_draft()?;
                            msg = Msg::from(content);
                        }
                        input::Choice::Quit => break,
                    },
                    Err(err) => eprintln!("{}", err),
                }
            }

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("forward") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let uid = matches.value_of("uid").unwrap();

            let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
            let tpl = msg.build_forward_tpl(&config, &account)?;
            let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
            let mut msg = Msg::from(content);

            loop {
                match input::post_edit_choice() {
                    Ok(choice) => match choice {
                        input::Choice::Send => {
                            println!("Sending…");
                            smtp::send(&account, &msg.to_sendable_msg()?)?;
                            imap_conn.append_msg("Sent", &msg.to_vec()?)?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Draft => {
                            println!("Saving to draft…");
                            imap_conn.append_msg("Drafts", &msg.to_vec()?)?;
                            println!("Done!");
                            break;
                        }
                        input::Choice::Edit => {
                            let content = input::open_editor_with_draft()?;
                            msg = Msg::from(content);
                        }
                        input::Choice::Quit => break,
                    },
                    Err(err) => eprintln!("{}", err),
                }
            }

            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("send") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let msg = matches.value_of("message").unwrap();
            let msg = Msg::from(msg.to_string());
            let msg = msg.to_sendable_msg()?;

            smtp::send(&account, &msg)?;
            imap_conn.append_msg("Sent", &msg.formatted())?;
            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("save") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;

            let mbox = matches.value_of("mailbox").unwrap();
            let msg = matches.value_of("message").unwrap();
            let msg = Msg::from(msg.to_string());

            imap_conn.append_msg(mbox, &msg.to_vec()?)?;
            imap_conn.logout();
        }

        if let Some(matches) = matches.subcommand_matches("idle") {
            let config = Config::new_from_file()?;
            let account = config.find_account_by_name(account)?;
            let mut imap_conn = ImapConnector::new(&account)?;
            let mbox = matches.value_of("mailbox").unwrap();
            imap_conn.idle(&config, &mbox)?;
        }

        Ok(())
    }
}
