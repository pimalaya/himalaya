use anyhow::{anyhow, Context, Error, Result};
use std::convert::{TryFrom, TryInto};

use crate::{
    config::MaildirBackendConfig,
    domain::{BackendService, Envelope, Envelopes, Mboxes, Msg},
};

enum Flag {
    Replied,
    Deleted,
    Draft,
    Flagged,
    Seen,
}

impl Into<char> for &Flag {
    fn into(self) -> char {
        match self {
            Flag::Replied => 'R',
            Flag::Deleted => 'T',
            Flag::Draft => 'D',
            Flag::Flagged => 'F',
            Flag::Seen => 'S',
        }
    }
}

impl TryFrom<&str> for Flag {
    type Error = Error;

    fn try_from(flag_str: &str) -> Result<Self, Self::Error> {
        match flag_str {
            "replied" => Ok(Flag::Replied),
            "deleted" => Ok(Flag::Deleted),
            "draft" => Ok(Flag::Draft),
            "flagged" => Ok(Flag::Flagged),
            "seen" => Ok(Flag::Seen),
            _ => Err(anyhow!("cannot parse flag {:?}", flag_str)),
        }
    }
}

struct Flags(Vec<Flag>);

impl ToString for Flags {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|flag| {
                let flag_char: char = flag.into();
                flag_char
            })
            .collect()
    }
}

impl TryFrom<&str> for Flags {
    type Error = Error;

    fn try_from(flags_str: &str) -> Result<Self, Self::Error> {
        let mut flags = vec![];
        for flag_str in flags_str.split_whitespace() {
            flags.push(flag_str.trim().try_into()?);
        }
        Ok(Flags(flags))
    }
}

pub struct MaildirService {
    maildir: maildir::Maildir,
}

impl<'a> BackendService<'a> for MaildirService {
    fn get_mboxes(&mut self) -> Result<Mboxes> {
        unimplemented!()
    }

    fn get_envelopes(
        &mut self,
        _mbox: &str,
        _sort: &str,
        filter: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Envelopes> {
        let mut envelopes = vec![];

        let mail_entries = match filter {
            "new" => Ok(self.maildir.list_new()),
            "cur" => Ok(self.maildir.list_cur()),
            filter => Err(anyhow!("cannot use invalid filter {:?}", filter)),
        }?;

        for mail_entry in mail_entries {
            let mut parsed_mail = mail_entry?;
            let parsed_mail = parsed_mail
                .parsed()
                .context(format!("cannot parse message"))?;
            let envelope =
                Envelope::try_from(parsed_mail).context(format!("cannot parse message"))?;
            envelopes.push(envelope);
        }

        Ok(envelopes.into())
    }

    fn add_msg(&mut self, _mbox: &str, msg: &[u8], flags: &str) -> Result<String> {
        let flags: Flags = flags.try_into()?;
        self.maildir
            .store_cur_with_flags(msg, &flags.to_string())
            .context("cannot add message to the \"cur\" folder")
    }

    fn get_msg(&mut self, _mbox: &str, id: &str) -> Result<Msg> {
        let mut mail_entry = self
            .maildir
            .find(id)
            .ok_or_else(|| anyhow!("cannot find message {:?}", id))?;
        // TODO: parse flags
        let parsed_mail = mail_entry
            .parsed()
            .context(format!("cannot parse message {:?}", id))?;
        Msg::try_from(parsed_mail).context(format!("cannot parse message {:?}", id))
    }

    fn copy_msg(&mut self, _mbox_src: &str, _mbox_dest: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn move_msg(&mut self, _mbox_src: &str, _mbox_dest: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_msg(&mut self, _mbox: &str, id: &str) -> Result<()> {
        self.maildir
            .delete(id)
            .context(format!("cannot delete message {:?}", id))
    }

    fn add_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .add_flags(id, &flags.to_string())
            .context(format!(
                "cannot add flags {:?} to message {:?}",
                flags_str, id
            ))
    }

    fn set_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .set_flags(id, &flags.to_string())
            .context(format!(
                "cannot set flags {:?} to message {:?}",
                flags_str, id
            ))
    }

    fn del_flags(&mut self, _mbox: &str, id: &str, flags_str: &str) -> Result<()> {
        let flags: Flags = flags_str.try_into()?;
        self.maildir
            .remove_flags(id, &flags.to_string())
            .context(format!(
                "cannot remove flags {:?} to message {:?}",
                flags_str, id
            ))
    }
}

impl<'a> MaildirService {
    pub fn new(maildir_config: &'a MaildirBackendConfig) -> Self {
        Self {
            maildir: maildir_config.maildir_dir.clone().into(),
        }
    }
}
