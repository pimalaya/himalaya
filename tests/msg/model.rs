use himalaya::config::model::Account;
use himalaya::msg::body::Body;
use himalaya::msg::envelope::Envelope;
use himalaya::msg::model::Msg;

#[test]
fn test_new() {
    // -- Accounts -
    let account1 = Account::new_with_signature(Some("Soywod"), "clement.douin@posteo.net", None);
    let account2 = Account::new_with_signature(None, "tornax07@gmail.com", None);

    // -- Creating message --
    let msg1 = Msg::new(&account1);
    let msg2 = Msg::new(&account2);

    // -- Expected outputs --
    let expected_envelope1 = Envelope {
        from: vec![String::from("Soywod <clement.douin@posteo.net>")],
        signature: Some(String::from("Account Signature")),
        ..Envelope::default()
    };

    let expected_envelope2 = Envelope {
        from: vec![String::from("tornax07@gmail.com")],
        signature: Some(String::from("Account Signature")),
        ..Envelope::default()
    };

    // -- Tests --
    assert_eq!(msg1.envelope, expected_envelope1,
            "Left: {:?}, Right: {:?}",
            &msg1.envelope, &expected_envelope1);
    assert_eq!(msg2.envelope, expected_envelope2,
            "Left: {:?}, Right: {:?}",
            &msg2.envelope, &expected_envelope2);

    assert!(msg1.get_raw().unwrap().is_empty());
    assert!(msg2.get_raw().unwrap().is_empty());
}

#[test]
fn test_new_with_envelope() {
    let account = Account::new(Some("Name"), "test@msg.asdf");
    let account_with_signature = Account::new_with_signature(Some("Name"), "test@msg.asdf", Some("lol"));

    // -- Test-Messages --
    let msg_with_custom_from = Msg::new_with_envelope(
        &account,
        Envelope {
            from: vec![String::from("Someone <Else@msg.asdf>")],
            ..Envelope::default()
        },
        );

    let msg_with_custom_signature = Msg::new_with_envelope(
        &account_with_signature,
        Envelope::default()
        );

    // -- Expectations --
    let expected_with_custom_from = Msg {
        envelope: Envelope {
            // the Msg::new_with_envelope function should use the from
            // address in the envelope struct, not the from address of the
            // account
            from: vec![String::from("Someone <Else@msg.asdf>")],
            ..Envelope::default()
        },
        // The signature should be added automatically
        body: Body::from(""),
        ..Msg::default()
    };

    let expected_with_custom_signature = Msg {
        envelope: Envelope {
            from: vec![String::from("Name <test@msg.asdf>")],
            signature: Some(String::from("lol")),
            ..Envelope::default()
        },
        body: Body::from("lol"),
        ..Msg::default()
    };

    // -- Testing --
    assert_eq!(msg_with_custom_from, expected_with_custom_from,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg_with_custom_from), dbg!(&expected_with_custom_from));
    assert_eq!(msg_with_custom_signature, expected_with_custom_signature,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg_with_custom_signature), dbg!(&expected_with_custom_signature));
}

