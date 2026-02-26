use imap_codec::{
    CommandCodec, GreetingCodec, ResponseCodec,
    decode::Decoder,
    encode::Encoder,
    imap_types::{
        auth::AuthMechanism,
        body::{BasicFields, Body, BodyStructure, SpecificFields},
        command::{Command, CommandBody},
        core::{AString, IString, Literal, NString, Quoted, Tag},
        datetime::DateTime,
        envelope::{Address, Envelope},
        fetch::{Macro, MessageDataItem, MessageDataItemName, Section},
        flag::{Flag, FlagFetch, FlagPerm, StoreResponse, StoreType},
        response::{Capability, Code, Data, Response, Status},
        secret::Secret,
    },
};

enum Who {
    Client,
    Server,
}

enum Message<'a> {
    Command(Command<'a>),
    Response(Response<'a>),
}

struct TraceLines<'a> {
    trace: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for TraceLines<'a> {
    type Item = (Who, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let input = &self.trace[self.offset..];

        if let Some(pos) = input.iter().position(|b| *b == b'\n') {
            let who = match &input[..3] {
                b"C: " => Who::Client,
                b"S: " => Who::Server,
                _ => panic!("Line must begin with \"C: \" or \"S: \"."),
            };

            self.offset += pos + 1;

            Some((who, &input[3..pos + 1]))
        } else {
            None
        }
    }
}

fn split_trace(trace: &[u8]) -> impl Iterator<Item = (Who, &[u8])> {
    TraceLines { trace, offset: 0 }
}

fn test_lines_of_trace(trace: &[u8]) {
    for (who, line) in split_trace(trace) {
        // Replace last "\n" with "\r\n".
        let line = {
            let mut line = line[..line.len().saturating_sub(1)].to_vec();
            line.extend_from_slice(b"\r\n");
            line
        };

        match who {
            Who::Client => {
                println!("C:          {}", String::from_utf8_lossy(&line).trim());
                let (rem, parsed) = CommandCodec::default().decode(&line).unwrap();
                assert!(rem.is_empty());
                println!("Parsed      {parsed:?}");
                let serialized = CommandCodec::default().encode(&parsed).dump();
                println!(
                    "Serialized: {}",
                    String::from_utf8_lossy(&serialized).trim()
                );
                let (rem, parsed2) = CommandCodec::default().decode(&serialized).unwrap();
                assert!(rem.is_empty());
                assert_eq!(parsed, parsed2);
                println!()
            }
            Who::Server => {
                println!("S:          {}", String::from_utf8_lossy(&line).trim());
                let (rem, parsed) = ResponseCodec::default().decode(&line).unwrap();
                println!("Parsed:     {parsed:?}");
                assert!(rem.is_empty());
                let serialized = ResponseCodec::default().encode(&parsed).dump();
                println!(
                    "Serialized: {}",
                    String::from_utf8_lossy(&serialized).trim()
                );
                let (rem, parsed2) = ResponseCodec::default().decode(&serialized).unwrap();
                assert!(rem.is_empty());
                assert_eq!(parsed, parsed2);
                println!()
            }
        }
    }
}

fn test_trace_known_positive(tests: Vec<(&[u8], Message)>) {
    for (test, expected) in tests.into_iter() {
        println!("// {}", std::str::from_utf8(test).unwrap().trim());
        match expected {
            Message::Command(expected) => {
                let (rem, got) = CommandCodec::default().decode(test).unwrap();
                assert!(rem.is_empty());
                assert_eq!(expected, got);
                println!("{got:?}");
                let encoded = CommandCodec::default().encode(&got).dump();
                println!("// {}", String::from_utf8(encoded.clone()).unwrap().trim());
                let (rem2, got2) = CommandCodec::default().decode(&encoded).unwrap();
                assert!(rem2.is_empty());
                assert_eq!(expected, got2);
            }
            Message::Response(expected) => {
                let (rem, got) = ResponseCodec::default().decode(test).unwrap();
                assert!(rem.is_empty());
                assert_eq!(expected, got);
                println!("{got:?}");
                let encoded = ResponseCodec::default().encode(&got).dump();
                println!("// {}", String::from_utf8(encoded.clone()).unwrap().trim());
                let (rem2, got2) = ResponseCodec::default().decode(&encoded).unwrap();
                assert!(rem2.is_empty());
                assert_eq!(expected, got2);
            }
        };

        println!();
    }
}

