//! # Message configuration
//!
//! The `message` block, global or per account.

use clap::ValueEnum;
use serde::Deserialize;

/// Message options under `message.*`.
///
/// Unknown keys are accepted rather than denied, so the rest of a v1
/// `[message]` table (`read`, `write`, `delete`...) keeps loading.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageConfig {
    /// Sending options.
    #[serde(default)]
    pub send: MessageSendConfig,
    /// `message reply` options.
    #[serde(default)]
    pub reply: MessageQuoteConfig,
    /// `message forward` options.
    #[serde(default)]
    pub forward: MessageQuoteConfig,
}

/// Quoting options under `message.reply.*` and `message.forward.*`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageQuoteConfig {
    /// Posting style when `--posting-style` is not passed.
    pub posting_style: Option<PostingStyle>,
}

/// Where a quoted source body sits relative to the written one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
pub enum PostingStyle {
    /// The written body above the quoted source body.
    #[default]
    Top,
    /// The quoted source body above the written body.
    Bottom,
    /// The written body alone, the source left unquoted, for a writer
    /// who already laid the quote out.
    None,
}

/// Sending options under `message.send.*`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageSendConfig {
    /// Mailbox a sent message is copied to when `--save` is not passed.
    ///
    /// Leave it unset on Gmail and Microsoft Graph, which file the sent
    /// message themselves.
    pub save_copy: Option<SaveCopyConfig>,
}

/// The `message.send.save-copy` value.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SaveCopyConfig {
    /// `true` for the `sent` mailbox, `false` for no copy, the v1 form.
    Enabled(bool),
    /// A mailbox name, alias or role.
    Mailbox(String),
}
