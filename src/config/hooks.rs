use serde::Deserialize;

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Hooks {
    pub pre_send: Option<String>,
}
