use std::io::{Read, Write};

use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use io_sasl::{
    coroutine::{SaslArg, SaslCoroutine, SaslCoroutineState, SaslYield},
    login::{SaslLogin, SaslLoginCreds},
    mechanism::Sasl,
    rfc4616::plain::{SaslPlain, SaslPlainCreds},
};
use pimalaya_stream::stream::{Stream, TcpConnectOptions, TlsConnectOptions, UnixConnectOptions};
use url::Url;

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, SieveConfig, parse_server},
    sieve::protocol::{
        SieveCapability, SieveData, SieveResponse, SieveScript, literal_size, parse_capabilities,
        parse_script, parse_string, parse_tokens, quote_string, status,
    },
};

const DEFAULT_PORT: u16 = 4190;
const MAX_LINE: usize = 1024 * 1024;
const MAX_LITERAL: usize = 64 * 1024 * 1024;

/// Blocking ManageSieve client implementing RFC 5804 framing and commands.
pub struct SieveClient {
    stream: Stream,
    read_buffer: Vec<u8>,
    capabilities: Vec<SieveCapability>,
}

impl SieveClient {
    /// Opens a ManageSieve session, reads the greeting, upgrades with
    /// STARTTLS when requested, and authenticates when configured.
    pub fn new(config: SieveConfig) -> Result<Self> {
        let server = parse_sieve_server(&config.server)?;
        let tls = config.tls.into_tls(config.alpn);
        let port = server.port().unwrap_or(DEFAULT_PORT);

        if config.starttls && server.scheme() != "sieve" {
            bail!("ManageSieve STARTTLS requires a `sieve://` server")
        }

        let stream = match server.scheme() {
            "sieve" => Stream::connect_tcp(
                server
                    .host_str()
                    .ok_or_else(|| anyhow!("ManageSieve server has no host: {server}"))?,
                port,
                TcpConnectOptions::default(),
            )?,
            "sieves" => Stream::connect_tls(
                server
                    .host_str()
                    .ok_or_else(|| anyhow!("ManageSieve server has no host: {server}"))?,
                port,
                TlsConnectOptions {
                    tls: tls.clone(),
                    ..Default::default()
                },
            )?,
            "unix" => Stream::connect_unix(
                server
                    .to_file_path()
                    .map_err(|()| anyhow!("Invalid ManageSieve Unix socket URL: {server}"))?,
                UnixConnectOptions::default(),
            )?,
            scheme => bail!("Invalid ManageSieve server scheme `{scheme}`"),
        };

        let mut client = Self {
            stream,
            read_buffer: Vec::new(),
            capabilities: Vec::new(),
        };

        let greeting = client.read_response()?.ensure_ok("greeting")?;
        client.set_capabilities(&greeting.data)?;

        if config.starttls {
            client.command(b"STARTTLS")?.ensure_ok("STARTTLS")?;
            client.stream = client.stream.upgrade_tls(&tls)?;

            // RFC 5804 requires the server to send a fresh capability
            // response after TLS negotiation.
            let capabilities = client.read_response()?.ensure_ok("TLS greeting")?;
            client.set_capabilities(&capabilities.data)?;
        }

        if let Some(sasl_config) = config.sasl {
            if server.scheme() == "unix" {
                // A Unix socket can point at a pre-authenticated proxy,
                // matching the IMAP/SMTP behavior in this repository.
                return Ok(client);
            }

            let host = server
                .host_str()
                .ok_or_else(|| anyhow!("Cannot derive host from ManageSieve server `{server}`"))?;
            let sasl = sasl_config.try_into_sasl(host, port)?;
            if server.scheme() == "sieve"
                && !config.starttls
                && matches!(sasl, Sasl::Login(_) | Sasl::Plain(_))
            {
                bail!("ManageSieve LOGIN and PLAIN authentication require TLS")
            }
            client.authenticate(sasl)?;
        }

        Ok(client)
    }

    pub fn capability(&mut self) -> Result<Vec<SieveCapability>> {
        let response = self.command(b"CAPABILITY")?.ensure_ok("CAPABILITY")?;
        self.set_capabilities(&response.data)?;
        Ok(self.capabilities.clone())
    }

    pub fn list_scripts(&mut self) -> Result<Vec<SieveScript>> {
        let response = self.command(b"LISTSCRIPTS")?.ensure_ok("LISTSCRIPTS")?;
        response.data.iter().map(parse_script).collect()
    }

    pub fn get_script(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut command = b"GETSCRIPT ".to_vec();
        command.extend(quote_string(name.as_bytes())?);
        let response = self.command(&command)?.ensure_ok("GETSCRIPT")?;
        let item = response
            .data
            .first()
            .ok_or_else(|| anyhow!("ManageSieve GETSCRIPT returned no script"))?;
        parse_string(item)
    }

