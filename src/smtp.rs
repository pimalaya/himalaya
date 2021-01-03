use lettre::{
    message::{header, Message, SinglePart},
    transport::smtp::{authentication::Credentials, SmtpTransport},
    Transport,
};
use mailparse;

use crate::config;

pub fn send(config: &config::Config, bytes: &[u8]) {
    let email_origin = mailparse::parse_mail(bytes).unwrap();
    let email = email_origin
        .headers
        .iter()
        .fold(Message::builder(), |msg, h| {
            match h.get_key().to_lowercase().as_str() {
                "to" => msg.to(h.get_value().parse().unwrap()),
                "cc" => match h.get_value().parse() {
                    Err(_) => msg,
                    Ok(addr) => msg.cc(addr),
                },
                "bcc" => match h.get_value().parse() {
                    Err(_) => msg,
                    Ok(addr) => msg.bcc(addr),
                },
                "subject" => msg.subject(h.get_value()),
                _ => msg,
            }
        })
        .from(config.email_full().parse().unwrap())
        .singlepart(
            SinglePart::builder()
                .header(header::ContentType(
                    "text/plain; charset=utf-8".parse().unwrap(),
                ))
                .header(header::ContentTransferEncoding::Base64)
                .body(email_origin.get_body_raw().unwrap()),
        )
        .unwrap();

    let creds = Credentials::new(config.smtp.login.clone(), config.smtp.password.clone());
    let mailer = SmtpTransport::relay(&config.smtp.host)
        .unwrap()
        .credentials(creds)
        .build();

    println!("Sending ...");
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {:?}", e),
    }
}
