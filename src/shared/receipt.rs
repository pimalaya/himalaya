//! # Receipts
//!
//! What a write in a protocol namespace prints when it has no payload of
//! its own to show: the item it acted on, by id, or the items.
//!
//! The keys keep the meanings the machine output contract gives them:
//! `id` names the item as it is after the write, `ids` several items in
//! the order acted on. The sentence of the text form is not data, so it
//! stays out of the JSON.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

/// The receipt of a write acting on one item.
#[derive(Debug, Serialize, JsonSchema)]
pub struct IdReceipt {
    /// The id of the item acted on, as it is after the write.
    pub id: String,
    /// What the text form says was done, as in `Gmail label created`.
    #[serde(skip)]
    pub done: &'static str,
}

impl fmt::Display for IdReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.done, self.id)
    }
}

/// The receipt of a write acting on several items.
#[cfg(any(
    feature = "gmail",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "m2dir"
))]
#[derive(Debug, Serialize, JsonSchema)]
pub struct IdsReceipt {
    /// The ids of the items acted on, in the order acted on.
    pub ids: Vec<String>,
    /// What the text form says was done, as in `Gmail messages modified`.
    #[serde(skip)]
    pub done: &'static str,
}

#[cfg(any(
    feature = "gmail",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "m2dir"
))]
impl fmt::Display for IdsReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.done, self.ids.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::receipt::IdReceipt;

    #[test]
    fn the_sentence_stays_out_of_the_json() {
        let receipt = IdReceipt {
            id: "Label_1".into(),
            done: "Gmail label created",
        };

        assert_eq!(receipt.to_string(), "Gmail label created: Label_1\n");
        assert_eq!(
            serde_json::to_string(&receipt).unwrap(),
            r#"{"id":"Label_1"}"#
        );
    }
}
