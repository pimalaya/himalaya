//! Shared helper for fetching the raw RFC 5322 bytes of a single
//! message via the active account's backend.
//!
//! Used by `messages compose --reply/--forward`, `attachments list`,
//! `attachments download`, etc. Cross-backend commands always treat
//! the IMAP id as a UID — sequence-number addressing belongs to the
//! protocol-specific `imap` subcommands.

#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Fetches the raw RFC 5322 bytes of `id` from `mailbox` via the first
/// configured backend that matches `backend`. Bails when no backend
/// matches.
pub(crate) fn fetch_raw(
    config: &Config,
    account_config: &AccountConfig,
    backend: BackendArg,
    mailbox: &str,
    id: &str,
) -> Result<Vec<u8>> {
    #[cfg(feature = "imap")]
    if backend.allows_imap() {
        if let Some(imap_config) = account_config.imap.clone() {
            use std::num::NonZeroU32;

            use io_email::imap::message_get::{MessageGet, MessageGetResult};
            use io_imap::types::mailbox::Mailbox;
            use pimalaya_toolbox::stream::imap::ImapSession;

            let account =
                crate::account::Account::new(config.clone(), account_config.clone(), imap_config)?;
            let mut session = ImapSession::new(
                account.backend.url.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.starttls,
                account.backend.sasl.clone().try_into()?,
            )?;

            let imap_mailbox: Mailbox<'static> = mailbox.to_owned().try_into()?;
            let id: NonZeroU32 = id.parse()?;
            let mut coroutine = MessageGet::new(session.context, imap_mailbox, id, true);
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut arg: Option<&[u8]> = None;

            return loop {
                match coroutine.resume(arg.take()) {
                    MessageGetResult::Ok(raw) => break Ok(raw),
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
        }
    }

    #[cfg(feature = "jmap")]
    if backend.allows_jmap() {
        if let Some(jmap_config) = account_config.jmap.clone() {
            use io_email::jmap::message_get::{MessageGet, MessageGetResult};
            use pimalaya_toolbox::stream::jmap::JmapSession;

            let _ = mailbox;
            let account =
                crate::account::Account::new(config.clone(), account_config.clone(), jmap_config)?;
            let mut session = JmapSession::new(
                account.backend.server.clone(),
                account.backend.tls.clone().try_into()?,
                account.backend.auth.clone().try_into()?,
            )?;
            let mut coroutine = MessageGet::new(&session.session, &session.http_auth, id)?;
            let mut buf = [0u8; READ_BUFFER_SIZE];
            let mut arg: Option<&[u8]> = None;

            return loop {
                match coroutine.resume(arg.take()) {
                    MessageGetResult::Ok(raw) => break Ok(raw),
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
        }
    }

    #[cfg(feature = "maildir")]
    if backend.allows_maildir() {
        if let Some(maildir_config) = account_config.maildir.clone() {
            use io_email::maildir::message_get::{MessageGet, MessageGetArg, MessageGetResult};
            use io_maildir::maildir::Maildir;

            let account = crate::account::Account::new(
                config.clone(),
                account_config.clone(),
                maildir_config,
            )?;
            let path = account.backend.root.join(mailbox);
            let maildir = Maildir::try_from(path)?;

            let mut coroutine = MessageGet::new(maildir, id);
            let mut arg: Option<MessageGetArg> = None;

            return loop {
                match coroutine.resume(arg.take()) {
                    MessageGetResult::Ok(raw) => break Ok(raw),
                    MessageGetResult::WantsDirRead(paths) => {
                        arg = Some(MessageGetArg::DirRead(crate::maildir::runtime::dir_read(
                            paths,
                        )?));
                    }
                    MessageGetResult::WantsFileRead(paths) => {
                        arg = Some(MessageGetArg::FileRead(crate::maildir::runtime::file_read(
                            paths,
                        )?));
                    }
                    MessageGetResult::Err(err) => bail!("{err}"),
                }
            };
        }
    }

    bail!("no backend matching `{backend}` is configured for this account")
}
