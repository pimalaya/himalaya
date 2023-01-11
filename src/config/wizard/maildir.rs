use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedMaildirAccountConfig,
};
use anyhow::Result;
use himalaya_lib::MaildirConfig;

#[cfg(feature = "maildir-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    let backend = MaildirConfig::default();
    // TODO

    Ok(DeserializedAccountConfig::Maildir(
        DeserializedMaildirAccountConfig { base, backend },
    ))
}
