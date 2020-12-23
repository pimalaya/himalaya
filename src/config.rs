use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use toml::Value;

#[derive(Debug)]
pub struct Config {
    name: String,
    email: String,
}

impl Config {
    fn new() -> Config {
        Config {
            name: String::new(),
            email: String::new(),
        }
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

pub fn read_file() -> Config {
    let path = file_path();
    match File::open(path) {
        Err(_) => panic!("Config file not found!"),
        Ok(mut file) => {
            let mut content = String::new();
            match file.read_to_string(&mut content) {
                Err(err) => panic!(err),
                Ok(_) => {
                    let toml = content.parse::<Value>().unwrap();
                    println!("{:?}", toml);
                    Config::new()
                }
            }
        }
    }
}
