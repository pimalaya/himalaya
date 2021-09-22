use std::convert::{TryFrom, TryInto};

use anyhow::Result;
use log::{debug, trace};

use crate::{
    config::entity::Account,
    domain::{
        imap::ImapServiceInterface,
        msg::{self, entity::Msg, Envelope, Tpl, TplOverride},
    },
    output::service::OutputServiceInterface,
};

pub fn new<'a, OutputService: OutputServiceInterface>(
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
) -> Result<()> {
    let tpl = Tpl::new(&opts, account);
    trace!("template: {:#?}", tpl);
    output.print(tpl)?;
    Ok(())
}

pub fn reply<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    seq: &str,
    all: bool,
    opts: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    debug!("entering reply handler");
    let msg = imap.find_msg(seq)?;
    trace!("message: {:#?}", msg);
    let tpl = Tpl::reply(all, &msg, &opts, account);
    trace!("template: {:#?}", tpl);
    output.print(tpl)?;
    Ok(())
}

pub fn forward<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    tpl: TplOverride<'a>,
    account: &'a Account,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let mut msg = imap.get_msg(&uid)?.into_forward(&account)?;
    override_msg_with_args(&mut msg, tpl);
    trace!("Message: {:?}", msg);
    // output.print(MsgSerialized::try_from(&msg)?)?;
    Ok(())
}

// == Helper functions ==
// -- Template Subcommands --
// These functions are more used for the "template" subcommand
fn override_msg_with_args<'a>(msg: &mut Msg, tpl: TplOverride<'a>) {
    // -- Collecting credentials --
    // let from: Vec<String> = match tpl.from {
    //     Some(from) => from.map(|arg| arg.to_string()).collect(),
    //     None => msg.headers.from.clone(),
    // };
    // let to: Vec<String> = match tpl.to {
    //     Some(to) => to.map(|arg| arg.to_string()).collect(),
    //     None => Vec::new(),
    // };
    // let subject = tpl
    //     .subject
    //     .map(String::from)
    //     .or_else(|| msg.headers.subject.clone())
    //     .or_else(|| Some(String::new()));
    // let cc: Option<Vec<String>> = tpl
    //     .cc
    //     .map(|cc| cc.map(|arg| arg.to_string()).collect())
    //     .or_else(|| msg.headers.cc.clone());
    // let bcc: Option<Vec<String>> = tpl
    //     .bcc
    //     .map(|bcc| bcc.map(|arg| arg.to_string()).collect())
    //     .or_else(|| msg.headers.bcc.clone());

    // let custom_headers: Option<HashMap<String, Vec<String>>> = {
    //     if let Some(matched_headers) = tpl.headers {
    //         let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();

    //         // collect the custom headers
    //         for header in matched_headers {
    //             let mut header = header.split(":");
    //             let key = header.next().unwrap_or_default();
    //             let val = header.next().unwrap_or_default().trim_start();

    //             custom_headers.insert(key.to_string(), vec![val.to_string()]);
    //         }

    //         Some(custom_headers)
    //     } else {
    //         None
    //     }
    // };

    // let body = {
    //     if atty::isnt(Stream::Stdin) {
    //         let body = io::stdin()
    //             .lock()
    //             .lines()
    //             .filter_map(|line| line.ok())
    //             .map(|line| line.to_string())
    //             .collect::<Vec<String>>()
    //             .join("\n");
    //         debug!("overriden body from stdin: {:?}", body);
    //         body
    //     } else if let Some(body) = tpl.body {
    //         debug!("overriden body: {:?}", body);
    //         body.to_string()
    //     } else {
    //         msg.body
    //             .plain
    //             .as_ref()
    //             .map(String::from)
    //             .unwrap_or_default()
    //     }
    // };

    // let body = Body::new_with_text(body);

    // -- Creating and printing --
    // let headers = Headers {
    //     from,
    //     subject,
    //     to,
    //     cc,
    //     bcc,
    //     custom_headers,
    //     ..msg.headers.clone()
    // };

    // msg.headers = headers;
    // msg.body = body;
    // msg.sig = tpl.sig.map(String::from).unwrap_or(msg.sig.to_owned());
}
