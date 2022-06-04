use anyhow::{Context, Result};
use himalaya_lib::msg::Envelopes;

use super::notmuch_envelope;

/// Represents a list of raw envelopees returned by the `notmuch`
/// crate.
pub type RawNotmuchEnvelopes = notmuch::Messages;

pub fn from_notmuch_msgs(msgs: RawNotmuchEnvelopes) -> Result<Envelopes> {
    let mut envelopes = Envelopes::default();
    for msg in msgs {
        let envelope =
            notmuch_envelope::from_notmuch_msg(msg).context("cannot parse notmuch message")?;
        envelopes.push(envelope);
    }
    Ok(envelopes)
}
