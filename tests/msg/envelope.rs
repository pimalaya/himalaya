use himalaya::{
    config::model::Account,
    flag::model::Flags,
    msg::{
        attachment::Attachment,
        body::Body,
        envelope::Envelope,
        model::{Msg, Msgs},
    },
};

// ==========
// Tests
// ==========
#[test]
fn test() {
    let account = Account::new("inbox@localhost");

    // Try default envelope first.
    let msg = Msg::new(&account);
    let msg_to_compare = Msg::new_with_envelope(
        &account,
        Envelope {
            from: vec!["inbox@localhost".to_string()],
            .. Envelope::default()
        },
    );
    assert_eq!(
        // left
        msg_to_compare,
        // right
        msg
    );
}
