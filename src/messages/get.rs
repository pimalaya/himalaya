#[cfg(feature = "maildir")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use std::fmt;
#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::Read;
use std::io::{stdout, Write};

use anyhow::{bail, Result};
use clap::Parser;
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use mail_parser::{Message, MessageParser};
use pimalaya_toolbox::terminal::printer::Printer;
#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
use serde::Serialize;

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Get a message from the active account.
///
/// By default the message is parsed and rendered as headers + text
/// bodies. Pass `--raw` to dump the original RFC 5322 bytes to stdout
/// instead, or use the global `--json` flag to emit the parsed message
/// as JSON.
#[derive(Debug, Parser)]
pub struct MessagesGetCommand {
    /// Identifier of the message (IMAP UID, JMAP email id, or Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox name or path (IMAP mailbox / Maildir path). Ignored for
    /// JMAP, which addresses messages by id directly.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Write the raw RFC 5322 bytes to stdout. Mutually exclusive with
    /// the global `--json` flag.
    #[arg(long)]
    pub raw: bool,
}

impl MessagesGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        mut account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        if self.raw && printer.is_json() {
            bail!("`--raw` and `--json` cannot be combined");
        }

        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use std::num::NonZeroU32;

                use io_email::imap::message_get::{MessageGet, MessageGetResult};
                use io_imap::types::mailbox::Mailbox;
                use pimalaya_toolbox::stream::imap::ImapSession;

                let account = Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let mailbox: Mailbox<'static> = self.mailbox.clone().try_into()?;
                let id: NonZeroU32 = self.id.parse()?;
                let mut coroutine = MessageGet::new(session.context, mailbox, id, true);
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let raw = loop {
                    match coroutine.resume(arg.take()) {
                        MessageGetResult::Ok(raw) => break raw,
                        MessageGetResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageGetResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageGetResult::Err(err) => bail!("{err}"),
                    }
                };

                return emit(printer, raw, self.raw);
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::message_get::{MessageGet, MessageGetResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let account = Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;
                let mut coroutine =
                    MessageGet::new(&session.session, &session.http_auth, &self.id)?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let raw = loop {
                    match coroutine.resume(arg.take()) {
                        MessageGetResult::Ok(raw) => break raw,
                        MessageGetResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MessageGetResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MessageGetResult::Err(err) => bail!("{err}"),
                    }
                };

                return emit(printer, raw, self.raw);
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::message_get::{MessageGet, MessageGetArg, MessageGetResult};
                use io_maildir::maildir::Maildir;

                let account = Account::new(config, account_config, maildir_config)?;
                let path = account.backend.root.join(&self.mailbox);
                let maildir = Maildir::try_from(path)?;

                let mut coroutine = MessageGet::new(maildir, &self.id);
                let mut arg: Option<MessageGetArg> = None;

                let raw = loop {
                    match coroutine.resume(arg.take()) {
                        MessageGetResult::Ok(raw) => break raw,
                        MessageGetResult::WantsDirRead(paths) => {
                            arg = Some(MessageGetArg::DirRead(read_dirs(&paths)?));
                        }
                        MessageGetResult::WantsFileRead(paths) => {
                            arg = Some(MessageGetArg::FileRead(read_files(&paths)?));
                        }
                        MessageGetResult::Err(err) => bail!("{err}"),
                    }
                };

                return emit(printer, raw, self.raw);
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
}

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
fn emit(printer: &mut impl Printer, raw: Vec<u8>, raw_mode: bool) -> Result<()> {
    if raw_mode {
        let mut out = stdout().lock();
        out.write_all(&raw)?;
        return Ok(());
    }

    let Some(parsed) = MessageParser::new().parse(&raw) else {
        bail!("Failed to parse RFC 5322 message");
    };

    printer.out(MessageView(parsed.into_owned()))
}

#[cfg(feature = "maildir")]
fn read_dirs(paths: &BTreeSet<String>) -> Result<BTreeMap<String, BTreeSet<String>>> {
    use std::fs;

    let mut out = BTreeMap::new();

    for path in paths {
        let mut entries = BTreeSet::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            if let Some(s) = entry.path().to_str() {
                entries.insert(s.to_owned());
            }
        }

        out.insert(path.clone(), entries);
    }

    Ok(out)
}

#[cfg(feature = "maildir")]
fn read_files(paths: &BTreeSet<String>) -> Result<BTreeMap<String, Vec<u8>>> {
    use std::fs;

    let mut out = BTreeMap::new();

    for path in paths {
        out.insert(path.clone(), fs::read(path)?);
    }

    Ok(out)
}

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageView(Message<'static>);

#[cfg(any(feature = "imap", feature = "jmap", feature = "maildir"))]
impl fmt::Display for MessageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for header in self.0.headers() {
            writeln!(f, "{}: {:?}", header.name.as_str(), header.value)?;
        }

        writeln!(f)?;

        for (i, part) in self.0.text_bodies().enumerate() {
            if i > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            if let Some(contents) = part.text_contents() {
                write!(f, "{}", contents.trim_end())?;
            }
        }

        Ok(())
    }
}
