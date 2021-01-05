use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use toml;

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub host: String,
    pub port: u16,
    pub login: String,
    pub password: String,
}

impl ServerInfo {
    pub fn get_addr(&self) -> (&str, u16) {
        (&self.host, self.port)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub email: String,
    pub imap: ServerInfo,
    pub smtp: ServerInfo,
}

impl Config {
    pub fn new_from_file() -> Self {
        match read_file_content() {
            Err(err) => panic!(err),
            Ok(content) => toml::from_str(&content).unwrap(),
        }
    }

    pub fn email_full(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

pub fn from_xdg() -> Option<PathBuf> {
    match env::var("XDG_CONFIG_HOME") {
        Err(_) => None,
        Ok(path_str) => {
            let mut path = PathBuf::from(path_str);
            path.push("himalaya");
            path.push("config.toml");
            Some(path)
        }
    }
}

pub fn from_home() -> Option<PathBuf> {
    match env::var("HOME") {
        Err(_) => None,
        Ok(path_str) => {
            let mut path = PathBuf::from(path_str);
            path.push(".config");
            path.push("himalaya");
            path.push("config.toml");
            Some(path)
        }
    }
}

pub fn from_tmp() -> Option<PathBuf> {
    let mut path = env::temp_dir();
    path.push("himalaya");
    path.push("config.toml");
    Some(path)
}

pub fn file_path() -> PathBuf {
    match from_xdg().or_else(from_home).or_else(from_tmp) {
        None => panic!("Config file path not found."),
        Some(path) => path,
    }
}

pub fn read_file_content() -> Result<String, io::Error> {
    let path = file_path();
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
