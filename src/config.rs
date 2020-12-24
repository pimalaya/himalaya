use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use toml;

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    host: String,
    port: usize,
    login: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    name: String,
    email: String,
    imap: ServerInfo,
    smtp: ServerInfo,
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

pub fn read_file() -> Config {
    match read_file_content() {
        Err(err) => panic!(err),
        Ok(content) => toml::from_str(&content).unwrap(),
    }
}
