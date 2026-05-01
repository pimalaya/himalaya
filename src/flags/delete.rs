#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
    flags::arg::{FlagsArg, MailboxFlag, MessageIdsArg},
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Remove flag(s) from message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagsDeleteCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
    #[command(flatten)]
    pub mailbox: MailboxFlag,
}

impl FlagsDeleteCommand {
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
                use io_email::imap::flag_delete::{FlagDelete, FlagDeleteResult};
                use io_imap::types::{mailbox::Mailbox, sequence::SequenceSet};
                use pimalaya_toolbox::stream::imap::ImapSession;

                let account = Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let mailbox: Mailbox<'static> = self.mailbox.inner.clone().try_into()?;
                let sequence_set: SequenceSet = self.ids.inner.join(",").as_str().try_into()?;
                let imap_flags = self.flags.inner.iter().map(|f| f.imap()).collect();
                let mut coroutine =
                    FlagDelete::new(session.context, mailbox, sequence_set, imap_flags, true);
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        FlagDeleteResult::Ok => break,
                        FlagDeleteResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        FlagDeleteResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        FlagDeleteResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Flag(s) successfully removed"));
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::flag_delete::{FlagDelete, FlagDeleteResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let account = Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;

                let keywords: Vec<String> = self
                    .flags
                    .inner
                    .iter()
                    .map(|f| f.jmap().to_owned())
                    .collect();
                let mut coroutine = FlagDelete::new(
                    &session.session,
                    &session.http_auth,
                    self.ids.inner.iter().cloned(),
                    keywords,
                )?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                loop {
                    match coroutine.resume(arg.take()) {
                        FlagDeleteResult::Ok => break,
                        FlagDeleteResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        FlagDeleteResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        FlagDeleteResult::Err(err) => bail!("{err}"),
                    }
                }

                return printer.out(Message::new("Flag(s) successfully removed"));
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::flag_delete::{FlagDelete, FlagDeleteArg, FlagDeleteResult};
                use io_maildir::{
                    flag::{Flag, Flags},
                    maildir::Maildir,
                };

                use crate::maildir::runtime;

                let account = Account::new(config, account_config, maildir_config)?;
                let path = account.backend.root.join(&self.mailbox.inner);
                let maildir = Maildir::try_from(path)?;
                let maildir_flags: Flags = self.flags.inner.iter().map(|f| Flag::from(f)).collect();

                for id in &self.ids.inner {
                    let mut coroutine =
                        FlagDelete::new(maildir.clone(), id.as_str(), maildir_flags.clone());
                    let mut arg: Option<FlagDeleteArg> = None;

                    loop {
                        match coroutine.resume(arg.take()) {
                            FlagDeleteResult::Ok => break,
                            FlagDeleteResult::WantsDirRead(paths) => {
                                arg = Some(FlagDeleteArg::DirRead(read_dirs(&paths)?));
                            }
                            FlagDeleteResult::WantsRename(pairs) => {
                                runtime::rename(pairs)?;
                                arg = Some(FlagDeleteArg::Rename);
                            }
                            FlagDeleteResult::Err(err) => bail!("{err}"),
                        }
                    }
                }

                return printer.out(Message::new("Flag(s) successfully removed"));
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
}

#[cfg(feature = "maildir")]
fn read_dirs(
    paths: &std::collections::BTreeSet<String>,
) -> Result<std::collections::BTreeMap<String, std::collections::BTreeSet<String>>> {
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
    };

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
