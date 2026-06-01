// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{borrow::Cow, collections::HashSet};

use anyhow::{Result, bail};
use clap::Parser;
use io_smtp::rfc5321::types::{
    domain::Domain, ehlo_domain::EhloDomain, forward_path::ForwardPath, local_part::LocalPart,
    mailbox::Mailbox, reverse_path::ReversePath,
};
use mail_parser::{Addr, Address, HeaderName, HeaderValue, MessageParser};
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::messages::arg::MessageArg, smtp::client::SmtpClient};

/// Send a raw RFC 5322 message via SMTP.
///
/// The message can be passed as a positional file path, an inline
/// raw string, or piped via stdin (see [`MessageArg`] for resolution
/// order). The envelope sender is taken from the `From:` header and
/// recipients are collected from `To:` / `Cc:` / `Bcc:`.
#[derive(Debug, Parser)]
pub struct SmtpMessageSendCommand {
    #[command(flatten)]
    pub message: MessageArg,
}

impl SmtpMessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        let message = self.message.parse()?;
        let (reverse_path, forward_paths) = into_smtp_msg(message.as_bytes())?;
        client.send(reverse_path, forward_paths, message.into_bytes())?;
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

    let mbox = Mailbox {
        local_part: LocalPart(Cow::Owned(local.to_owned())),
        domain: EhloDomain::Domain(Domain(Cow::Owned(domain.to_owned()))),
    };

    let reverse_path = ReversePath::Mailbox(mbox);

    let mut forward_paths = Vec::new();

    for rcpt_to in rcpt_to {
        let Some((local, domain)) = rcpt_to.split_once('@') else {
            bail!("The message contains an invalid recipient: {rcpt_to}");
        };

        let mbox = Mailbox {
            local_part: LocalPart(Cow::Owned(local.to_owned())),
            domain: EhloDomain::Domain(Domain(Cow::Owned(domain.to_owned()))),
        };

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
