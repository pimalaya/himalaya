use io_maildir::client::MaildirClient;

use crate::{account::Account, config::MaildirConfig};

pub type MaildirAccount = Account<MaildirConfig>;

impl MaildirAccount {
    /// Builds a [`MaildirClient`] rooted at the configured Maildir
    /// path.
    pub fn new_maildir_client(&self) -> MaildirClient {
        MaildirClient::new(self.backend.root.clone())
    }
}
