use clap::{self, App, Arg, ArgMatches, SubCommand};
use error_chain::error_chain;
use std::fs;

use crate::{
    config::model::Config,
    imap::model::ImapConnector,
    input,
    msg::model::{Attachments, Msg, Msgs, ReadableMsg},
    output::utils::print,
    smtp,
};

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        Imap(crate::imap::model::Error, crate::imap::model::ErrorKind);
        Input(crate::input::Error, crate::input::ErrorKind);
        MsgModel(crate::msg::model::Error, crate::msg::model::ErrorKind);
        OutputUtils(crate::output::utils::Error, crate::output::utils::ErrorKind);
        Smtp(crate::smtp::Error, crate::smtp::ErrorKind);
    }
}

fn uid_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("uid")
        .help("Message UID")
        .value_name("UID")
        .required(true)
}

fn reply_all_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("a")
        .long("all")
}

fn page_size_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("size")
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
        .default_value("10")
}

fn page_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("page")
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value("0")
}

pub fn msg_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        SubCommand::with_name("messages")
            .aliases(&["message", "msgs", "msg", "m"])
            .about("Lists all messages")
            .arg(page_size_arg())
            .arg(page_arg()),
        SubCommand::with_name("search")
            .aliases(&["query", "q", "s"])
            .about("Lists messages matching the given IMAP query")
            .arg(page_size_arg())
            .arg(page_arg())
            .arg(
                Arg::with_name("query")
                    .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                    .value_name("QUERY")
                    .multiple(true)
                    .required(true),
            ),
        SubCommand::with_name("write").about("Writes a new message"),
        SubCommand::with_name("send")
            .about("Sends a raw message")
            .arg(Arg::with_name("message").raw(true)),
        SubCommand::with_name("save")
            .about("Saves a raw message")
            .arg(Arg::with_name("message").raw(true)),
        SubCommand::with_name("read")
            .aliases(&["r"])
            .about("Reads text bodies of a message")
            .arg(uid_arg())
            .arg(
                Arg::with_name("mime-type")
                    .help("MIME type to use")
                    .short("t")
                    .long("mime-type")
                    .value_name("STRING")
                    .possible_values(&["plain", "html"])
                    .default_value("plain"),
            ),
        SubCommand::with_name("attachments")
            .aliases(&["attach", "att", "a"])
            .about("Downloads all attachments from an email")
            .arg(uid_arg()),
        SubCommand::with_name("reply")
            .aliases(&["rep", "re"])
            .about("Answers to an email")
            .arg(uid_arg())
            .arg(reply_all_arg()),
        SubCommand::with_name("forward")
            .aliases(&["fwd", "f"])
            .about("Forwards an email")
            .arg(uid_arg()),
        SubCommand::with_name("template")
            .aliases(&["tpl", "t"])
            .about("Generates a message template")
            .subcommand(
                SubCommand::with_name("new")
                    .aliases(&["n"])
                    .about("Generates a new message template"),
            )
            .subcommand(
                SubCommand::with_name("reply")
                    .aliases(&["rep", "r"])
                    .about("Generates a reply message template")
                    .arg(uid_arg())
                    .arg(reply_all_arg()),
            )
            .subcommand(
                SubCommand::with_name("forward")
                    .aliases(&["fwd", "fw", "f"])
                    .about("Generates a forward message template")
                    .arg(uid_arg()),
            ),
    ]
}

pub fn msg_matches(matches: &ArgMatches) -> Result<()> {
    let config = Config::new_from_file()?;
    let account = config.find_account_by_name(matches.value_of("account"))?;
    let output_fmt = matches.value_of("output").unwrap();
    let mbox = matches.value_of("mailbox").unwrap();
    let mut imap_conn = ImapConnector::new(&account)?;

    loop {
        if let Some(matches) = matches.subcommand_matches("messages") {
            let page_size: usize = matches.value_of("size").unwrap().parse().unwrap();
            let page: usize = matches.value_of("page").unwrap().parse().unwrap();

            let msgs = imap_conn.list_msgs(&mbox, &page_size, &page)?;
            let msgs = Msgs::from(&msgs);

            print(&output_fmt, msgs)?;
            break;
        }

        if let Some(matches) = matches.subcommand_matches("search") {
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

            print(&output_fmt, msgs)?;
            break;
        }

        if let Some(matches) = matches.subcommand_matches("read") {
            let uid = matches.value_of("uid").unwrap();
            let mime = format!("text/{}", matches.value_of("mime-type").unwrap());

            let msg = imap_conn.read_msg(&mbox, &uid)?;
            let msg = ReadableMsg::from_bytes(&mime, &msg)?;

            print(&output_fmt, msg)?;
            break;
        }

        if let Some(matches) = matches.subcommand_matches("attachments") {
            let uid = matches.value_of("uid").unwrap();

            let msg = imap_conn.read_msg(&mbox, &uid)?;
            let attachments = Attachments::from_bytes(&msg)?;

            match output_fmt {
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

            break;
        }

        if let Some(_) = matches.subcommand_matches("write") {
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

            break;
        }

        if let Some(matches) = matches.subcommand_matches("template") {
            if let Some(_) = matches.subcommand_matches("new") {
                let tpl = Msg::build_new_tpl(&config, &account)?;
                print(&output_fmt, &tpl)?;
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

                print(&output_fmt, &tpl)?;
            }

            if let Some(matches) = matches.subcommand_matches("forward") {
                let uid = matches.value_of("uid").unwrap();
                let mbox = matches.value_of("mailbox").unwrap();

                let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
                let tpl = msg.build_forward_tpl(&config, &account)?;

                print(&output_fmt, &tpl)?;
            }

            break;
        }

        if let Some(matches) = matches.subcommand_matches("reply") {
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

            break;
        }

        if let Some(matches) = matches.subcommand_matches("forward") {
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

            break;
        }

        if let Some(matches) = matches.subcommand_matches("send") {
            let msg = matches.value_of("message").unwrap();
            let msg = Msg::from(msg.to_string());
            let msg = msg.to_sendable_msg()?;

            smtp::send(&account, &msg)?;
            imap_conn.append_msg("Sent", &msg.formatted())?;
            break;
        }

        if let Some(matches) = matches.subcommand_matches("save") {
            let msg = matches.value_of("message").unwrap();
            let msg = Msg::from(msg.to_string());

            imap_conn.append_msg(mbox, &msg.to_vec()?)?;
            break;
        }

        // Default case: list all messages

        let msgs = imap_conn.list_msgs(&mbox, &10, &0)?;
        let msgs = Msgs::from(&msgs);

        print(&output_fmt, msgs)?;
        break;
    }

    imap_conn.logout();
    Ok(())
}
