//! # Address
//!
//! An email address shared across all protocols.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single email address with an optional display name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Address {
    /// The display name, `Alice` in `Alice <alice@example.org>`.
    pub name: Option<String>,
    /// The address itself.
    pub email: String,
}
