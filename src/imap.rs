use imap;
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;

use crate::config::{Config, ServerInfo};

type ImapClient = imap::Client<TlsStream<TcpStream>>;
type ImapSession = imap::Session<TlsStream<TcpStream>>;

pub fn create_tls_connector() -> TlsConnector {
    match native_tls::TlsConnector::new() {
        Ok(connector) => connector,
        Err(err) => {
            println!("The TLS connector could not be created.");
            panic!(err);
        }
    }
}

pub fn create_imap_client(server: &ServerInfo, tls: &TlsConnector) -> ImapClient {
    match imap::connect(server.get_addr(), server.get_host(), &tls) {
        Ok(client) => client,
        Err(err) => {
            println!("The IMAP socket could not be opened.");
            panic!(err);
        }
    }
}

pub fn create_imap_sess(client: ImapClient, server: &ServerInfo) -> ImapSession {
    match client.login(server.get_login(), server.get_password()) {
        Ok(sess) => sess,
        Err((err, _)) => {
            println!("The IMAP connection could not be established.");
            panic!(err);
        }
    }
}

pub fn login(config: &Config) -> ImapSession {
    let tls = create_tls_connector();
    let client = create_imap_client(&config.imap, &tls);
    let sess = create_imap_sess(client, &config.imap);
    sess
}
