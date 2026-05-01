use std::io::{stdin, BufRead, IsTerminal};
#[cfg(any(feature = "smtp", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
};

#[cfg(any(feature = "smtp", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Send a message via the active account.
///
/// Supported over SMTP and JMAP. JMAP requires `identity-id` and
/// `drafts-mailbox-id` to be set on the account's `[jmap]` config block.
#[derive(Debug, Parser)]
pub struct MessagesSendCommand {
    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl MessagesSendCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        mut account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let raw = if stdin().is_terminal() || printer.is_json() {
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

        #[cfg(feature = "smtp")]
        if backend.allows_smtp() {
            if let Some(smtp_config) = account_config.smtp.take() {
                use io_email::smtp::message_send::{MessageSend, MessageSendResult};
                use pimalaya_toolbox::stream::smtp::SmtpSession;

                let account = crate::account::Account::new(config, account_config, smtp_config)?;
                let mut session = SmtpSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let (reverse_path, forward_paths) = parse_envelope(raw.as_bytes())?;
                let mut coroutine = MessageSend::new(reverse_path, forward_paths, raw.into_bytes());
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageSendResult::Ok => break,
                        MessageSendResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageSendResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageSendResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message successfully sent"));
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::message_send::{MessageSend, MessageSendResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let identity_id = jmap_config.identity_id.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "JMAP send requires `identity-id` in the [jmap] config; \
                         run `himalaya jmap identity get` to find one"
                    )
                })?;
                let drafts_mailbox_id = jmap_config.drafts_mailbox_id.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "JMAP send requires `drafts-mailbox-id` in the [jmap] config; \
                         run `himalaya jmap mailbox query --role drafts` to find one"
                    )
                })?;
                let account = crate::account::Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;

                let mut coroutine = MessageSend::new(
                    &session.session,
                    &session.http_auth,
                    raw.into_bytes(),
                    identity_id,
                    drafts_mailbox_id,
                )?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageSendResult::Ok => break,
                        MessageSendResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageSendResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageSendResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message successfully sent"));
            }
        }

        let _ = config;
        let _ = raw;
        bail!("no backend matching `{backend}` is configured for this account")
    }
}

#[cfg(feature = "smtp")]
pub(crate) fn parse_envelope<'a>(
    msg: &[u8],
) -> Result<(
    io_smtp::rfc5321::types::reverse_path::ReversePath<'a>,
    Vec<io_smtp::rfc5321::types::forward_path::ForwardPath<'a>>,
)> {
    use std::{borrow::Cow, collections::HashSet};

    use io_smtp::rfc5321::types::{
        domain::Domain, ehlo_domain::EhloDomain, forward_path::ForwardPath, local_part::LocalPart,
        mailbox::Mailbox, reverse_path::ReversePath,
    };
    use mail_parser::{Address, HeaderName, HeaderValue, MessageParser};

    let Some(parsed) = MessageParser::new().parse_headers(msg) else {
        bail!("Invalid message to send")
    };

    let mut mail_from = None;
    let mut rcpt_to = HashSet::new();

    for header in parsed.headers() {
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

    for rcpt in rcpt_to {
        let Some((local, domain)) = rcpt.split_once('@') else {
            bail!("The message contains an invalid recipient: {rcpt}");
        };

        let mbox = Mailbox {
            local_part: LocalPart(Cow::Owned(local.to_owned())),
            domain: EhloDomain::Domain(Domain(Cow::Owned(domain.to_owned()))),
        };

        forward_paths.push(ForwardPath(mbox))
    }

    Ok((reverse_path, forward_paths))
}

#[cfg(feature = "smtp")]
fn find_valid_email(addr: &mail_parser::Addr) -> Option<String> {
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
