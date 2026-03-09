use std::{
    collections::HashSet,
    io::{stdin, BufRead, IsTerminal},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_smtp::{
    coroutines::send_message::*,
    types::core::{EhloDomain, ForwardPath, Mailbox, ReversePath},
};
use io_stream::runtimes::std::handle;
use mail_parser::{Addr, Address, HeaderName, HeaderValue, MessageParser};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::smtp::{account::SmtpAccount, stream};

/// Send a message to a mailbox.
///
/// This command appends a message to the specified mailbox. The
/// message is read from stdin in RFC 5322 format (raw email).
#[derive(Debug, Parser)]
pub struct SendMessageCommand {
    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl SendMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: SmtpAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let message = if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
        } else {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        let (reverse_path, forward_paths) = into_smtp_msg(message.as_bytes())?;

        let mut arg = None;
        let mut coroutine =
            SendSmtpMessage::new(context, reverse_path, forward_paths, message.into_bytes());

        loop {
            match coroutine.resume(arg.take()) {
                SendSmtpMessageResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                SendSmtpMessageResult::Ok { .. } => break,
                SendSmtpMessageResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Message successfully sent"))
    }
}

fn into_smtp_msg<'a>(msg: &[u8]) -> Result<(ReversePath<'a>, Vec<ForwardPath<'a>>)> {
    let Some(msg) = MessageParser::new().parse_headers(msg) else {
        bail!("Invalid message to send")
    };

    let mut mail_from = None;
    let mut rcpt_to = HashSet::new();

    for header in msg.headers() {
        let key = &header.name;
        let val = header.value();

        match key {
            HeaderName::From => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    if let Some(email) = addrs.first().and_then(find_valid_email) {
                        mail_from = email.to_string().into();
                    }
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    if let Some(group) = groups.first() {
                        if let Some(email) = group.addresses.first().and_then(find_valid_email) {
                            mail_from = email.to_string().into();
                        }
                    }
                }
                _ => (),
            },
            HeaderName::To | HeaderName::Cc | HeaderName::Bcc => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    rcpt_to.extend(addrs.iter().filter_map(find_valid_email));
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    rcpt_to.extend(
                        groups
                            .iter()
                            .flat_map(|group| group.addresses.iter())
                            .filter_map(find_valid_email),
                    );
                }
                _ => (),
            },
            _ => (),
        };
    }

    let Some(mail_from) = mail_from else {
        bail!("The message does not contain any sender");
    };

    if rcpt_to.is_empty() {
        bail!("The message does not contain any recipient");
    }

    let Some((local, domain)) = mail_from.split_once('@') else {
        bail!("The message contains an invalid sender");
    };

    let mbox = Mailbox::new(
        local.to_owned().try_into()?,
        EhloDomain::Domain(domain.to_owned().try_into()?),
    );

    let reverse_path = ReversePath::Mailbox(mbox);

    let mut forward_paths = Vec::new();

    for rcpt_to in rcpt_to {
        let Some((local, domain)) = rcpt_to.split_once('@') else {
            bail!("The message contains an invalid recipient: {rcpt_to}");
        };

        let mbox = Mailbox::new(
            local.to_owned().try_into()?,
            EhloDomain::Domain(domain.to_owned().try_into()?),
        );

        forward_paths.push(ForwardPath(mbox))
    }

    Ok((reverse_path, forward_paths))
}

fn find_valid_email(addr: &Addr) -> Option<String> {
    match &addr.address {
        None => None,
        Some(email) => {
            let email = email.trim();
            if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            }
        }
    }
}