    pub fn put_script(&mut self, name: &str, script: &[u8]) -> Result<()> {
        let mut have_space = b"HAVESPACE ".to_vec();
        have_space.extend(quote_string(name.as_bytes())?);
        have_space.extend_from_slice(format!(" {}", script.len()).as_bytes());
        self.command(&have_space)?.ensure_ok("HAVESPACE")?;

        let mut prefix = b"PUTSCRIPT ".to_vec();
        prefix.extend(quote_string(name.as_bytes())?);
        self.literal_command(&prefix, script)?
            .ensure_ok("PUTSCRIPT")?;
        Ok(())
    }

    pub fn check_script(&mut self, script: &[u8]) -> Result<()> {
        self.literal_command(b"CHECKSCRIPT", script)?
            .ensure_ok("CHECKSCRIPT")?;
        Ok(())
    }

    pub fn set_active(&mut self, name: Option<&str>) -> Result<()> {
        let mut command = b"SETACTIVE ".to_vec();
        command.extend(quote_string(name.unwrap_or_default().as_bytes())?);
        self.command(&command)?.ensure_ok("SETACTIVE")?;
        Ok(())
    }

    pub fn delete_script(&mut self, name: &str) -> Result<()> {
        let mut command = b"DELETESCRIPT ".to_vec();
        command.extend(quote_string(name.as_bytes())?);
        self.command(&command)?.ensure_ok("DELETESCRIPT")?;
        Ok(())
    }

    #[cfg(test)]
    pub fn logout(&mut self) -> Result<()> {
        self.command(b"LOGOUT")?.ensure_ok("LOGOUT")?;
        Ok(())
    }

    /// Sends one raw command line and renders its complete response.
    /// Literal-bearing commands belong to the high-level methods above;
    /// raw is intentionally limited to one line so it cannot desync the
    /// response reader with an unframed batch.
    pub fn raw(&mut self, command: &str) -> Result<String> {
        let command = command.trim_end_matches(['\r', '\n']);
        if command.contains('\r') || command.contains('\n') {
            bail!("ManageSieve raw accepts a single command line")
        }
        if command.trim().is_empty() {
            bail!("ManageSieve raw command is empty")
        }

        Ok(self.command(command.as_bytes())?.to_text())
    }

    fn authenticate(&mut self, sasl: Sasl) -> Result<()> {
        match sasl {
            Sasl::Plain(creds) => self.authenticate_plain(creds),
            Sasl::Login(creds) => self.authenticate_login(creds),
            sasl => bail!(
                "ManageSieve SASL mechanism `{}` is not supported; use LOGIN or PLAIN",
                sasl.mechanism().as_str()
            ),
        }
    }

    fn authenticate_plain(&mut self, creds: SaslPlainCreds) -> Result<()> {
        let mut mechanism = SaslPlain::new(creds);
        let payload = wants_write(mechanism.resume(SaslArg::None))?;
        let encoded = STANDARD.encode(payload);
        let mut command = b"AUTHENTICATE \"PLAIN\" ".to_vec();
        command.extend(quote_string(encoded.as_bytes())?);
        let response = self.command(&command)?.ensure_ok("AUTHENTICATE PLAIN")?;
        complete_sasl(&mut mechanism)?;
        self.set_capabilities(&response.data)?;
        Ok(())
    }

    fn authenticate_login(&mut self, creds: SaslLoginCreds) -> Result<()> {
        let mut mechanism = SaslLogin::new(creds);
        self.write_line(b"AUTHENTICATE \"LOGIN\"")?;
        let _challenge = self.read_auth_challenge()?;

        let username = wants_write(mechanism.resume(SaslArg::None))?;
        self.write_auth_response(&username)?;
        let challenge = self.read_auth_challenge()?;

        let password = wants_write(mechanism.resume(SaslArg::Input(&challenge)))?;
        self.write_auth_response(&password)?;
        let response = self.read_response()?.ensure_ok("AUTHENTICATE LOGIN")?;
        complete_sasl(&mut mechanism)?;
        self.set_capabilities(&response.data)?;
        Ok(())
    }

    fn write_auth_response(&mut self, payload: &[u8]) -> Result<()> {
        let encoded = STANDARD.encode(payload);
        let command = quote_string(encoded.as_bytes())?;
        self.write_line(&command)
    }