#[test]
fn test_from_capability() {
    let tests = {
        vec![
            (
                b"abcd CAPABILITY\r\n".as_ref(),
                Message::Command(Command::new("abcd", CommandBody::Capability).unwrap()),
            ),
            (
                b"* CAPABILITY IMAP4rev1 STARTTLS AUTH=GSSAPI LOGINDISABLED\r\n",
                Message::Response(Response::Data(
                    // FIXME(API): accept &[...]
                    Data::capability(vec![
                        Capability::Imap4Rev1,
                        #[cfg(feature = "starttls")]
                        Capability::StartTls,
                        #[cfg(not(feature = "starttls"))]
                        Capability::try_from("STARTTLS").unwrap(),
                        Capability::Auth(AuthMechanism::try_from("GSSAPI").unwrap()),
                        Capability::LoginDisabled,
                    ])
                    .unwrap(),
                )),
            ),
            (
                b"abcd OK CAPABILITY completed\r\n",
                // FIXME(API): Option<Tag> no TryInto ...
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("abcd").unwrap()),
                        None,
                        "CAPABILITY completed",
                    )
                    .unwrap(),
                )),
            ),
            #[cfg(feature = "starttls")]
            (
                b"efgh STARTTLS\r\n",
                Message::Command(Command::new("efgh", CommandBody::StartTLS).unwrap()),
            ),
            (
                b"efgh OK STARTLS completed\r\n",
                // FIXME(API): Option<Tag> no TryInto ...
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("efgh").unwrap()),
                        None,
                        "STARTLS completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"ijkl CAPABILITY\r\n",
                Message::Command(Command::new("ijkl", CommandBody::Capability).unwrap()),
            ),
            (
                b"* CAPABILITY IMAP4rev1 AUTH=GSSAPI AUTH=PLAIN\r\n",
                Message::Response(Response::Data(
                    Data::capability(vec![
                        Capability::Imap4Rev1,
                        Capability::Auth(AuthMechanism::try_from("GSSAPI").unwrap()),
                        Capability::Auth(AuthMechanism::Plain),
                    ])
                    .unwrap(),
                )),
            ),
            (
                b"ijkl OK CAPABILITY completed\r\n",
                // FIXME(API): Option<Tag> no TryInto ...
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("ijkl").unwrap()),
                        None,
                        "CAPABILITY completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_from_noop() {
    let tests = {
        vec![
            (
                b"a002 NOOP\r\n".as_ref(),
                Message::Command(Command::new("a002", CommandBody::Noop).unwrap()),
            ),
            (
                b"a002 OK NOOP completed\r\n",
                // FIXME(API)
                Message::Response(Response::Status(
                    Status::ok(Some(Tag::try_from("a002").unwrap()), None, "NOOP completed")
                        .unwrap(),
                )),
            ),
            (
                b"a047 NOOP\r\n",
                Message::Command(Command::new("a047", CommandBody::Noop).unwrap()),
            ),
            (
                b"* 22 EXPUNGE\r\n",
                Message::Response(Response::Data(Data::expunge(22).unwrap())),
            ),
            (
                b"* 23 EXISTS\r\n",
                Message::Response(Response::Data(Data::Exists(23))),
            ),
            (
                b"* 3 RECENT\r\n",
                Message::Response(Response::Data(Data::Recent(3))),
            ),
            (
                b"* 14 FETCH (FLAGS (\\Seen \\Deleted))\r\n",
                // FIXME(API)
                Message::Response(Response::Data(
                    Data::fetch(
                        14,
                        vec![MessageDataItem::Flags(vec![
                            FlagFetch::Flag(Flag::Seen),
                            FlagFetch::Flag(Flag::Deleted),
                        ])],
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a047 OK NOOP completed\r\n",
                // FIXME(API)
                Message::Response(Response::Status(
                    Status::ok(Some(Tag::try_from("a047").unwrap()), None, "NOOP completed")
                        .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_from_logout() {
    let tests = {
        vec![
            (
                b"A023 LOGOUT\r\n".as_ref(),
                Message::Command(Command::new("A023", CommandBody::Logout).unwrap()),
            ),
            (
                b"* BYE IMAP4rev1 Server logging out\r\n",
                Message::Response(Response::Status(
                    Status::bye(None, "IMAP4rev1 Server logging out").unwrap(),
                )),
            ),
            (
                b"A023 OK LOGOUT completed\r\n",
                // FIXME(API)
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("A023").unwrap()),
                        None,
                        "LOGOUT completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[cfg(feature = "starttls")]
#[test]
fn test_from_starttls() {
    let trace = br#"C: a001 CAPABILITY
S: * CAPABILITY IMAP4rev1 STARTTLS LOGINDISABLED
S: a001 OK CAPABILITY completed
C: a002 STARTTLS
S: a002 OK Begin TLS negotiation now
C: a003 CAPABILITY
S: * CAPABILITY IMAP4rev1 AUTH=PLAIN
S: a003 OK CAPABILITY completed
C: a004 LOGIN joe password
S: a004 OK LOGIN completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_authenticate() {
    // S: * OK IMAP4rev1 Server
    // C: A001 AUTHENTICATE GSSAPI
    // S: +
    // C: YIIB+wYJKoZIhvcSAQICAQBuggHqMIIB5qADAgEFoQMCAQ6iBw
    //    MFACAAAACjggEmYYIBIjCCAR6gAwIBBaESGxB1Lndhc2hpbmd0
    //    b24uZWR1oi0wK6ADAgEDoSQwIhsEaW1hcBsac2hpdmFtcy5jYW
    //    Mud2FzaGluZ3Rvbi5lZHWjgdMwgdCgAwIBAaEDAgEDooHDBIHA
    //    cS1GSa5b+fXnPZNmXB9SjL8Ollj2SKyb+3S0iXMljen/jNkpJX
    //    AleKTz6BQPzj8duz8EtoOuNfKgweViyn/9B9bccy1uuAE2HI0y
    //    C/PHXNNU9ZrBziJ8Lm0tTNc98kUpjXnHZhsMcz5Mx2GR6dGknb
    //    I0iaGcRerMUsWOuBmKKKRmVMMdR9T3EZdpqsBd7jZCNMWotjhi
    //    vd5zovQlFqQ2Wjc2+y46vKP/iXxWIuQJuDiisyXF0Y8+5GTpAL
    //    pHDc1/pIGmMIGjoAMCAQGigZsEgZg2on5mSuxoDHEA1w9bcW9n
    //    FdFxDKpdrQhVGVRDIzcCMCTzvUboqb5KjY1NJKJsfjRQiBYBdE
    //    NKfzK+g5DlV8nrw81uOcP8NOQCLR5XkoMHC0Dr/80ziQzbNqhx
    //    O6652Npft0LQwJvenwDI13YxpwOdMXzkWZN/XrEqOWp6GCgXTB
    //    vCyLWLlWnbaUkZdEYbKHBPjd8t/1x5Yg==
    // S: + YGgGCSqGSIb3EgECAgIAb1kwV6ADAgEFoQMCAQ+iSzBJoAMC
    //    AQGiQgRAtHTEuOP2BXb9sBYFR4SJlDZxmg39IxmRBOhXRKdDA0
    //    uHTCOT9Bq3OsUTXUlk0CsFLoa8j+gvGDlgHuqzWHPSQg==
    // C:
    // S: + YDMGCSqGSIb3EgECAgIBAAD/////6jcyG4GE3KkTzBeBiVHe
    //    ceP2CWY0SR0fAQAgAAQEBAQ=
    // C: YDMGCSqGSIb3EgECAgIBAAD/////3LQBHXTpFfZgrejpLlLImP
    //    wkhbfa2QteAQAgAG1yYwE=
    // S: A001 OK GSSAPI authentication successful
}

#[test]
fn test_from_login() {
    let tests = {
        vec![
            (
                b"a001 LOGIN SMITH SESAME\r\n".as_ref(),
                // We know that `CommandBody::login()` will create two atoms.
                Message::Command(
                    Command::new("a001", CommandBody::login("SMITH", "SESAME").unwrap()).unwrap(),
                ),
            ),
            (
                // Addition: We change the previous command here to test a quoted string ...
                b"a001 LOGIN \"SMITH\" SESAME\r\n".as_ref(),
                Message::Command(
                    Command::new(
                        "a001",
                        // ... and construct the command manually ...
                        CommandBody::Login {
                            // ... using a quoted string ...
                            username: AString::String(IString::Quoted(
                                Quoted::try_from("SMITH").unwrap(),
                            )),
                            // ... and an atom (knowing that `AString::try_from(...)` will create it.
                            password: Secret::new(AString::try_from("SESAME").unwrap()),
                        },
                    )
                    .unwrap(),
                ),
            ),
            (
                b"a001 OK LOGIN completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a001").unwrap()),
                        None,
                        "LOGIN completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_from_select() {
    let tests = {
        vec![
            (
                b"A142 SELECT INBOX\r\n".as_ref(),
                Message::Command(
                    Command::new("A142", CommandBody::select("inbox").unwrap()).unwrap(),
                ),
            ),
            (
                b"* 172 EXISTS\r\n",
                Message::Response(Response::Data(Data::Exists(172))),
            ),
            (
                b"* 1 RECENT\r\n",
                Message::Response(Response::Data(Data::Recent(1))),
            ),
            (
                b"* OK [UNSEEN 12] Message 12 is first unseen\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::unseen(12).unwrap()),
                        "Message 12 is first unseen",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* OK [UIDVALIDITY 3857529045] UIDs valid\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::uidvalidity(3857529045).unwrap()),
                        "UIDs valid",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* OK [UIDNEXT 4392] Predicted next UID\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::uidnext(4392).unwrap()),
                        "Predicted next UID",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n",
                Message::Response(Response::Data(Data::Flags(vec![
                    Flag::Answered,
                    Flag::Flagged,
                    Flag::Deleted,
                    Flag::Seen,
                    Flag::Draft,
                ]))),
            ),
            (
                b"* OK [PERMANENTFLAGS (\\Deleted \\Seen \\*)] Limited\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::PermanentFlags(vec![
                            FlagPerm::Flag(Flag::Deleted),
                            FlagPerm::Flag(Flag::Seen),
                            FlagPerm::Asterisk,
                        ])),
                        "Limited",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"A142 OK [READ-WRITE] SELECT completed\r\n",
                // FIXME(API)
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("A142").unwrap()),
                        Some(Code::ReadWrite),
                        "SELECT completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_from_examine() {
    let tests = {
        vec![
            (
                b"A932 EXAMINE blurdybloop\r\n".as_ref(),
                Message::Command(
                    Command::new("A932", CommandBody::examine("blurdybloop").unwrap()).unwrap(),
                ),
            ),
            (
                b"* 17 EXISTS\r\n",
                Message::Response(Response::Data(Data::Exists(17))),
            ),
            (
                b"* 2 RECENT\r\n",
                Message::Response(Response::Data(Data::Recent(2))),
            ),
            (
                b"* OK [UNSEEN 8] Message 8 is first unseen\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::unseen(8).unwrap()),
                        "Message 8 is first unseen",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* OK [UIDVALIDITY 3857529045] UIDs valid\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::uidvalidity(3857529045).unwrap()),
                        "UIDs valid",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* OK [UIDNEXT 4392] Predicted next UID\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::uidnext(4392).unwrap()),
                        "Predicted next UID",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n",
                Message::Response(Response::Data(Data::Flags(vec![
                    Flag::Answered,
                    Flag::Flagged,
                    Flag::Deleted,
                    Flag::Seen,
                    Flag::Draft,
                ]))),
            ),
            (
                b"* OK [PERMANENTFLAGS ()] No permanent flags permitted\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::PermanentFlags(vec![])),
                        "No permanent flags permitted",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"A932 OK [READ-ONLY] EXAMINE completed\r\n",
                // FIXME(API)
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("A932").unwrap()),
                        Some(Code::ReadOnly),
                        "EXAMINE completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_from_create() {
    let trace = br#"C: A003 CREATE owatagusiam/
S: A003 OK CREATE completed
C: A004 CREATE owatagusiam/blurdybloop
S: A004 OK CREATE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_delete() {
    let trace = br#"C: A682 LIST "" *
S: * LIST () "/" blurdybloop
S: * LIST (\Noselect) "/" foo
S: * LIST () "/" foo/bar
S: A682 OK LIST completed
C: A683 DELETE blurdybloop
S: A683 OK DELETE completed
C: A684 DELETE foo
S: A684 NO Name "foo" has inferior hierarchical names
C: A685 DELETE foo/bar
S: A685 OK DELETE Completed
C: A686 LIST "" *
S: * LIST (\Noselect) "/" foo
S: A686 OK LIST completed
C: A687 DELETE foo
S: A687 OK DELETE Completed
C: A82 LIST "" *
S: * LIST () "." blurdybloop
S: * LIST () "." foo
S: * LIST () "." foo.bar
S: A82 OK LIST completed
C: A83 DELETE blurdybloop
S: A83 OK DELETE completed
C: A84 DELETE foo
S: A84 OK DELETE Completed
C: A85 LIST "" *
S: * LIST () "." foo.bar
S: A85 OK LIST completed
C: A86 LIST "" %
S: * LIST (\Noselect) "." foo
S: A86 OK LIST completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_rename() {
    let trace = br#"C: A682 LIST "" *
S: * LIST () "/" blurdybloop
S: * LIST (\Noselect) "/" foo
S: * LIST () "/" foo/bar
S: A682 OK LIST completed
C: A683 RENAME blurdybloop sarasoop
S: A683 OK RENAME completed
C: A684 RENAME foo zowie
S: A684 OK RENAME Completed
C: A685 LIST "" *
S: * LIST () "/" sarasoop
S: * LIST (\Noselect) "/" zowie
S: * LIST () "/" zowie/bar
S: A685 OK LIST completed
C: Z432 LIST "" *
S: * LIST () "." INBOX
S: * LIST () "." INBOX.bar
S: Z432 OK LIST completed
C: Z433 RENAME INBOX old-mail
S: Z433 OK RENAME completed
C: Z434 LIST "" *
S: * LIST () "." INBOX
S: * LIST () "." INBOX.bar
S: * LIST () "." old-mail
S: Z434 OK LIST completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_subscribe() {
    let trace = br#"C: A002 SUBSCRIBE #news.comp.mail.mime
S: A002 OK SUBSCRIBE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_unsubscribe() {
    let trace = br#"C: A002 UNSUBSCRIBE #news.comp.mail.mime
S: A002 OK UNSUBSCRIBE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_list() {
    let trace = br#"C: A101 LIST "" ""
S: * LIST (\Noselect) "/" ""
S: A101 OK LIST Completed
C: A102 LIST #news.comp.mail.misc ""
S: * LIST (\Noselect) "." #news.
S: A102 OK LIST Completed
C: A103 LIST /usr/staff/jones ""
S: * LIST (\Noselect) "/" /
S: A103 OK LIST Completed
C: A202 LIST ~/Mail/ %
S: * LIST (\Noselect) "/" ~/Mail/foo
S: * LIST () "/" ~/Mail/meetings
S: A202 OK LIST completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_lsub() {
    let trace = br#"C: A002 LSUB "news." "comp.mail.*"
S: * LSUB () "." #news.comp.mail.mime
S: * LSUB () "." #news.comp.mail.misc
S: A002 OK LSUB completed
C: A003 LSUB "news." "comp.%"
S: * LSUB (\NoSelect) "." #news.comp.mail
S: A003 OK LSUB completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_status() {
    let trace = br#"C: A042 STATUS blurdybloop (UIDNEXT MESSAGES)
S: * STATUS blurdybloop (MESSAGES 231 UIDNEXT 44292)
S: A042 OK STATUS completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_append() {
    // C: A003 APPEND saved-messages (\Seen) {310}
    // S: + Ready for literal data
    // C: Date: Mon, 7 Feb 1994 21:52:25 -0800 (PST)
    // C: From: Fred Foobar <foobar@Blurdybloop.COM>
    // C: Subject: afternoon meeting
    // C: To: mooch@owatagu.siam.edu
    // C: Message-Id: <B27397-0100000@Blurdybloop.COM>
    // C: MIME-Version: 1.0
    // C: Content-Type: TEXT/PLAIN; CHARSET=US-ASCII
    // C:
    // C: Hello Joe, do you think we can meet at 3:30 tomorrow?
    // C:
    // S: A003 OK APPEND completed
}

#[test]
fn test_from_check() {
    let trace = br#"C: FXXZ CHECK
S: FXXZ OK CHECK Completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_close() {
    let trace = br#"C: A341 CLOSE
S: A341 OK CLOSE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_expunge() {
    let trace = br#"C: A202 EXPUNGE
S: * 3 EXPUNGE
S: * 3 EXPUNGE
S: * 5 EXPUNGE
S: * 8 EXPUNGE
S: A202 OK EXPUNGE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_search() {
    // C: A284 SEARCH CHARSET UTF-8 TEXT {6}
    // C: XXXXXX
    let trace = br#"C: A282 SEARCH FLAGGED SINCE 1-Feb-1994 NOT FROM "Smith"
S: * SEARCH 2 84 882
S: A282 OK SEARCH completed
C: A283 SEARCH TEXT "string not in mailbox"
S: * SEARCH
S: A283 OK SEARCH completed
S: * SEARCH 43
S: A284 OK SEARCH completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_fetch() {
    // S: * 2 FETCH ....
    // S: * 3 FETCH ....
    // S: * 4 FETCH ....
    let trace = br#"C: A654 FETCH 2:4 (FLAGS BODY[HEADER.FIELDS (DATE FROM)])
S: A654 OK FETCH completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_store() {
    let trace = br#"C: A003 STORE 2:4 +FLAGS (\Deleted)
S: * 2 FETCH (FLAGS (\Deleted \Seen))
S: * 3 FETCH (FLAGS (\Deleted))
S: * 4 FETCH (FLAGS (\Deleted \Flagged \Seen))
S: A003 OK STORE completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_copy() {
    let trace = br#"C: A003 COPY 2:4 MEETING
S: A003 OK COPY completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_from_uid() {
    let trace = br#"C: A999 UID FETCH 4827313:4828442 FLAGS
S: * 23 FETCH (FLAGS (\Seen) UID 4827313)
S: * 24 FETCH (FLAGS (\Seen) UID 4827943)
S: * 25 FETCH (FLAGS (\Seen) UID 4828442)
S: A999 OK UID FETCH completed
"#;

    test_lines_of_trace(trace);
}

//#[test]
//fn test_from_X() {
//    let trace = br#"C: a441 CAPABILITY
//S: * CAPABILITY IMAP4rev1 XPIG-LATIN
//S: a441 OK CAPABILITY completed
//C: A442 XPIG-LATIN
//S: * XPIG-LATIN ow-nay eaking-spay ig-pay atin-lay
//S: A442 OK XPIG-LATIN ompleted-cay"#;
//
//    test_lines_of_trace(trace);
//}

#[test]
fn test_transcript_from_rfc() {
    let tests = {
        vec![
            (
                b"* OK IMAP4rev1 Service Ready\r\n".as_ref(),
                Message::Response(Response::Status(
                    Status::ok(None, None, "IMAP4rev1 Service Ready").unwrap(),
                )),
            ),
            (
                b"a001 login mrc secret\r\n",
                Message::Command(
                    Command::new("a001", CommandBody::login("mrc", "secret").unwrap()).unwrap(),
                ),
            ),
            (
                b"a001 OK LOGIN completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a001").unwrap()),
                        None,
                        "LOGIN completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a002 select inbox\r\n",
                Message::Command(
                    Command::new("a002", CommandBody::select("inbox").unwrap()).unwrap(),
                ),
            ),
            (
                b"* 18 EXISTS\r\n",
                Message::Response(Response::Data(Data::Exists(18))),
            ),
            (
                b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n",
                Message::Response(Response::Data(Data::Flags(vec![
                    Flag::Answered,
                    Flag::Flagged,
                    Flag::Deleted,
                    Flag::Seen,
                    Flag::Draft,
                ]))),
            ),
            (
                b"* 2 RECENT\r\n",
                Message::Response(Response::Data(Data::Recent(2))),
            ),
            (
                b"* OK [UNSEEN 17] Message 17 is the first unseen message\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::unseen(17).unwrap()),
                        "Message 17 is the first unseen message",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"* OK [UIDVALIDITY 3857529045] UIDs valid\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        None,
                        Some(Code::uidvalidity(3857529045).unwrap()),
                        "UIDs valid",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a002 OK [READ-WRITE] SELECT completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a002").unwrap()),
                        Some(Code::ReadWrite),
                        "SELECT completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a003 fetch 12 full\r\n",
                Message::Command(
                    Command::new(
                        "a003",
                        CommandBody::fetch("12", Macro::Full, false).unwrap(),
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* 12 FETCH (FLAGS (\\Seen) INTERNALDATE \"17-Jul-1996 02:44:25 -0700\" RFC822.SIZE 4286 ENVELOPE (\"Wed, 17 Jul 1996 02:23:25 -0700 (PDT)\" \"IMAP4rev1 WG mtg summary and minutes\" ((\"Terry Gray\" NIL \"gray\" \"cac.washington.edu\")) ((\"Terry Gray\" NIL \"gray\" \"cac.washington.edu\")) ((\"Terry Gray\" NIL \"gray\" \"cac.washington.edu\")) ((NIL NIL \"imap\" \"cac.washington.edu\")) ((NIL NIL \"minutes\" \"CNRI.Reston.VA.US\")(\"John Klensin\" NIL \"KLENSIN\" \"MIT.EDU\")) NIL NIL \"<B27397-0100000@cac.washington.edu>\") BODY (\"TEXT\" \"PLAIN\" (\"CHARSET\" \"US-ASCII\") NIL NIL \"7BIT\" 3028 92))\r\n",
                Message::Response(Response::Data(
                    Data::fetch(
                        12,
                        vec![
                            MessageDataItem::Flags(vec![FlagFetch::Flag(Flag::Seen)]),
                            MessageDataItem::InternalDate(DateTime::try_from(
                                chrono::DateTime::parse_from_rfc3339("1996-07-17T02:44:25-07:00")
                                    .unwrap(),
                            ).unwrap()),
                            MessageDataItem::Rfc822Size(4286),
                            MessageDataItem::Envelope(Envelope {
                                date: NString::from(
                                    Quoted::try_from("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)")
                                        .unwrap(),
                                ),
                                subject: NString::from(
                                    Quoted::try_from("IMAP4rev1 WG mtg summary and minutes")
                                        .unwrap(),
                                ),
                                from: vec![Address {
                                    name: NString::from(Quoted::try_from("Terry Gray").unwrap()),
                                    adl: NString(None),
                                    mailbox: NString::from(Quoted::try_from("gray").unwrap()),
                                    host: NString::from(
                                        Quoted::try_from("cac.washington.edu").unwrap(),
                                    ),
                                }],
                                sender: vec![Address {
                                    name: NString::from(Quoted::try_from("Terry Gray").unwrap()),
                                    adl: NString(None),
                                    mailbox: NString::from(Quoted::try_from("gray").unwrap()),
                                    host: NString::from(
                                        Quoted::try_from("cac.washington.edu").unwrap(),
                                    ),
                                }],
                                reply_to: vec![Address {
                                    name: NString::from(Quoted::try_from("Terry Gray").unwrap()),
                                    adl: NString(None),
                                    mailbox: NString::from(Quoted::try_from("gray").unwrap()),
                                    host: NString::from(
                                        Quoted::try_from("cac.washington.edu").unwrap(),
                                    ),
                                }],
                                to: vec![Address {
                                    name: NString(None),
                                    adl: NString(None),
                                    mailbox: NString::from(Quoted::try_from("imap").unwrap()),
                                    host: NString::from(
                                        Quoted::try_from("cac.washington.edu").unwrap(),
                                    ),
                                }],
                                cc: vec![
                                    Address {
                                        name: NString(None),
                                        adl: NString(None),
                                        mailbox: NString::from(
                                            Quoted::try_from("minutes").unwrap(),
                                        ),
                                        host: NString::from(
                                            Quoted::try_from("CNRI.Reston.VA.US").unwrap(),
                                        ),
                                    },
                                    Address {
                                        name: NString::from(
                                            Quoted::try_from("John Klensin").unwrap(),
                                        ),
                                        adl: NString(None),
                                        mailbox: NString::from(
                                            Quoted::try_from("KLENSIN").unwrap(),
                                        ),
                                        host: NString::from(Quoted::try_from("MIT.EDU").unwrap()),
                                    },
                                ],
                                bcc: vec![],
                                in_reply_to: NString(None),
                                message_id: NString::from(
                                    Quoted::try_from("<B27397-0100000@cac.washington.edu>")
                                        .unwrap(),
                                ),
                            }),
                            MessageDataItem::Body(BodyStructure::Single {
                                body: Body {
                                    basic: BasicFields {
                                        parameter_list: vec![(
                                            IString::from(Quoted::try_from("CHARSET").unwrap()),
                                            IString::from(Quoted::try_from("US-ASCII").unwrap()),
                                        )],
                                        id: NString(None),
                                        description: NString(None),
                                        content_transfer_encoding: IString::from(
                                            Quoted::try_from("7BIT").unwrap(),
                                        ),
                                        size: 3028,
                                    },
                                    specific: SpecificFields::Text {
                                        subtype: IString::from(Quoted::try_from("PLAIN").unwrap()),
                                        number_of_lines: 92,
                                    },
                                },
                                extension_data: None,
                            }),
                        ],
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a003 OK FETCH completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a003").unwrap()),
                        None,
                        "FETCH completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a004 fetch 12 body[header]\r\n",
                Message::Command(
                    Command::new(
                        "a004",
                        CommandBody::fetch(
                            "12",
                            vec![MessageDataItemName::BodyExt {
                                section: Some(Section::Header(None)),
                                peek: false,
                                partial: None,
                            }],
                            false,
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* 12 FETCH (BODY[HEADER] {342}\r
Date: Wed, 17 Jul 1996 02:23:25 -0700 (PDT)\r
From: Terry Gray <gray@cac.washington.edu>\r
Subject: IMAP4rev1 WG mtg summary and minutes\r
To: imap@cac.washington.edu\r
cc: minutes@CNRI.Reston.VA.US, John Klensin <KLENSIN@MIT.EDU>\r
Message-Id: <B27397-0100000@cac.washington.edu>\r
MIME-Version: 1.0\r
Content-Type: TEXT/PLAIN; CHARSET=US-ASCII\r
\r
)\r\n",
                Message::Response(Response::Data(
                    Data::fetch(
                        12,
                        vec![MessageDataItem::BodyExt {
                            section: Some(Section::Header(None)),
                            origin: None,
                            data: NString::from(
                                Literal::try_from(
                                    b"Date: Wed, 17 Jul 1996 02:23:25 -0700 (PDT)\r
From: Terry Gray <gray@cac.washington.edu>\r
Subject: IMAP4rev1 WG mtg summary and minutes\r
To: imap@cac.washington.edu\r
cc: minutes@CNRI.Reston.VA.US, John Klensin <KLENSIN@MIT.EDU>\r
Message-Id: <B27397-0100000@cac.washington.edu>\r
MIME-Version: 1.0\r
Content-Type: TEXT/PLAIN; CHARSET=US-ASCII\r
\r
"
                                    .as_ref(),
                                )
                                .unwrap(),
                            ),
                        }],
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a004 OK FETCH completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a004").unwrap()),
                        None,
                        "FETCH completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a005 store 12 +flags \\deleted\r\n",
                Message::Command(
                    Command::new(
                        "a005",
                        CommandBody::store(
                            "12",
                            StoreType::Add,
                            StoreResponse::Answer,
                            vec![Flag::Deleted],
                            false,
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                ),
            ),
            (
                b"* 12 FETCH (FLAGS (\\Seen \\Deleted))\r\n",
                Message::Response(Response::Data(
                    Data::fetch(
                        12,
                        vec![MessageDataItem::Flags(vec![
                            FlagFetch::Flag(Flag::Seen),
                            FlagFetch::Flag(Flag::Deleted),
                        ])],
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a005 OK +FLAGS completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a005").unwrap()),
                        None,
                        "+FLAGS completed",
                    )
                    .unwrap(),
                )),
            ),
            (
                b"a006 logout\r\n",
                Message::Command(Command::new("a006", CommandBody::Logout).unwrap()),
            ),
            (
                b"* BYE IMAP4rev1 server terminating connection\r\n",
                Message::Response(Response::Status(
                    Status::bye(None, "IMAP4rev1 server terminating connection").unwrap(),
                )),
            ),
            (
                b"a006 OK LOGOUT completed\r\n",
                Message::Response(Response::Status(
                    Status::ok(
                        Some(Tag::try_from("a006").unwrap()),
                        None,
                        "LOGOUT completed",
                    )
                    .unwrap(),
                )),
            ),
        ]
    };

    test_trace_known_positive(tests);
}

#[test]
fn test_transcript_from_rfc5161() {
    let trace = br#"C: t1 CAPABILITY
S: * CAPABILITY IMAP4rev1 ID LITERAL+ ENABLE X-GOOD-IDEA
S: t1 OK foo
C: t2 ENABLE CONDSTORE X-GOOD-IDEA
S: * ENABLED X-GOOD-IDEA
S: t2 OK foo
C: t3 CAPABILITY
S: * CAPABILITY IMAP4rev1 ID LITERAL+ ENABLE X-GOOD-IDEA
S: t3 OK foo again
C: a1 ENABLE CONDSTORE
S: * ENABLED CONDSTORE
S: a1 OK Conditional Store enabled
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_status_ok() {
    let trace = br#"S: * OK IMAP4rev1 server ready
C: A001 LOGIN fred blurdybloop
S: * OK [ALERT] System shutdown in 10 minutes
S: A001 OK LOGIN Completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_status_no() {
    let trace = br#"C: A222 COPY 1:2 owatagusiam
S: * NO Disk is 98% full, please delete unnecessary data
S: A222 OK COPY completed
C: A223 COPY 3:200 blurdybloop
S: * NO Disk is 98% full, please delete unnecessary data
S: * NO Disk is 99% full, please delete unnecessary data
S: A223 NO COPY failed: disk is full
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_status_bad() {
    let trace = br#"S: * BAD Command line too long
S: * BAD Empty command line
C: A443 EXPUNGE
S: * BAD Disk crash, attempting salvage to a new disk!
S: * OK Salvage successful, no data lost
S: A443 OK Expunge completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_status_preauth() {
    let line = b"* PREAUTH IMAP4rev1 server logged in as Smith\r\n";

    println!("S:          {}", String::from_utf8_lossy(line).trim());
    let (rem, parsed) = GreetingCodec::default().decode(line).unwrap();
    println!("Parsed:     {parsed:?}");
    assert!(rem.is_empty());
    let serialized = GreetingCodec::default().encode(&parsed).dump();
    println!(
        "Serialized: {}",
        String::from_utf8_lossy(&serialized).trim()
    );
    let (rem, parsed2) = GreetingCodec::default().decode(&serialized).unwrap();
    assert!(rem.is_empty());
    assert_eq!(parsed, parsed2);
    println!()
}

#[test]
fn test_response_status_bye() {
    let trace = br#"S: * BYE Autologout; idle for too long
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_capability() {
    let trace = br#"S: * CAPABILITY IMAP4rev1 STARTTLS AUTH=GSSAPI XPIG-LATIN
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_list() {
    let trace = br#"S: * LIST (\Noselect) "/" ~/Mail/foo
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_lsub() {
    let trace = br#"S: * LSUB () "." #news.comp.mail.misc
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_status() {
    let trace = br#"S: * STATUS blurdybloop (MESSAGES 231 UIDNEXT 44292)
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_search() {
    let trace = br#"S: * SEARCH 2 3 6
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_flags() {
    let trace = br#"S: * FLAGS (\Answered \Flagged \Deleted \Seen \Draft)
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_exists() {
    let trace = br#"S: * 23 EXISTS
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_recent() {
    let trace = br#"S: * 5 RECENT
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_expunge() {
    let trace = br#"S: * 44 EXPUNGE
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_fetch() {
    let trace = br#"S: * 23 FETCH (FLAGS (\Seen) RFC822.SIZE 44827)
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_response_data_continue() {
    // C: A001 LOGIN {11}
    // C: FRED FOOBAR {7}
    // C: fat man
    // C: A044 BLURDYBLOOP {102856}

    let trace = br#"S: + Ready for additional command text
S: A001 OK LOGIN completed
S: A044 BAD No such command as "BLURDYBLOOP"
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_trace_rfc2088() {
    let test = b"A001 LOGIN {11+}\r\nFRED FOOBAR {7+}\r\nfat man\r\n".as_ref();

    let (rem, got) = CommandCodec::default().decode(test).unwrap();
    assert!(rem.is_empty());
    assert_eq!(got, {
        let username = Literal::try_from("FRED FOOBAR").unwrap().into_non_sync();
        let password = Literal::try_from("fat man").unwrap().into_non_sync();

        Command::new(
            Tag::try_from("A001").unwrap(),
            CommandBody::login(username, password).unwrap(),
        )
        .unwrap()
    })
}

#[test]
fn test_trace_sort() {
    let trace = br#"C: A282 SORT (SUBJECT) UTF-8 SINCE 1-Feb-1994
S: * SORT 2 84 882
S: A282 OK SORT completed
C: A283 SORT (SUBJECT REVERSE DATE) UTF-8 ALL
S: * SORT 5 3 4 1 2
S: A283 OK SORT completed
C: A284 SORT (SUBJECT) US-ASCII TEXT "not in mailbox"
S: * SORT
S: A284 OK SORT completed
"#;

    test_lines_of_trace(trace);
}

#[test]
fn test_trace_thread() {
    let trace = br#"C: A283 THREAD ORDEREDSUBJECT UTF-8 SINCE 5-MAR-2000
S: * THREAD (166)(167)(168)(169)(172)(170)(171)(173)(174 (175)(176)(178)(181)(180))(179)(177 (183)(182)(188)(184)(185)(186)(187)(189))(190)(191)(192)(193)(194 195)(196 (197)(198))(199)(200 202)(201)(203)(204)(205)(206 207)(208)
S: A283 OK THREAD completed
C: A284 THREAD ORDEREDSUBJECT US-ASCII TEXT "gewp"
S: * THREAD
S: A284 OK THREAD completed
C: A285 THREAD REFERENCES UTF-8 SINCE 5-MAR-2000
S: * THREAD (166)(167)(168)(169)(172)((170)(179))(171)(173)((174)(175)(176)(178)(181)(180))((177)(183)(182)(188 (184)(189))(185 186)(187))(190)(191)(192)(193)((194)(195 196))(197 198)(199)(200 202)(201)(203)(204)(205 206 207)(208)
S: A285 OK THREAD completed
"#;

    test_lines_of_trace(trace);
}
