#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::Write as _;
use std::{
    io::{stdin, IsTerminal, Read},
    path::PathBuf,
};

use anyhow::{bail, Result};
use clap::Parser;
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use pimalaya_toolbox::terminal::printer::Message;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    flags::arg::FlagArg,
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message body is read from stdin by default; pass `--file
/// <PATH>` to read from a file instead. IMAP appends via `APPEND`
/// (RFC 3501); JMAP uploads the blob and imports it via `Email/import`
/// (the destination mailbox is resolved from `--mailbox` by exact-match
/// name); Maildir writes a new file under the target maildir's `cur/`
/// subdir using the standard tmp-then-rename delivery protocol.
#[derive(Debug, Parser)]
pub struct MessagesAddCommand {
    /// Destination mailbox name or path. Mandatory.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,

    /// Flag(s) to set on the new message. Optional.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,

    /// Read the raw message from this file instead of stdin.
    #[arg(long = "file", value_name = "PATH")]
    pub file: Option<PathBuf>,
}

impl MessagesAddCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        #[cfg_attr(
            not(any(feature = "imap", feature = "jmap", feature = "maildir")),
            allow(unused_mut)
        )]
        mut account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let raw = read_raw(&self.file)?;

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use io_email::imap::message_add::{MessageAdd, MessageAddResult};
                use io_imap::types::mailbox::Mailbox;
                use pimalaya_toolbox::stream::imap::ImapSession;

                let account = crate::account::Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let mailbox: Mailbox<'static> = self.mailbox.clone().try_into()?;
                let imap_flags = self.flag.iter().map(|f| f.imap()).collect();
                let mut coroutine = MessageAdd::new(session.context, mailbox, imap_flags, raw)?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageAddResult::Ok { .. } => break,
                        MessageAddResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageAddResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageAddResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message successfully added"));
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::message_add::{MessageAdd, MessageAddResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let account = crate::account::Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;

                let keywords: Vec<String> = self.flag.iter().map(|f| f.jmap().to_owned()).collect();

                let mut coroutine = MessageAdd::new(
                    &session.session,
                    &session.http_auth,
                    raw,
                    &self.mailbox,
                    keywords,
                )?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageAddResult::Ok { .. } => break,
                        MessageAddResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageAddResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageAddResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message successfully added"));
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::message_add::{MessageAdd, MessageAddArg, MessageAddResult};
                use io_maildir::{
                    flag::{Flag as MaildirFlag, Flags as MaildirFlags},
                    maildir::Maildir,
                };

                use crate::maildir::runtime;

                let account = crate::account::Account::new(config, account_config, maildir_config)?;
                let path = account.backend.root.join(&self.mailbox);
                let maildir = Maildir::try_from(path)?;

                let flags: MaildirFlags = self.flag.iter().map(MaildirFlag::from).collect();
                let mut coroutine = MessageAdd::new(maildir, None, flags, raw);
                let mut arg: Option<MessageAddArg> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        MessageAddResult::Ok { .. } => break,
                        MessageAddResult::WantsFileCreate(files) => {
                            runtime::file_create(files)?;
                            arg = Some(MessageAddArg::FileCreate);
                        }
                        MessageAddResult::WantsRename(pairs) => {
                            runtime::rename(pairs)?;
                            arg = Some(MessageAddArg::Rename);
                        }
                        MessageAddResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Message successfully added"));
            }
        }

        let _ = config;
        let _ = account_config;
        let _ = raw;
        let _ = printer;
        bail!("no backend matching `{backend}` is configured for this account")
    }
}

fn read_raw(file: &Option<PathBuf>) -> Result<Vec<u8>> {
    if let Some(path) = file {
        return Ok(std::fs::read(path)?);
    }

    if stdin().is_terminal() {
        bail!(
            "`messages add` reads the raw message from stdin or `--file <PATH>` — \
             nothing was provided"
        );
    }

    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf)?;
    Ok(buf)
}