    fn read_auth_challenge(&mut self) -> Result<Vec<u8>> {
        let line = self.read_line()?;
        if let Some(status) = status(&line) {
            bail!(
                "ManageSieve authentication ended with {}: {}",
                status.as_str(),
                String::from_utf8_lossy(&line)
            )
        }

        if let Some(size) = literal_size(&line) {
            let (literal, suffix) = self.read_literal(size)?;
            if !suffix.is_empty() {
                bail!("invalid ManageSieve authentication challenge")
            }
            return Ok(literal);
        }

        let tokens = parse_tokens(&line)?;
        if tokens.len() != 1 {
            bail!("invalid ManageSieve authentication challenge")
        }
        Ok(tokens[0].clone())
    }

    fn set_capabilities(&mut self, data: &[SieveData]) -> Result<()> {
        if !data.is_empty() {
            self.capabilities = parse_capabilities(data)?;
        }
        Ok(())
    }

    fn command(&mut self, command: &[u8]) -> Result<SieveResponse> {
        self.write_line(command)?;
        self.read_response()
    }

    fn literal_command(&mut self, prefix: &[u8], value: &[u8]) -> Result<SieveResponse> {
        let mut header = prefix.to_vec();
        header.extend_from_slice(format!(" {{{}+}}", value.len()).as_bytes());
        self.write_line(&header)?;
        self.stream.write_all(value)?;
        self.stream.write_all(b"\r\n")?;
        self.stream.flush()?;
        self.read_response()
    }

