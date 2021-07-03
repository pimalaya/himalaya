// use himalaya::msg::{
//     envelope::Envelope,
//     model::{Msg, Msgs}, 
//     body::Body,
//     attachment::Attachment
// };
// use himalaya::config::model::Account;
//
// fn get_account(addr: &str) -> Account {
//     Account {
//         name: None,
//         downloads_dir: None,
//         signature: None,
//         default_page_size: None,
//         default: Some(true),
//         email: addr.into(),
//         watch_cmds: None,
//         imap_host: String::from("localhost"),
//         imap_port: 3993,
//         imap_starttls: Some(false),
//         imap_insecure: Some(true),
//         imap_login: addr.into(),
//         imap_passwd_cmd: String::from("echo 'password'"),
//         smtp_host: String::from("localhost"),
//         smtp_port: 3465,
//         smtp_starttls: Some(false),
//         smtp_insecure: Some(true),
//         smtp_login: addr.into(),
//         smtp_passwd_cmd: String::from("echo 'password'"),
//     }
// }
//
// #[test]
// fn test_new_with_envelope() {
//     let account = get_account("inbox@localhost");
//
//     // Try default envelope first.
//     let msg = Msg::new(&account);
//     assert_eq!(
//         // left
//         Msg {
//             flags: Vec::new(),
//             envelope: Envelope {
//                 from: vec![" <inbox@localhost>"],
//                 .. Envelope::default()
//             },
//             body: Body::from(String::from("")),
//             uid: None,
//             date: None,
//         },
//         // right
//         msg
//     );
// }
