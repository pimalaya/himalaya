#[cfg(feature = "notmuch-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    use crate::account::DeserializedNotmuchAccountConfig;
    use himalaya_lib::NotmuchConfig;

    let backend = NotmuchConfig::default();
    // TODO

    Ok(DeserializedAccountConfig::Notmuch(
        DeserializedNotmuchAccountConfig { base, backend },
    ))
}
