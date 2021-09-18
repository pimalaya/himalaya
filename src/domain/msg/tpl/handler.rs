use std::{
    collections::HashMap,
    convert::TryFrom,
    io::{self, BufRead},
};

use anyhow::Result;
use atty::Stream;
use clap::Values;
use log::{debug, trace};

use crate::{
    config::entity::Account,
    domain::{
        imap::service::ImapServiceInterface,
        msg::{
            body::Body,
            entity::{Msg, MsgSerialized},
            headers::Headers,
        },
    },
    output::service::OutputServiceInterface,
};

pub fn new<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    subject: Option<&'a str>,
    from: Option<Values<'a>>,
    to: Option<Values<'a>>,
    cc: Option<Values<'a>>,
    bcc: Option<Values<'a>>,
    headers: Option<Values<'a>>,
    body: Option<&'a str>,
    sig: Option<&'a str>,
    account: &Account,
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let mut msg = Msg::new(&account);
    override_msg_with_args(&mut msg, subject, from, to, cc, bcc, headers, body, sig);
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
fn override_msg_with_args<'a>(
    msg: &mut Msg,
    subject: Option<&'a str>,
    from: Option<Values<'a>>,
    to: Option<Values<'a>>,
    cc: Option<Values<'a>>,
    bcc: Option<Values<'a>>,
    headers: Option<Values<'a>>,
    body: Option<&'a str>,
    sig: Option<&'a str>,
) {
    // -- Collecting credentials --
    let from: Vec<String> = match from {
        Some(from) => from.map(|arg| arg.to_string()).collect(),
        None => msg.headers.from.clone(),
    };

    let to: Vec<String> = match to {
        Some(to) => to.map(|arg| arg.to_string()).collect(),
        None => Vec::new(),
    };

    let subject = subject.map(String::from);
    let cc: Option<Vec<String>> = cc.map(|cc| cc.map(|arg| arg.to_string()).collect());
    let bcc: Option<Vec<String>> = bcc.map(|bcc| bcc.map(|arg| arg.to_string()).collect());
    let signature = sig.map(String::from).or(msg.headers.signature.to_owned());

    let custom_headers: Option<HashMap<String, Vec<String>>> = {
        if let Some(matched_headers) = headers {
            let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();

            // collect the custom headers
            for header in matched_headers {
                let mut header = header.split(":");
                let key = header.next().unwrap_or_default();
                let val = header.next().unwrap_or_default().trim_start();

                custom_headers.insert(key.to_string(), vec![val.to_string()]);
            }

            Some(custom_headers)
        } else {
            None
        }
    };

    let body = {
        if atty::isnt(Stream::Stdin) {
            let body = io::stdin()
                .lock()
                .lines()
                .filter_map(|line| line.ok())
                .map(|line| line.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            debug!("overriden body from stdin: {:?}", body);
            body
        } else if let Some(body) = body {
            debug!("overriden body: {:?}", body);
            body.to_string()
        } else {
            String::new()
        }
    };

    let body = Body::new_with_text(body);

    // -- Creating and printing --
    let headers = Headers {
        from,
        subject,
        to,
        cc,
        bcc,
        signature,
        custom_headers,
        ..msg.headers.clone()
    };

    msg.headers = headers;
    msg.body = body;
}
