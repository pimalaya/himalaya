#[cfg(feature = "maildir")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
    mailboxes::table::MailboxesTable,
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 8 * 1024;

/// List mailboxes for the active account, regardless of the underlying
/// backend (IMAP, JMAP or Maildir).
#[derive(Debug, Parser)]
pub struct MailboxesListCommand;

impl MailboxesListCommand {
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
                use io_email::imap::mailbox_list::{MailboxList, MailboxListResult};
                use pimalaya_toolbox::stream::imap::ImapSession;

                let account = Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;
                let mut coroutine = MailboxList::new(session.context);
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let mailboxes = loop {
                    match coroutine.resume(arg.take()) {
                        MailboxListResult::Ok(mailboxes) => break mailboxes,
                        MailboxListResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MailboxListResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MailboxListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(MailboxesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    mailboxes,
                });
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::mailbox_list::{MailboxList, MailboxListResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let account = Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;
                let mut coroutine = MailboxList::new(&session.session, &session.http_auth)?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let mailboxes = loop {
                    match coroutine.resume(arg.take()) {
                        MailboxListResult::Ok(mailboxes) => break mailboxes,
                        MailboxListResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        MailboxListResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        MailboxListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(MailboxesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    mailboxes,
                });
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::mailbox_list::{
                    MailboxList, MailboxListArg, MailboxListResult,
                };

                let account = Account::new(config, account_config, maildir_config)?;
                let mut coroutine = MailboxList::new(&account.backend.root);
                let mut arg: Option<MailboxListArg> = None;

                let mailboxes = loop {
                    match coroutine.resume(arg.take()) {
                        MailboxListResult::Ok(mailboxes) => break mailboxes,
                        MailboxListResult::WantsDirRead(paths) => {
                            arg = Some(MailboxListArg::DirRead(read_dirs(&paths)?));
                        }
                        MailboxListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(MailboxesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    mailboxes,
                });
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
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
