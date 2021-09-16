use std::convert::TryFrom;

use anyhow::Result;
use log::trace;

use crate::{
    domain::{
        account::entity::Account,
        imap::service::ImapServiceInterface,
        msg::entity::{Msg, MsgSerialized},
    },
    output::service::OutputServiceInterface,
};

pub fn new<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let msg = Msg::new(&account);
    // FIXME
    // override_msg_with_args(&mut msg, &matches);
    trace!("message: {:#?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    imap.logout()?;
    Ok(())
}

pub fn reply<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    all: bool,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let mut msg = imap.get_msg(&uid)?;
    msg.change_to_reply(&account, all)?;
    // FIXME
    // override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    imap.logout()?;
    Ok(())
}

pub fn forward<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &str,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let mut msg = imap.get_msg(&uid)?;
    msg.change_to_forwarding(&account);
    // FIXME
    // override_msg_with_args(&mut msg, &matches);
    trace!("Message: {:?}", msg);
    output.print(MsgSerialized::try_from(&msg)?)?;
    imap.logout()?;
    Ok(())
}

// == Helper functions ==
// -- Template Subcommands --
// These functions are more used for the "template" subcommand
// fn override_msg_with_args(msg: &mut Msg) {
//     // -- Collecting credentials --
//     let from: Vec<String> = match matches.values_of("from") {
//         Some(from) => from.map(|arg| arg.to_string()).collect(),
//         None => msg.headers.from.clone(),
//     };

//     let to: Vec<String> = match matches.values_of("to") {
//         Some(to) => to.map(|arg| arg.to_string()).collect(),
//         None => Vec::new(),
//     };

//     let subject = matches
//         .value_of("subject")
//         .and_then(|subject| Some(subject.to_string()));

//     let cc: Option<Vec<String>> = matches
//         .values_of("cc")
//         .and_then(|cc| Some(cc.map(|arg| arg.to_string()).collect()));

//     let bcc: Option<Vec<String>> = matches
//         .values_of("bcc")
//         .and_then(|bcc| Some(bcc.map(|arg| arg.to_string()).collect()));

//     let signature = matches
//         .value_of("signature")
//         .and_then(|signature| Some(signature.to_string()))
//         .or(msg.headers.signature.clone());

//     let custom_headers: Option<HashMap<String, Vec<String>>> = {
//         if let Some(matched_headers) = matches.values_of("header") {
//             let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();

//             // collect the custom headers
//             for header in matched_headers {
//                 let mut header = header.split(":");
//                 let key = header.next().unwrap_or_default();
//                 let val = header.next().unwrap_or_default().trim_start();

//                 debug!("overriden header: {}={}", key, val);

//                 custom_headers.insert(key.to_string(), vec![val.to_string()]);
//             }

//             Some(custom_headers)
//         } else {
//             None
//         }
//     };

//     let body = {
//         if atty::isnt(Stream::Stdin) {
//             let body = io::stdin()
//                 .lock()
//                 .lines()
//                 .filter_map(|line| line.ok())
//                 .map(|line| line.to_string())
//                 .collect::<Vec<String>>()
//                 .join("\n");
//             debug!("overriden body from stdin: {:?}", body);
//             body
//         } else if let Some(body) = matches.value_of("body") {
//             debug!("overriden body: {:?}", body);
//             body.to_string()
//         } else {
//             String::new()
//         }
//     };

//     let body = Body::new_with_text(body);

//     // -- Creating and printing --
//     let headers = Headers {
//         from,
//         subject,
//         to,
//         cc,
//         bcc,
//         signature,
//         custom_headers,
//         ..msg.headers.clone()
//     };

//     msg.headers = headers;
//     msg.body = body;
// }