#[test]
fn test_change_to_reply() {
    // in this test, we are gonna reproduce the same situation as shown
    // here: https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.2

    // == Preparations ==
    // -- rfc test --
    // accounts for the rfc test
    let john_doe = Account::new(Some("John Doe"), "jdoe@machine.example");
    let mary_smith = Account::new(Some("Mary Smith"), "mary@example.net");

    let msg_rfc_test = Msg {
        envelope: Envelope {
            from: vec!["John Doe <jdoe@machine.example>".to_string()],
            to: vec!["Mary Smith <mary@example.net>".to_string()],
            subject: Some("Saying Hello".to_string()),
            message_id: Some("<1234@local.machine.example>".to_string()),
            ..Envelope::default()
        },
        body: Body::from(concat![
                         "This is a message just to say hello.\n",
                         "So, \"Hello\".",
        ]),
        ..Msg::default()
    };

    // -- for general tests --
    let account = Account::new(Some("Name"), "some@address.asdf");

    // -- for reply_all --
    // a custom test to look what happens, if we want to reply to all addresses.
    // Take a look into the doc of the "change_to_reply" what should happen, if we
    // set "reply_all" to "true".
    let mut msg_reply_all = Msg {
        envelope: Envelope {
            from: vec!["Boss <someone@boss.asdf>".to_string()],
            to: vec![
                "msg@1.asdf".to_string(),
                "msg@2.asdf".to_string(),
                "Name <some@address.asdf>".to_string(),
            ],
            cc: Some(vec![
                     "test@testing".to_string(),
                     "test2@testing".to_string(),
            ]),
            message_id: Some("RandomID123".to_string()),
            reply_to: Some(vec!["Reply@Mail.rofl".to_string()]),
            subject: Some("Have you heard of himalaya?".to_string()),
            ..Envelope::default()
        },
        body: Body::from(concat!["A body test\n", "\n", "Sincerely",]),
        ..Msg::default()
    };

    // == Expected output(s) ==
    // -- rfc test --
    // the first step
    let expected_rfc1 = Msg {
        envelope: Envelope {
            from: vec!["Mary Smith <mary@example.net>".to_string()],
            to: vec!["John Doe <jdoe@machine.example>".to_string()],
            reply_to: Some(vec![
                           "\"Mary Smith: Personal Account\" <smith@home.example>".to_string(),
            ]),
            subject: Some("Re: Saying Hello".to_string()),
            message_id: Some("<3456@example.net>".to_string()),
            in_reply_to: Some("<1234@local.machine.example>".to_string()),
            ..Envelope::default()
        },
        body: Body::from(concat![
                         "> This is a message just to say hello.\n",
                         "> So, \"Hello\".",
        ]),
        ..Msg::default()
    };

    // then the response the the first respone above
    let expected_rfc2 = Msg {
        envelope: Envelope {
            to: vec!["\"Mary Smith: Personal Account\" <smith@home.example>".to_string()],
            from: vec!["John Doe <jdoe@machine.example>".to_string()],
            subject: Some("Re: Saying Hello".to_string()),
            message_id: Some("<abcd.1234@local.machine.test>".to_string()),
            in_reply_to: Some("<3456@example.net>".to_string()),
            ..Envelope::default()
        },
        body: Body::from(concat![
                         "> > This is a message just to say hello.\n",
                         "> > So, \"Hello\".",
        ]),
        ..Msg::default()
    };

    // -- reply all --
    let expected_reply_all = Msg {
        envelope: Envelope {
            from: vec!["Name <some@address.asdf>".to_string()],
            to: vec![
                "msg@1.asdf".to_string(),
                "msg@2.asdf".to_string(),
                "Reply@Mail.rofl".to_string(),
            ],
            cc: Some(vec![
                     "test@testing".to_string(),
                     "test2@testing".to_string(),
            ]),
            in_reply_to: Some("RandomID123".to_string()),
            subject: Some("Re: Have you heard of himalaya?".to_string()),
            ..Envelope::default()
        },
        body: Body::from(concat![
                         "> A body test\n",
                         "> \n",
                         "> Sincerely"
        ]),
        ..Msg::default()
    };

    // == Testing ==
    // -- rfc test --
    // represents the message for the first reply
    let mut rfc_reply_1 = msg_rfc_test.clone();
    rfc_reply_1.change_to_reply(&mary_smith, false).unwrap();

    // the user would enter this normally
    rfc_reply_1.envelope = Envelope {
        message_id: Some("<3456@example.net>".to_string()),
        reply_to: Some(vec![
                       "\"Mary Smith: Personal Account\" <smith@home.example>".to_string(),
        ]),
        ..rfc_reply_1.envelope.clone()
    };

    // represents the message for the reply to the reply
    let mut rfc_reply_2 = rfc_reply_1.clone();
    rfc_reply_2.change_to_reply(&john_doe, false).unwrap();
    rfc_reply_2.envelope = Envelope {
        message_id: Some("<abcd.1234@local.machine.test>".to_string()),
        ..rfc_reply_2.envelope.clone()
    };

    assert_eq!(rfc_reply_1, expected_rfc1,
               "Left: {:?}, Right: {:?}",
               dbg!(&rfc_reply_1), dbg!(&expected_rfc1));

    assert_eq!(rfc_reply_2, expected_rfc2,
               "Left: {:?}, Right: {:?}",
               dbg!(&rfc_reply_2), dbg!(&expected_rfc2));

    // -- custom tests -—
    msg_reply_all.change_to_reply(&account, true).unwrap();
    assert_eq!(msg_reply_all, expected_reply_all,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg_reply_all), dbg!(&expected_reply_all));
}

