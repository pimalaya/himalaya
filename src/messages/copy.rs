#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
    flags::arg::MessageIdsArg,
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Copy message(s) from one mailbox to another within the active
/// account.
///
/// IMAP uses `UID COPY` (RFC 3501); JMAP uses `Email/set` patches that
/// add the destination to each email's `mailboxIds`; Maildir copies
/// the underlying file. Cross-account / cross-backend copy is out of
/// scope.
#[derive(Debug, Parser)]
pub struct MessagesCopyCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    /// Source mailbox name or path (IMAP/Maildir). For JMAP this is
    /// resolved by exact-match name against `Mailbox/get`.
    #[arg(
        long = "from",
        short = 'f',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub from: String,

    /// Destination mailbox name or path. Mandatory.
    #[arg(long = "to", short = 't', value_name = "NAME")]
    pub to: String,
}

impl MessagesCopyCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        mut account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use crate::imap::session::ImapSession;
                use io_email::imap::message_copy::{MessageCopy, MessageCopyResult};
                use io_imap::types::{mailbox::Mailbox, sequence::SequenceSet};

                let account = Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let from: Mailbox<'static> = self.from.clone().try_into()?;
                let to: Mailbox<'static> = self.to.clone().try_into()?;
                let sequence_set: SequenceSet = self.ids.inner.join(",").as_str().try_into()?;
                let mut coroutine = MessageCopy::new(session.context, from, to, sequence_set, true);
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageCopyResult::Ok => break,
                        MessageCopyResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageCopyResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageCopyResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message(s) successfully copied"));
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use crate::jmap::session::JmapSession;
                use io_email::jmap::message_copy::{MessageCopy, MessageCopyResult};

                let account = Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;

                let mut coroutine = MessageCopy::new(
                    &session.session,
                    &session.http_auth,
                    self.ids.inner.iter().cloned(),
                    &self.to,
                )?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageCopyResult::Ok => break,
                        MessageCopyResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageCopyResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageCopyResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message(s) successfully copied"));
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::message_copy::{
                    MessageCopy, MessageCopyArg, MessageCopyResult,
                };
                use io_maildir::maildir::Maildir;

                use crate::maildir::runtime;

                let account = Account::new(config, account_config, maildir_config)?;
                let source = Maildir::try_from(account.backend.root.join(&self.from))?;
                let target = Maildir::try_from(account.backend.root.join(&self.to))?;

                for id in &self.ids.inner {
                    let mut coroutine =
                        MessageCopy::new(id.as_str(), source.clone(), target.clone(), None);
                    let mut arg: Option<MessageCopyArg> = None;

                    loop {
                        match coroutine.resume(arg.take()) {
                            MessageCopyResult::Ok => break,
                            MessageCopyResult::WantsDirRead(paths) => {
                                arg = Some(MessageCopyArg::DirRead(runtime::dir_read(paths)?));
                            }
                            MessageCopyResult::WantsCopy(pairs) => {
                                runtime::copy(pairs)?;
                                arg = Some(MessageCopyArg::Copy);
                            }
                            MessageCopyResult::Err(err) => bail!("{err}"),
                        }
                    }
                }

                return printer.out(Message::new("Message(s) successfully copied"));
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
}