    fn write_line(&mut self, line: &[u8]) -> Result<()> {
        if line.contains(&b'\r') || line.contains(&b'\n') {
            bail!("ManageSieve command contains an unexpected newline")
        }
        self.stream.write_all(line)?;
        self.stream.write_all(b"\r\n")?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_response(&mut self) -> Result<SieveResponse> {
        let mut data = Vec::new();

        loop {
            let line = self.read_line()?;
            if let Some(response_status) = status(&line) {
                let mut detail = line.clone();
                if let Some(size) = literal_size(&line) {
                    let (literal, suffix) = self.read_literal(size)?;
                    detail.extend_from_slice(b" ");
                    detail.extend_from_slice(&literal);
                    detail.extend_from_slice(&suffix);
                }
                return Ok(SieveResponse {
                    data,
                    status: response_status,
                    detail,
                });
            }

            if let Some((start, size)) = crate::sieve::protocol::literal_marker(&line) {
                let prefix = line[..start].trim_ascii_end().to_vec();
                let (literal, suffix) = self.read_literal(size)?;
                data.push(SieveData::literal(prefix, literal, suffix));
            } else {
                data.push(SieveData::line(line));
            }
        }
    }

    fn read_literal(&mut self, size: usize) -> Result<(Vec<u8>, Vec<u8>)> {
        ensure_literal_size(size)?;
        let mut bytes = vec![0; size];
        self.read_exact(&mut bytes)?;
        Ok((bytes, self.read_line()?))
    }

    fn read_exact(&mut self, output: &mut [u8]) -> Result<()> {
        let mut offset = 0;
        while offset < output.len() {
            if !self.read_buffer.is_empty() {
                let count = (output.len() - offset).min(self.read_buffer.len());
                output[offset..offset + count].copy_from_slice(&self.read_buffer[..count]);
                self.read_buffer.drain(..count);
                offset += count;
                continue;
            }

            let mut chunk = [0; 8192];
            let count = self.stream.read(&mut chunk)?;
            if count == 0 {
                bail!("ManageSieve connection closed while reading a literal")
            }
            self.read_buffer.extend_from_slice(&chunk[..count]);
        }
        Ok(())
    }

    fn read_line(&mut self) -> Result<Vec<u8>> {
        loop {
            if let Some(index) = self.read_buffer.iter().position(|&byte| byte == b'\n') {
                if index > MAX_LINE {
                    bail!("ManageSieve response line exceeds {MAX_LINE} bytes")
                }
                let mut line = self.read_buffer.drain(..=index).collect::<Vec<_>>();
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(line);
            }

            if self.read_buffer.len() > MAX_LINE {
                bail!("ManageSieve response line exceeds {MAX_LINE} bytes")
            }

            let mut chunk = [0; 8192];
            let count = self.stream.read(&mut chunk)?;
            if count == 0 {
                bail!("ManageSieve connection closed before a CRLF response")
            }
            self.read_buffer.extend_from_slice(&chunk[..count]);
        }
    }
}

fn ensure_literal_size(size: usize) -> Result<()> {
    if size > MAX_LITERAL {
        bail!("ManageSieve literal exceeds {MAX_LITERAL} bytes")
    }
    Ok(())
}

/// Parses a ManageSieve server string into a URL.
pub fn parse_sieve_server(server: &str) -> Result<Url> {
    parse_server(server, "sieves", &["sieve", "sieves", "unix"])
}

/// Opens the Sieve session for an already-resolved account.
pub fn build_sieve_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, SieveClient)> {
    let sieve_config = account_config
        .sieve
        .take()
        .ok_or_else(|| anyhow!("Sieve config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    let client = SieveClient::new(sieve_config)?;
    Ok((account, client))
}

fn wants_write<E>(state: SaslCoroutineState<SaslYield, Result<(), E>>) -> Result<Vec<u8>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    match state {
        SaslCoroutineState::Yielded(SaslYield::WantsWrite(payload)) => Ok(payload),
        SaslCoroutineState::Yielded(SaslYield::WantsRead) => {
            bail!("ManageSieve SASL mechanism unexpectedly requested a read")
        }
        SaslCoroutineState::Complete(result) => result.map(|()| Vec::new()).map_err(Into::into),
    }
}

fn complete_sasl<M>(mechanism: &mut M) -> Result<()>
where
    M: SaslCoroutine,
    M::Error: std::error::Error + Send + Sync + 'static,
{
    match mechanism.resume(SaslArg::Done) {
        SaslCoroutineState::Complete(result) => result.map_err(Into::into),
        SaslCoroutineState::Yielded(_) => bail!("ManageSieve SASL exchange did not complete"),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    use super::*;

    #[test]
    fn exercises_literal_commands_against_a_local_server() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || fake_server(listener));

        let config = SieveConfig {
            server: format!("sieve://{address}"),
            tls: Default::default(),
            starttls: false,
            alpn: Vec::new(),
            sasl: None,
        };
        let mut client = SieveClient::new(config).unwrap();
        assert_eq!(client.list_scripts().unwrap()[1].name, "vacation script");
        assert_eq!(
            client.get_script("main").unwrap(),
            b"require [\"fileinto\"];\n"
        );
        client
            .put_script("new", b"require [\"fileinto\"];\n")
            .unwrap();
        client.check_script(b"require [\"fileinto\"];\n").unwrap();
        client.set_active(Some("new")).unwrap();
        client.delete_script("new").unwrap();
        client.logout().unwrap();

        server.join().unwrap();
    }

    #[test]
    fn rejects_oversized_literals_before_allocating() {
        assert!(ensure_literal_size(MAX_LITERAL + 1).is_err());
        ensure_literal_size(MAX_LITERAL).unwrap();
    }

    fn fake_server(listener: TcpListener) {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        write_capabilities(&mut writer);

        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                return;
            }
            let command = line.trim_end_matches(['\r', '\n']);
            if command == "LISTSCRIPTS" {
                writer
                    .write_all(b"\"main\"\r\n{15}\r\nvacation script\r\n ACTIVE\r\nOK\r\n")
                    .unwrap();
            } else if command == "GETSCRIPT \"main\"" {
                let script = b"require [\"fileinto\"];\n";
                write!(writer, "{{{}}}\r\n", script.len()).unwrap();
                writer.write_all(script).unwrap();
                writer.write_all(b"\r\nOK\r\n").unwrap();
            } else if command.starts_with("HAVESPACE ") {
                writer.write_all(b"OK\r\n").unwrap();
            } else if command.starts_with("PUTSCRIPT ") || command.starts_with("CHECKSCRIPT ") {
                let size = line_literal_size(command.as_bytes());
                let mut script = vec![0; size];
                reader.read_exact(&mut script).unwrap();
                let mut terminator = String::new();
                reader.read_line(&mut terminator).unwrap();
                writer.write_all(b"OK\r\n").unwrap();
            } else if command.starts_with("SETACTIVE ") || command.starts_with("DELETESCRIPT ") {
                writer.write_all(b"OK\r\n").unwrap();
            } else if command == "LOGOUT" {
                writer.write_all(b"OK\r\n").unwrap();
                return;
            } else {
                panic!("unexpected fake ManageSieve command: {command}");
            }
            writer.flush().unwrap();
        }
    }

    fn write_capabilities(writer: &mut TcpStream) {
        writer
            .write_all(b"\"IMPLEMENTATION\" \"fake\"\r\n\"SIEVE\" \"fileinto vacation\"\r\nOK\r\n")
            .unwrap();
        writer.flush().unwrap();
    }

    fn line_literal_size(line: &[u8]) -> usize {
        let start = line.iter().rposition(|&byte| byte == b'{').unwrap();
        let end = line.iter().rposition(|&byte| byte == b'}').unwrap();
        std::str::from_utf8(&line[start + 1..end])
            .unwrap()
            .trim_end_matches('+')
            .parse()
            .unwrap()
    }
}