#[test]
fn test_change_to_forwarding() {
    // == Preparations ==
    let account = Account::new_with_signature(Some("Name"), "some@address.asdf", Some("lol"));
    let mut msg = Msg::new_with_envelope(
        &account,
        Envelope {
            from: vec![String::from("ThirdPerson <some@msg.asdf>")],
            subject: Some(String::from("Test subject")),
            ..Envelope::default()
        },
        );

    msg.body = Body::from(concat!["The body text, nice!\n", "Himalaya is nice!",]);

    // == Expected Results ==
    let expected_msg = Msg {
        envelope: Envelope {
            from: vec![String::from("ThirdPerson <some@msg.asdf>")],
            sender: Some(String::from("Name <some@address.asdf>")),
            signature: Some(String::from("lol")),
            subject: Some(String::from("Fwd: Test subject")),
            ..Envelope::default()
        },
        body: Body::from(concat![
                         "lol\n",
                         "\n",
                         "---------- Forwarded Message ----------\n",
                         "The body text, nice!\n",
                         "Himalaya is nice!\n",
        ]),
        ..Msg::default()
    };

    // == Tests ==
    msg.change_to_forwarding(&account);
    assert_eq!(msg, expected_msg,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg), dbg!(&expected_msg));
}

#[test]
fn test_edit_body() {
    // == Preparations ==
    let account = Account::new_with_signature(Some("Name"), "some@address.asdf", None);
    let mut msg = Msg::new_with_envelope(
        &account,
        Envelope {
            bcc: Some(Vec::new()),
            cc: Some(Vec::new()),
            subject: Some(String::new()),
            ..Envelope::default()
        },
        );

    // == Expected Results ==
    let expected_msg = Msg {
        envelope: Envelope {
            from: vec![String::from("Name <some@address.asdf>")],
            to: vec![String::from("")],
            // these fields should exist now
            subject: Some(String::from("")),
            bcc: Some(vec![String::from("")]),
            cc: Some(vec![String::from("")]),
            ..Envelope::default()
        },
        body: Body::from("Account Signature\n"),
        ..Msg::default()
    };

    // == Tests ==
    msg.edit_body().unwrap();
    assert_eq!(msg, expected_msg,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg), dbg!(&expected_msg));
}

#[test]
fn test_parse_from_str() {
    use std::collections::HashMap;

    // == Preparations ==
    let account = Account::new_with_signature(Some("Name"), "some@address.asdf", None);
    let msg_template = Msg::new(&account);

    let normal_content = concat![
        "From: Some <user@msg.sf>\n",
        "Subject: Awesome Subject\n",
        "Bcc: mail1@rofl.lol,name <rofl@lol.asdf>\n",
        "To: To <name@msg.rofl>\n",
        "\n",
        "Account Signature\n",
    ];

    let content_with_custom_headers = concat![
        "From: Some <user@msg.sf>\n",
        "Subject: Awesome Subject\n",
        "Bcc: mail1@rofl.lol,name <rofl@lol.asdf>\n",
        "To: To <name@msg.rofl>\n",
        "CustomHeader1: Value1\n",
        "CustomHeader2: Value2\n",
        "\n",
        "Account Signature\n",
    ];

    // == Expected outputs ==
    let expect = Msg {
        envelope: Envelope {
            from: vec![String::from("Some <user@msg.sf>")],
            subject: Some(String::from("Awesome Subject")),
            bcc: Some(vec![
                      String::from("name <rofl@lol.asdf>"),
                      String::from("mail1@rofl.lol"),
            ]),
            to: vec![String::from("To <name@msg.rofl>")],
            ..Envelope::default()
        },
        body: Body::from("Account Signature\n"),
        ..Msg::default()
    };

    // -- with custom headers --
    let mut custom_headers: HashMap<String, Vec<String>> = HashMap::new();
    custom_headers.insert("CustomHeader1".to_string(), vec!["Value1".to_string()]);
    custom_headers.insert("CustomHeader2".to_string(), vec!["Value2".to_string()]);

    let expect_custom_header = Msg {
        envelope: Envelope {
            from: vec![String::from("Some <user@msg.sf>")],
            subject: Some(String::from("Awesome Subject")),
            bcc: Some(vec![
                      String::from("name <rofl@lol.asdf>"),
                      String::from("mail1@rofl.lol"),
            ]),
            to: vec![String::from("To <name@msg.rofl>")],
            custom_headers: Some(custom_headers),
            ..Envelope::default()
        },
        body: Body::from("Account Signature\n"),
        ..Msg::default()
    };

    // == Testing ==
    let mut msg1 = msg_template.clone();
    let mut msg2 = msg_template.clone();

    msg1.parse_from_str(normal_content).unwrap();
    msg2.parse_from_str(content_with_custom_headers).unwrap();

    assert_eq!(msg1, expect,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg1), dbg!(&expect));

    assert_eq!(msg2, expect_custom_header,
               "Left: {:?}, Right: {:?}",
               dbg!(&msg2), dbg!(&expect_custom_header));
}
