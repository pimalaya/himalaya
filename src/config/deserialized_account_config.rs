use serde::Deserialize;
use std::path::PathBuf;

pub trait ToDeserializedBaseAccountConfig {
    fn to_base(&self) -> DeserializedBaseAccountConfig;
}

/// Represents all existing kind of account config.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DeserializedAccountConfig {
    Imap(DeserializedImapAccountConfig),
    Maildir(DeserializedMaildirAccountConfig),
}

impl ToDeserializedBaseAccountConfig for DeserializedAccountConfig {
    fn to_base(&self) -> DeserializedBaseAccountConfig {
        match self {
            Self::Imap(config) => config.to_base(),
            Self::Maildir(config) => config.to_base(),
        }
    }
}

macro_rules! make_account_config {
    ($AccountConfig:ident, $($element: ident: $ty: ty),*) => {
	#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
	#[serde(rename_all = "kebab-case")]
	pub struct $AccountConfig {
	    /// Overrides the display name of the user for this account.
            pub name: Option<String>,
            /// Overrides the downloads directory (mostly for attachments).
            pub downloads_dir: Option<PathBuf>,
            /// Overrides the signature for this account.
            pub signature: Option<String>,
            /// Overrides the signature delimiter for this account.
            pub signature_delimiter: Option<String>,
            /// Overrides the default page size for this account.
            pub default_page_size: Option<usize>,
            /// Overrides the inbox folder name for this account.
            pub inbox_folder: Option<String>,
            /// Overrides the sent folder name for this account.
            pub sent_folder: Option<String>,
            /// Overrides the draft folder name for this account.
            pub draft_folder: Option<String>,
            /// Overrides the notify command for this account.
            pub notify_cmd: Option<String>,
            /// Overrides the IMAP query used to fetch new messages for this account.
            pub notify_query: Option<String>,
            /// Overrides the watch commands for this account.
            pub watch_cmds: Option<Vec<String>>,

            /// Makes this account the default one.
            pub default: Option<bool>,
            /// Represents the account email address.
            pub email: String,

            /// Represents the SMTP host.
            pub smtp_host: String,
            /// Represents the SMTP port.
            pub smtp_port: u16,
            /// Enables StartTLS.
            pub smtp_starttls: Option<bool>,
            /// Trusts any certificate.
            pub smtp_insecure: Option<bool>,
            /// Represents the SMTP login.
            pub smtp_login: String,
            /// Represents the SMTP password command.
            pub smtp_passwd_cmd: String,

            /// Represents the command used to encrypt a message.
            pub pgp_encrypt_cmd: Option<String>,
            /// Represents the command used to decrypt a message.
            pub pgp_decrypt_cmd: Option<String>,

	    $(pub $element: $ty),*
	}

	impl ToDeserializedBaseAccountConfig for $AccountConfig {
	    fn to_base(&self) -> DeserializedBaseAccountConfig {
		DeserializedBaseAccountConfig {
            	    name: self.name.clone(),
            	    downloads_dir: self.downloads_dir.clone(),
            	    signature: self.signature.clone(),
            	    signature_delimiter: self.signature_delimiter.clone(),
            	    default_page_size: self.default_page_size.clone(),
            	    inbox_folder: self.inbox_folder.clone(),
            	    sent_folder: self.sent_folder.clone(),
            	    draft_folder: self.draft_folder.clone(),
            	    notify_cmd: self.notify_cmd.clone(),
            	    notify_query: self.notify_query.clone(),
            	    watch_cmds: self.watch_cmds.clone(),

            	    default: self.default.clone(),
            	    email: self.email.clone(),

            	    smtp_host: self.smtp_host.clone(),
            	    smtp_port: self.smtp_port.clone(),
            	    smtp_starttls: self.smtp_starttls.clone(),
            	    smtp_insecure: self.smtp_insecure.clone(),
            	    smtp_login: self.smtp_login.clone(),
            	    smtp_passwd_cmd: self.smtp_passwd_cmd.clone(),

            	    pgp_encrypt_cmd: self.pgp_encrypt_cmd.clone(),
            	    pgp_decrypt_cmd: self.pgp_decrypt_cmd.clone(),
		}
	    }
	}
    }
}

make_account_config!(DeserializedBaseAccountConfig,);

make_account_config!(
    DeserializedImapAccountConfig,
    imap_host: String,
    imap_port: u16,
    imap_starttls: Option<bool>,
    imap_insecure: Option<bool>,
    imap_login: String,
    imap_passwd_cmd: String
);

make_account_config!(DeserializedMaildirAccountConfig, maildir_dir: PathBuf);
